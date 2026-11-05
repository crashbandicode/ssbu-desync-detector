"""Summarize retained detector evidence; does not infer a hardware cause.

Usage: python3 -I tools/summarize_disconnect.py EVENTS [--previous PREVIOUS]
       [--tm TM_LOG] [--frames FRAMES_BIN]

`--previous` is an older `events.previous.log`, already chronological
before `EVENTS`. A telemetry `boot_tick` starts a boot when the install
line has rotated out. Samples from another boot are left out.
`unix_s` is used only when `clock_initialized=1` and `clock_result=0x0`.
That is the console clock, not a measured router offset. A `frame_gap`
is not a session end.
"""
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import struct

TICKS_PER_SECOND = 19_200_000
EVENT = re.compile(r"^(\d+) (\w+) (.*)$")
ANCHOR = re.compile(r"^(\d+) .*\[PreloadMetrics\].*\btick=(\d+)\b")
INTEGER = re.compile(r"-?\d+")
COUNTERS = ("event_drops", "frame_drops", "evidence_losses", "heap_allocations")


def fields(text):
    return dict(part.split("=", 1) for part in text.split() if "=" in part)


def utc_window(tick, utc_ms, anchor_tick):
    lower_ms = utc_ms + (tick - anchor_tick) * 1000 // TICKS_PER_SECOND

    def iso(value):
        return datetime.fromtimestamp(value / 1000, timezone.utc).isoformat(
            timespec="milliseconds"
        )

    return [iso(lower_ms), iso(lower_ms + 1000)]


