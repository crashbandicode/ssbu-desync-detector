"""Which checked RNG stream explains a desync value, from a vanilla playback's per-frame stream states.

The online check is the XOR of the `w` words of the game's nine xorshift128 streams
(docs/WHAT-IS-CHECKED.md). An Eden trace point on the playback check (eden/popup-route/steps.py
`arm_rng_trace`) records, every playback frame, the replay frame index and the 9 x 0x14-byte stream
array (x, y, z, w, then a 32-bit word). This tool:

1. checks each traced frame's XOR of `w` words against the detector's own playback value
   (frames-N.bin of the same playback), so the capture is the checked state;
2. reports, per frame around the desync, how many times each stream advanced (xorshift steps);
3. lists every single-stream change of up to --max-steps draws, forward or back, that turns this
   console's value at the desync frame into the other console's value.

Usage:
  stream_attribution.py <trace.jsonl> --frame 4348 --ours 0x7774d5d0 --theirs 0x9985577d
      [--playback-frames frames-N.bin] [--console-frames frames-0.bin] [--window 8] [--max-steps 8]
"""
import argparse
import json
import struct
import sys

STREAMS = 9
STRIDE = 0x14
NAMES = ['other', 'fighter', 'weapon', 'stage', 'ai', 'enemy', 'item', 'sound', 'camera']
MASK = 0xffffffff


def step(state):
    """One xorshift128 draw: (x, y, z, w) -> (y, z, w, w')."""
    x, y, z, w = state
    t = (x ^ (x << 11)) & MASK
    return (y, z, w, (w ^ (w >> 19) ^ t ^ (t >> 8)) & MASK)


