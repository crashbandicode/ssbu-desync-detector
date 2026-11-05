"""Host tests for stream_attribution.py: python3 -I -B -m unittest discover -s tools -p test_stream_attribution.py"""
import json
import os
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stream_attribution as sa  # noqa: E402


def seeded(seed):
    """Nine distinct non-zero xorshift128 states."""
    return [((seed * 7 + k * 0x9e3779b9) & sa.MASK or 1, (seed + k * 3 + 1) & sa.MASK, (k * 0x85ebca6b + 5) & sa.MASK,
             (seed ^ (k * 0xc2b2ae35) ^ 9) & sa.MASK) for k in range(9)]


def raw(states, tails):
    return b''.join(struct.pack('<5I', *s, t) for s, t in zip(states, tails))


def trace_file(frames):
    """frames: {player index: (states, tails)} -> a JSONL file shaped like Eden's trace records."""
    handle = tempfile.NamedTemporaryFile('w', suffix='.jsonl', delete=False)
    handle.write(json.dumps({'schema': 1, 'event': 'loss', 'records': '0'}) + '\n')
    for index, (states, tails) in frames.items():
        handle.write(json.dumps({'event': 'entry', 'captures': [
            {'expression': '[[x27+0x590]+0x200078]', 'type': 'u32', 'value': str(index)},
            {'expression': '[...]:180', 'type': 'bytes', 'bytes': raw(states, tails).hex()}]}) + '\n')
    handle.close()
    return handle.name


class StreamAttribution(unittest.TestCase):
    def test_step_and_unstep_are_inverse(self):
        for state in seeded(11):
            self.assertEqual(sa.unstep(sa.step(state)), state)
            self.assertEqual(sa.advance(sa.advance(state, 5), -5), state)

    def test_draws_between_frames_are_counted_per_stream(self):
        before = seeded(3)
        after = list(before)
        after[1] = sa.advance(before[1], 2)
        after[7] = sa.advance(before[7], 1)
        counts = [sa.steps_between(a, b) for a, b in zip(before, after)]
        self.assertEqual(counts, [0, 2, 0, 0, 0, 0, 0, 1, 0])
        self.assertIsNone(sa.steps_between(before[0], before[1], limit=8))

    def test_a_planted_extra_draw_is_found_and_a_fair_value_is_not(self):
        states = seeded(5)
        ours = sa.check_value(states)
        other = list(states)
        other[7] = sa.advance(states[7], 1)  # the other console drew `sound` once more
        theirs = sa.check_value(other)
        found = sa.hypotheses(states, ours, theirs, 4)
        self.assertIn({'stream': 7, 'name': 'sound', 'draws': 1}, found)
        self.assertEqual(sa.hypotheses(states, ours, ours ^ 0x1234567, 4), [])

    def test_reconstruction_follows_logged_values_and_stops_when_ambiguous(self):
        start = seeded(9)
        frames, states = {}, list(start)
        steps = {11: (1, 1), 13: (7, 2), 14: (1, 1)}
        for f in range(11, 16):
            if f in steps:
                k, n = steps[f]
                states[k] = sa.advance(states[k], n)
            frames[f] = sa.check_value(states)
        rebuilt, reached, lines = sa.reconstruct(start, frames, 10, 15)
        self.assertEqual(reached, 15)
        self.assertEqual(rebuilt, states)
        self.assertEqual(lines, ['frame 11: fighter +1', 'frame 13: sound +2', 'frame 14: fighter +1'])
        # A value no single or paired advance explains stops the walk at that frame.
        frames[16] = frames[15] ^ 0x5a5a5a5a
        _, reached, lines = sa.reconstruct(start, frames, 10, 16)
        self.assertEqual(reached, 15)
        self.assertTrue(lines[-1].startswith('frame 16: no explanation'))

    def test_trace_records_load_by_detector_frame_and_check_against_frames_files(self):
        first, second = seeded(1), seeded(1)
        second[1] = sa.advance(first[1], 1)
        path = trace_file({101: (first, [0] * 9), 102: (second, [0, 1] + [0] * 7)})
        try:
            trace = sa.load_trace(path)
        finally:
            os.unlink(path)
        self.assertEqual(sorted(trace), [100, 101])
        self.assertEqual(trace[101][1][1], 1)
        playback = {100: sa.check_value(first), 101: sa.check_value(second)}
        console = {100: sa.check_value(first), 101: sa.check_value(first)}
        lines = sa.report(trace, 101, sa.check_value(first), sa.check_value(second), playback, console, 2, 2)
        self.assertIn('trace XOR vs detector playback value: 2/2 frames equal', lines)
        self.assertIn('vanilla vs this console: 1/2 frames equal, first differing 101', lines)
        self.assertTrue(any(line.startswith("frame 101: draws {'fighter': 1}") for line in lines))


if __name__ == '__main__':
    unittest.main()