def iso_ticks(ticks, round_up=False):
    ticks_per_ms = TICKS_PER_SECOND // 1000
    millis = -(-ticks // ticks_per_ms) if round_up else ticks // ticks_per_ms
    return datetime.fromtimestamp(millis / 1000, timezone.utc).isoformat(
        timespec="milliseconds"
    )


def anchor_window(anchor, tick):
    delta = tick - anchor["tick"]
    lower = anchor["unix_s"] * TICKS_PER_SECOND + delta - anchor["clock_query_ticks"]
    upper = (anchor["unix_s"] + 1) * TICKS_PER_SECOND + delta
    return lower, upper


def integer(text):
    if text is None or not INTEGER.fullmatch(text):
        return None
    return int(text)


def record(source, number, line):
    item = {"file": source, "line": number}
    if "plugin=ssbu-desync-detector/" in line:
        item["install"] = True
        head = line.split(" ", 1)[0]
        if head.isdigit():
            item["tick"] = int(head)
        return item
    match = EVENT.fullmatch(line)
    if match:
        tick, kind, text = match.groups()
        item.update(tick=int(tick), kind=kind, fields=fields(text))
    return item


def rows_from(previous_text, events_text):
    rows = []
    for source, text in (("previous", previous_text or ""), ("events", events_text)):
        if source == "previous" and text == "":
            continue
        for number, line in enumerate(text.splitlines(), 1):
            if line:
                rows.append(record(source, number, line))
    return rows


def boot_slice(rows):
    """Last install starts a boot. A changed telemetry boot_tick does too."""
    boot_id = None
    start = None
    for index, item in enumerate(rows):
        if item.get("install") and "tick" in item:
            boot_id = item["tick"]
            start = index
            continue
        if item.get("kind") != "telemetry":
            continue
        raw = integer(item["fields"].get("boot_tick"))
        if raw is None or raw == boot_id:
            continue
        boot_id = raw
        start = index
    if start is None:
        raise ValueError("No detector install marker: boot scope is unknown")
    return rows[start:], boot_id


def publish(item, show_file):
    event = {"line": item["line"], "tick": item["tick"], "kind": item["kind"],
             "fields": item["fields"]}
    if show_file:
        event["file"] = item["file"]
    return event


def usable_anchor(item):
    raw = item["fields"]
    if raw.get("clock_initialized") != "1":
        return None
    result = raw.get("clock_result", "")
    if not result.startswith("0x"):
        return None
    try:
        code = int(result, 16)
    except ValueError:
        return None
    if code != 0:
        return None
    unix_s = integer(raw.get("unix_s"))
    span = integer(raw.get("clock_query_ticks"))
    if unix_s is None or span is None or span < 0:
        return None
    return {"tick": item["tick"], "unix_s": unix_s, "clock_query_ticks": span,
            "line": item["line"], "file": item["file"]}


def telemetry_clock(anchors, session, links):
    ordered = anchors
    jumps = []
    for left, right in zip(ordered, ordered[1:]):
        delta = right["tick"] - left["tick"]
        left_lower, left_upper = anchor_window(left, right["tick"])
        right_lower, right_upper = anchor_window(right, right["tick"])
        regressed = right["unix_s"] < left["unix_s"]
        if delta < 0 or left_upper < right_lower or right_upper < left_lower:
            jumps.append({"from_tick": left["tick"], "to_tick": right["tick"],
                          "from_unix_s": left["unix_s"], "to_unix_s": right["unix_s"],
                          "regression": regressed})
    clock = {
        "source": "telemetry",
        "reliable": not jumps,
        "anchors": ordered,
        "note": "Console-clock windows include second truncation and the SDK query span. "
                "Reliable means no observed clock discontinuity, not measured "
                "synchronization to a router clock. Upper endpoints are exclusive.",
    }
    if jumps:
        clock["jumps"] = jumps
        clock["session_start_utc"] = None
        return clock

    def instant(tick):
        anchor = min(ordered, key=lambda item: abs(item["tick"] - tick))
        lower, upper = anchor_window(anchor, tick)
        return [iso_ticks(lower), iso_ticks(upper, round_up=True)]

    if session is not None:
        clock["session_start_utc"] = instant(session["tick"])
        if links:
            clock["first_link_utc"] = instant(links[0]["tick"])
            clock["last_link_utc"] = instant(links[-1]["tick"])
    return clock


def tm_clock(tm_text, session, links):
    anchors = []
    for index, line in enumerate(tm_text.splitlines(), 1):
        match = ANCHOR.match(line)
        if match:
            utc_ms, anchor_tick = map(int, match.groups())
            anchors.append((utc_ms, anchor_tick, index))
    if not anchors or session is None:
        return None
    utc_ms, anchor_tick, line = min(anchors, key=lambda anchor: abs(anchor[1] - session["tick"]))
    return {
        "source": "tm",
        "telemetry_clock": "unavailable",
        "tm_line": line, "utc_ms": utc_ms, "tick": anchor_tick,
        "assumption": "Same boot, stable wall clock, second-truncated TM time; "
                      "log-call skew is not bounded. Windows are estimates.",
        "session_start_utc": utc_window(session["tick"], utc_ms, anchor_tick),
        "first_link_utc": utc_window(links[0]["tick"], utc_ms, anchor_tick) if links else None,
        "last_link_utc": utc_window(links[-1]["tick"], utc_ms, anchor_tick) if links else None,
    }


def capture_of(samples):
    if not samples:
        return None
    last = samples[-1]["fields"]
    values = {name: integer(last.get(name)) for name in COUNTERS}
    missing = [name for name, value in values.items() if value is None]
    return {"samples": len(samples), "line": samples[-1]["line"],
            "file": samples[-1].get("file", "events"), **values,
            "missing": missing}


def summarize(events_text, tm_text="", frames=None, previous_text=""):
    show_file = previous_text != ""
    scoped, boot_id = boot_slice(rows_from(previous_text, events_text))
    opened_by_install = scoped[0].get("install", False)
    body = scoped[1:] if opened_by_install else scoped
    events = [publish(item, show_file) for item in body if "kind" in item]
    sessions = [event for event in events if event["kind"] == "session"]
    telemetry = [event for event in events if event["kind"] == "telemetry"]
    gaps = [event for event in events if event["kind"] == "frame_gap"]
    reasons = [event for event in events if event["kind"] == "communication_text"]
    result = {
        "boot_install_line": scoped[0]["line"] if opened_by_install else None,
        "boot_tick": boot_id,
        "sessions": len(sessions),
        "recorded_mismatches": sum(event["kind"] == "desync" for event in events),
        "telemetry": telemetry,
        "frame_gaps": gaps,
        "frame_gap_note": "frame_gap does not end a session and is not a cause.",
        "communication_text": reasons,
        "capture": capture_of(telemetry),
        "latest_session": None,
    }
    if not opened_by_install:
        result["missing_install"] = True
        result["coverage"] = "from the first telemetry sample of this boot_tick"
    elif show_file:
        result["boot_install_file"] = scoped[0]["file"]
    if not telemetry:
        result["capture_note"] = "no telemetry in this boot"
    session = None
    links = []
    if sessions:
        session = sessions[-1]
        episode = events[events.index(session) + 1:]
        links = [event for event in episode if event["kind"] == "link"]
        result["latest_session"] = {
            "start": session,
            "sync_ends": [event for event in episode if event["kind"] == "sync_end"],
            "peer_ends": [event for event in episode if event["kind"] == "peer_end"],
            "links": links,
            "frame_gaps": [event for event in episode if event["kind"] == "frame_gap"],
            "communication_text": [event for event in episode
                                   if event["kind"] == "communication_text"],
        }
    anchors = [anchor for event in telemetry if (anchor := usable_anchor(
        {"tick": event["tick"], "line": event["line"], "fields": event["fields"],
         "file": event.get("file", "events")})) is not None]
    if anchors:
        result["clock_correlation"] = telemetry_clock(anchors, session, links)
        if tm_text:
            result["clock_correlation"]["tm_present_not_used"] = True
    else:
        fallback = tm_clock(tm_text, session, links)
        if fallback is not None:
            result["clock_correlation"] = fallback
        elif telemetry:
            result["clock_correlation"] = {
                "source": "missing",
                "note": "No telemetry sample has clock_initialized=1 and clock_result=0x0.",
            }
    if frames is not None:
        if len(frames) % 20:
            raise ValueError("Truncated 20-byte frame record")
        rows = list(struct.iter_unpack("<5I", frames))
        result["frames"] = {
            "records": len(rows),
            "first": rows[0][0] if rows else None,
            "last": rows[-1][0] if rows else None,
            "contiguous": all(b[0] == a[0] + 1 for a, b in zip(rows, rows[1:])),
        }
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("events", type=Path)
    parser.add_argument("--previous", type=Path)
    parser.add_argument("--tm", type=Path)
    parser.add_argument("--frames", type=Path)
    args = parser.parse_args()
    result = summarize(
        args.events.read_text(),
        args.tm.read_text() if args.tm else "",
        args.frames.read_bytes() if args.frames else None,
        args.previous.read_text() if args.previous else "",
    )
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