def _unshift_right(value, shift):
    result = value
    for _ in range(32 // shift + 1):
        result = value ^ (result >> shift)
    return result & MASK


def _unshift_left(value, shift):
    result = value
    for _ in range(32 // shift + 1):
        result = (value ^ (result << shift)) & MASK
    return result


def unstep(state):
    """The inverse draw: (y, z, w, w') -> (x, y, z, w)."""
    y, z, w, new_w = state
    t = _unshift_right(new_w ^ w ^ (w >> 19), 8)
    x = _unshift_left(t, 11)
    return (x, y, z, w)


def advance(state, n):
    for _ in range(abs(n)):
        state = step(state) if n > 0 else unstep(state)
    return state


def steps_between(before, after, limit=64):
    """How many draws turn `before` into `after` (0..limit), or None."""
    state = before
    for n in range(limit + 1):
        if state == after:
            return n
        state = step(state)
    return None


def streams(raw):
    """Nine (x, y, z, w) tuples and the trailing words from the captured stream array."""
    states, extra = [], []
    for k in range(STREAMS):
        x, y, z, w, tail = struct.unpack_from('<5I', raw, k * STRIDE)
        states.append((x, y, z, w))
        extra.append(tail)
    return states, extra


def check_value(states):
    value = 0
    for state in states:
        value ^= state[3]
    return value


def _capture_bytes(capture):
    for key in ('bytes', 'hex', 'data', 'value'):
        value = capture.get(key)
        if isinstance(value, str):
            text = value[2:] if value.startswith('0x') else value
            try:
                return bytes.fromhex(text.replace(' ', ''))
            except ValueError:
                continue
        if isinstance(value, list) and all(isinstance(v, int) for v in value):
            return bytes(value)
    return None


def _capture_int(capture):
    for key in ('value', 'u32', 'decoded'):
        value = capture.get(key)
        if isinstance(value, int):
            return value
        if isinstance(value, str):
            try:
                return int(value, 0)
            except ValueError:
                continue
    return None


def load_trace(path):
    """{frame: (states, extra)} from the playback-check trace records (entry events with two captures)."""
    frames = {}
    with open(path, encoding='utf-8') as handle:
        for line in handle:
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            captures = record.get('captures') or []
            if record.get('event', 'entry') != 'entry' or len(captures) < 2:
                continue
            frame = _capture_int(captures[0])
            raw = _capture_bytes(captures[1])
            if frame is None or raw is None or len(raw) < STREAMS * STRIDE:
                continue
            # The detector numbers a playback frame as the player's index minus one.
            frames.setdefault((frame - 1) & MASK, streams(raw))
    return frames


def load_frames(path, kind):
    """{frame: check value} from a detector frames-N.bin (first record of each frame)."""
    data = open(path, 'rb').read()
    values = {}
    for offset in range(0, len(data) - len(data) % 20, 20):
        frame, rng, _, _, flags = struct.unpack_from('<5I', data, offset)
        if flags & 0xff == kind:
            values.setdefault(frame, rng)
    return values


def hypotheses(states, ours, theirs, max_steps):
    """Single-stream changes of 1..max_steps draws (either way) that turn `ours` into `theirs`."""
    found = []
    for k, state in enumerate(states):
        for n in [d for m in range(1, max_steps + 1) for d in (m, -m)]:
            if ours ^ state[3] ^ advance(state, n)[3] == theirs:
                found.append({'stream': k, 'name': NAMES[k], 'draws': n})
    return found


def explain(states, target, max_steps=8, pair_steps=4):
    """Stream advances that turn `states` into a check value of `target`: the single-stream solutions
    (1..max_steps draws) if any, else the two-stream ones (1..pair_steps each)."""
    value = check_value(states)
    if value == target:
        return [()]
    single = []
    for k, state in enumerate(states):
        w = state[3]
        advanced = state
        for n in range(1, max_steps + 1):
            advanced = step(advanced)
            if value ^ w ^ advanced[3] == target:
                single.append(((k, n),))
    if single:
        return single
    pairs = []
    ws = [[s[3]] for s in states]
    for k, state in enumerate(states):
        advanced = state
        for _ in range(pair_steps):
            advanced = step(advanced)
            ws[k].append(advanced[3])
    for a in range(STREAMS):
        for b in range(a + 1, STREAMS):
            for na in range(1, pair_steps + 1):
                for nb in range(1, pair_steps + 1):
                    if value ^ ws[a][0] ^ ws[a][na] ^ ws[b][0] ^ ws[b][nb] == target:
                        pairs.append(((a, na), (b, nb)))
    return pairs


def reconstruct(states, console, start, end, max_steps=8):
    """Follow this console from `start` (its states equal the vanilla ones there) to `end`, one frame at a
    time, applying the unique stream advance that gives its logged check value. Stops at the first frame
    with no or several explanations. Returns (states at the last explained frame, that frame, lines)."""
    lines = []
    frame = start
    for f in range(start + 1, end + 1):
        if f not in console:
            lines.append(f'frame {f}: not in the console frames; stop')
            break
        options = explain(states, console[f], max_steps)
        if len(options) != 1:
            lines.append(f'frame {f}: {"no" if not options else len(options)} explanation(s) for '
                         f'0x{console[f]:08x}; stop')
            break
        for k, n in options[0]:
            states = list(states)
            states[k] = advance(states[k], n)
        if options[0]:
            lines.append(f'frame {f}: ' + ', '.join(f'{NAMES[k]} +{n}' for k, n in options[0]))
        frame = f
    return states, frame, lines


def report(trace, frame, ours, theirs, playback=None, console=None, window=8, max_steps=8):
    lines = [f'traced frames: {len(trace)} ({min(trace)}..{max(trace)})' if trace else 'traced frames: 0']
    if playback:
        common = [f for f in sorted(trace) if f in playback]
        bad = [f for f in common if check_value(trace[f][0]) != playback[f]]
        lines.append(f'trace XOR vs detector playback value: {len(common) - len(bad)}/{len(common)} frames equal'
                     + (f', first differing {bad[0]}' if bad else ''))
    if console:
        common = [f for f in sorted(trace) if f in console]
        bad = [f for f in common if check_value(trace[f][0]) != console[f]]
        lines.append(f'vanilla vs this console: {len(common) - len(bad)}/{len(common)} frames equal'
                     + (f', first differing {bad[0]}' if bad else ''))
    if frame in trace:
        value = check_value(trace[frame][0])
        lines.append(f'vanilla at frame {frame}: 0x{value:08x} (ours 0x{ours:08x}, theirs 0x{theirs:08x})')
    for f in range(frame - window, frame + 2):
        if f in trace and f - 1 in trace:
            counts = [steps_between(a, b) for a, b in zip(trace[f - 1][0], trace[f][0])]
            drawn = {NAMES[k]: n for k, n in enumerate(counts) if n != 0}
            tails = [b - a & MASK for a, b in zip(trace[f - 1][1], trace[f][1])]
            lines.append(f'frame {f}: draws {drawn or "none"}; tail-word deltas {tails}')
    if frame in trace:
        found = hypotheses(trace[frame][0], ours, theirs, max_steps)
        lines.append(f'single-stream changes turning ours into theirs (<= {max_steps} draws): '
                     + (json.dumps(found) if found else 'none'))
    return lines


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('trace')
    parser.add_argument('--frame', type=int, required=True)
    parser.add_argument('--ours', type=lambda v: int(v, 0), required=True)
    parser.add_argument('--theirs', type=lambda v: int(v, 0), required=True)
    parser.add_argument('--playback-frames')
    parser.add_argument('--console-frames')
    parser.add_argument('--window', type=int, default=8)
    parser.add_argument('--max-steps', type=int, default=8)
    parser.add_argument('--reconstruct', action='store_true',
                        help='follow --console-frames from the last traced frame to --frame, then test theirs there')
    args = parser.parse_args(argv)
    trace = load_trace(args.trace)
    playback = load_frames(args.playback_frames, 2) if args.playback_frames else None
    console = load_frames(args.console_frames, 1) if args.console_frames else None
    for line in report(trace, args.frame, args.ours, args.theirs, playback, console, args.window, args.max_steps):
        print(line)
    if args.reconstruct and trace and console:
        start = max(trace)
        states, reached, lines = reconstruct(trace[start][0], console, start, args.frame, args.max_steps)
        print(f'reconstruction from traced frame {start}: reached {reached}')
        for line in lines:
            print('  ' + line)
        if reached == args.frame:
            print(f'reconstructed value at {reached}: 0x{check_value(states):08x} (ours 0x{args.ours:08x})')
            for option in explain(states, args.theirs, args.max_steps):
                print('theirs explained by: ' + (', '.join(f'{NAMES[k]} +{n}' for k, n in option) or 'no change'))
    return 0 if trace else 1


if __name__ == '__main__':
    sys.exit(main())
