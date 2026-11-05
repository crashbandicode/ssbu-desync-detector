"""Host checks for evidence scope, clock arithmetic and damaged captures."""
import importlib.util
from pathlib import Path
import struct
import unittest

spec = importlib.util.spec_from_file_location(
    "summarize_disconnect", Path(__file__).with_name("summarize_disconnect.py")
)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class EvidenceSummaryTests(unittest.TestCase):
    def test_previous_boot_mismatch_is_excluded(self):
        text = ("1 plugin=ssbu-desync-detector/0.2.0\n2 desync frame=2\n"
                "3 plugin=ssbu-desync-detector/0.2.0\n"
                "4 session slot=0 kind=online\n5 link result=0x0 state=4\n")
        result = module.summarize(text)
        self.assertEqual(result["boot_install_line"], 3)
        self.assertEqual(result["recorded_mismatches"], 0)
        self.assertEqual(result["sessions"], 1)

    def test_latest_session_has_only_its_ends_and_links(self):
        text = ("1 plugin=ssbu-desync-detector/0.2.0\n"
                "2 session kind=online\n3 sync_end kind=1\n"
                "4 link state=4\n5 session kind=online\n6 link state=2\n")
        latest = module.summarize(text)["latest_session"]
        self.assertEqual(latest["sync_ends"], [])
        self.assertEqual(latest["peer_ends"], [])
        self.assertEqual([row["tick"] for row in latest["links"]], [6])

    def test_boot_without_session_and_missing_marker(self):
        self.assertIsNone(module.summarize(
            "1 plugin=ssbu-desync-detector/0.2.0\n")["latest_session"])
        with self.assertRaises(ValueError):
            module.summarize("2 session kind=online\n")

    def test_clock_correlation_allows_negative_tick_difference(self):
        text = ("1 plugin=ssbu-desync-detector/0.2.0\n"
                "19200000 session kind=online\n38400000 link state=0\n")
        tm = "2000 #1 [PreloadMetrics] snapshot tick=38400000\n"
        clock = module.summarize(text, tm)["clock_correlation"]
        self.assertEqual(clock["session_start_utc"],
                         ["1970-01-01T00:00:01.000+00:00",
                          "1970-01-01T00:00:02.000+00:00"])
        self.assertEqual(clock["first_link_utc"][0],
                         "1970-01-01T00:00:02.000+00:00")

    def test_frame_gaps_and_truncation_are_visible(self):
        text = "1 plugin=ssbu-desync-detector/0.2.0\n"
        frames = b"".join(struct.pack("<5I", frame, 0, 0, 0, 1)
                          for frame in [0, 1, 3])
        result = module.summarize(text, frames=frames)["frames"]
        self.assertEqual(result, {"records": 3, "first": 0, "last": 3,
                                  "contiguous": False})
        with self.assertRaises(ValueError):
            module.summarize(text, frames=frames[:-1])

    def sample(self, tick, **overrides):
        fields = {
            "boot_tick": "100", "unix_s": "0", "clock_initialized": "0",
            "clock_result": "0x0", "clock_query_ticks": "0", "poll": "1",
            "poll_gap_ms": "5000", "result": "0x0", "interface": "1",
            "strength": "0", "state": "0", "active": "0", "kind": "idle",
            "slot": "0", "frame": "0", "frame_age_ms": "0", "event_drops": "0",
            "frame_drops": "0", "evidence_losses": "0", "heap_allocations": "0",
        }
        fields.update(overrides)
        body = " ".join(f"{key}={value}" for key, value in fields.items())
        return f"{tick} telemetry {body}\n"

    def test_telemetry_clock_preserves_second_and_query_span_uncertainty(self):
        text = ("100 plugin=ssbu-desync-detector/0.2.0\n"
                "1000 session kind=online\n"
                + self.sample(1000, clock_initialized="1", unix_s="100", kind="online",
                              clock_query_ticks=str(module.TICKS_PER_SECOND // 4)))
        clock = module.summarize(text)["clock_correlation"]
        self.assertEqual(clock["source"], "telemetry")
        self.assertTrue(clock["reliable"])
        self.assertEqual(clock["session_start_utc"],
                         ["1970-01-01T00:01:39.750+00:00",
                          "1970-01-01T00:01:41.000+00:00"])
        self.assertIn("router", clock["note"])

    def test_slow_clock_query_is_not_misreported_as_a_clock_jump(self):
        text = ("100 plugin=ssbu-desync-detector/0.2.0\n"
                + self.sample(1000, clock_initialized="1", unix_s="100",
                              clock_query_ticks=str(5 * module.TICKS_PER_SECOND))
                + self.sample(1000 + 6 * module.TICKS_PER_SECOND, clock_initialized="1",
                              unix_s="102", poll="2"))
        clock = module.summarize(text)["clock_correlation"]
        self.assertTrue(clock["reliable"])
        self.assertNotIn("jumps", clock)

    def test_bad_console_clock_keeps_labelled_tm_fallback(self):
        text = ("1 plugin=ssbu-desync-detector/0.2.0\n"
                "19200000 session kind=online\n38400000 link state=0\n"
                + self.sample(19200000, boot_tick="1", clock_initialized="1",
                              clock_result="0x1", unix_s="50"))
        tm = "2000 #1 [PreloadMetrics] snapshot tick=38400000\n"
        clock = module.summarize(text, tm)["clock_correlation"]
        self.assertEqual(clock["source"], "tm")
        self.assertEqual(clock["telemetry_clock"], "unavailable")
        self.assertEqual(clock["session_start_utc"][0], "1970-01-01T00:00:01.000+00:00")

    def test_clock_regression_is_not_fitted(self):
        text = ("100 plugin=ssbu-desync-detector/0.2.0\n"
                "1000 session kind=online\n"
                + self.sample(1000, clock_initialized="1", unix_s="100")
                + self.sample(1000 + module.TICKS_PER_SECOND, clock_initialized="1",
                              unix_s="50", poll="2"))
        clock = module.summarize(text)["clock_correlation"]
        self.assertFalse(clock["reliable"])
        self.assertIsNone(clock["session_start_utc"])
        self.assertTrue(clock["jumps"][0]["regression"])
        self.assertEqual([anchor["unix_s"] for anchor in clock["anchors"]], [100, 50])

    def test_loss_counters_use_the_latest_total(self):
        text = ("100 plugin=ssbu-desync-detector/0.2.0\n"
                + self.sample(200, event_drops="1", frame_drops="2",
                              evidence_losses="3", heap_allocations="0")
                + self.sample(300, event_drops="5", frame_drops="2",
                              evidence_losses="3", heap_allocations="0", poll="2"))
        capture = module.summarize(text)["capture"]
        self.assertEqual(capture["samples"], 2)
        self.assertEqual(capture["event_drops"], 5)
        self.assertEqual(capture["missing"], [])

    def test_rotated_boot_is_not_mixed_and_gap_is_not_an_end(self):
        previous = "1 plugin=ssbu-desync-detector/0.2.0\n2 desync frame=2\n"
        events = (self.sample(10, boot_tick="10")
                  + "11 session slot=0 kind=online\n"
                  + "12 frame_gap kind=online slot=0 frame=4 last_frame_tick=11\n"
                  + "13 communication_text reason=3 text_units=12\n")
        result = module.summarize(events, previous_text=previous)
        self.assertTrue(result["missing_install"])
        self.assertEqual(result["boot_tick"], 10)
        self.assertEqual(result["recorded_mismatches"], 0)
        self.assertEqual(result["sessions"], 1)
        latest = result["latest_session"]
        self.assertEqual(latest["sync_ends"], [])
        self.assertEqual(len(latest["frame_gaps"]), 1)
        self.assertEqual(latest["communication_text"][0]["fields"]["reason"], "3")
        self.assertEqual(result["capture"]["event_drops"], 0)

    def test_same_boot_continues_from_previous_install(self):
        previous = "100 plugin=ssbu-desync-detector/0.2.0\n150 session kind=online\n"
        events = "200 link state=4\n" + self.sample(210, boot_tick="100", event_drops="1")
        result = module.summarize(events, previous_text=previous)
        self.assertFalse(result.get("missing_install", False))
        self.assertEqual(result["boot_install_line"], 1)
        self.assertEqual(result["boot_install_file"], "previous")
        self.assertEqual(result["sessions"], 1)
        self.assertEqual(result["latest_session"]["links"][0]["tick"], 200)
        self.assertEqual(result["capture"]["event_drops"], 1)


if __name__ == "__main__":
    unittest.main()
