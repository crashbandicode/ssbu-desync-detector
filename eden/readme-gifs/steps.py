"""Project step hooks for the popup-route Eden runs (tools/eden-runner StepContext)."""
import shutil
from pathlib import Path


def copy_detector_folder(ctx, name='detector'):
    """Copy the instance's whole detector folder (events logs, frames-N.bin, match-N.bin) into the row
    folder, so the frame files survive the lifecycle's profile restore at Down."""
    source = Path(ctx.instance['detector_log']).parent
    target = Path(ctx.row_dir) / f'{ctx.on}-{name}'
    target.mkdir(parents=True, exist_ok=True)
    copied = []
    for path in sorted(source.iterdir()):
        if path.is_file():
            shutil.copy2(path, target / path.name)
            copied.append(f'{path.name}:{path.stat().st_size}')
    ctx.log({'copied_detector_folder': str(target), 'files': copied})
    return copied


def to_main(ctx, reference='/home/intpa/ssbu-mod-playbook/tools/eden-runner/library/refs/main.png',
            roi=(150, 265, 450, 335), tolerance=8, tries=6, wait_s=4.0):
    """From the opening movie, the attract demo or the title to the main menu: check the library main-menu
    reference; if it does not match, press + (skips a movie or the title; no action on the main menu) and
    look again. Every look is kept as a screenshot."""
    import time
    diffs = []
    for attempt in range(tries + 1):
        path = ctx.shot(f'to-main-{attempt}')
        diff = ctx.runner.roi_diff(path, reference, list(roi), True)
        diffs.append(round(diff, 1))
        if diff <= tolerance:
            ctx.log({'to_main': 'main', 'attempt': attempt, 'diffs': diffs})
            return {'attempt': attempt, 'diffs': diffs}
        if attempt < tries:
            ctx.press('plus', 150)
            time.sleep(wait_s)
    raise RuntimeError(f'main menu not reached after {tries} presses; diffs {diffs}')


def copy_profile_dir(ctx, rel='user/sdmc/ultimate/arcropolis', name='arcropolis', limit=50 * 1024 * 1024):
    """Copy a folder of this instance's profile copy (for example ARCropolis's own logs) into the row
    folder; files over `limit` bytes are listed, not copied."""
    import shutil
    source = Path(ctx.instance['profile']) / rel
    target = Path(ctx.row_dir) / f'{ctx.on}-{name}'
    copied, skipped = [], []
    if source.is_dir():
        for path in sorted(source.rglob('*')):
            if not path.is_file():
                continue
            relative = path.relative_to(source)
            if path.stat().st_size > limit:
                skipped.append(f'{relative}:{path.stat().st_size}')
                continue
            (target / relative).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target / relative)
            copied.append(str(relative))
    ctx.log({'copied_profile_dir': str(source), 'files': len(copied), 'skipped': skipped})
    return {'files': len(copied), 'skipped': skipped}


# 13.0.5: the RNG manager slot (`*(*(main+0x52c35b0))` is the manager, its first word the stream array of
# nine 0x14-byte xorshift128 states, check word at +0xc), the replay playback check and the replay frame index
# (game x27 +0x590 -> player, +0x200078), as in src/model.rs.
RNG_MANAGER = 0x52c35b0
PLAYBACK_CHECK = 0x135fd60
STREAMS_BYTES = 9 * 0x14


def _main_base(modules):
    """The main module's base from modules.list, whatever its exact shape."""
    entries = modules.get('modules', modules) if isinstance(modules, dict) else modules
    for entry in entries or []:
        name = str(entry.get('name') or entry.get('module') or '')
        if name == 'main' or name.startswith('main') and 'nro' not in name:
            for key in ('base', 'start', 'address', 'load_base'):
                if entry.get(key) is not None:
                    value = entry[key]
                    return int(value, 16) if isinstance(value, str) else int(value)
    raise RuntimeError(f'no main module in modules.list: {str(modules)[:400]}')


def press_until(ctx, reference, roi, button='a', tries=3, wait_s=3.0, tolerance=8):
    """Look first; while the screen's region does not match `reference`, press `button` (a pop-up's OK) and
    look again. No press when the screen already matches. Every look is kept as a screenshot."""
    import time
    base = Path(ctx.runner.base)
    path_ref = reference if Path(reference).is_absolute() else str(base / reference)
    from PIL import Image
    with Image.open(path_ref) as image:
        ref_is_roi = image.size == (roi[2] - roi[0], roi[3] - roi[1])
    diffs = []
    for attempt in range(tries + 1):
        path = ctx.shot(f'press-until-{attempt}')
        diff = ctx.runner.roi_diff(path, path_ref, list(roi), ref_is_roi)
        diffs.append(round(diff, 1))
        if diff <= tolerance:
            ctx.log({'press_until': reference, 'attempt': attempt, 'diffs': diffs})
            return {'attempt': attempt, 'diffs': diffs}
        if attempt < tries:
            ctx.press(button, 150)
            time.sleep(wait_s)
    raise RuntimeError(f'{reference} not reached after {tries} presses; diffs {diffs}')


def to_vault(ctx, main_ref='/home/intpa/ssbu-mod-playbook/tools/eden-runner/library/refs/main.png',
             main_roi=(150, 265, 450, 335), gm_ref='refs/main-gm.png', gm_roi=(740, 105, 1070, 170), tolerance=8,
             tries=8, wait_s=4.0):
    """From the attract movie, the title, or the main menu with Smash or Games & More selected, to the Vault
    menu: look; Smash selected -> the library main-to-vault segment; Games & More selected -> Down, A;
    anything else -> + (skips the movie or the title) and look again. The next gate checks the Vault menu."""
    import time
    gm_path = gm_ref if Path(gm_ref).is_absolute() else str(Path(ctx.runner.base) / gm_ref)
    from PIL import Image
    with Image.open(gm_path) as image:
        gm_is_roi = image.size == (gm_roi[2] - gm_roi[0], gm_roi[3] - gm_roi[1])
    looks = []
    for attempt in range(tries + 1):
        path = ctx.shot(f'to-vault-{attempt}')
        d_main = ctx.runner.roi_diff(path, main_ref, list(main_roi), True)
        d_gm = ctx.runner.roi_diff(path, gm_path, list(gm_roi), gm_is_roi)
        looks.append([round(d_main, 1), round(d_gm, 1)])
        if d_main <= tolerance:
            ctx.play('main-to-vault')
            ctx.log({'to_vault': 'from main', 'looks': looks})
            return {'from': 'main', 'looks': looks}
        if d_gm <= tolerance:
            ctx.press('down', 150)
            time.sleep(1.0)
            ctx.press('a', 150)
            time.sleep(6.0)
            ctx.log({'to_vault': 'from games-and-more', 'looks': looks})
            return {'from': 'games-and-more', 'looks': looks}
        if attempt < tries:
            ctx.press('plus', 150)
            time.sleep(wait_s)
    raise RuntimeError(f'main menu not reached after {tries} presses; looks [main, gm] {looks}')


def run_off_and_watch(ctx, mover='b', peer='a', run_s=4.0, fall_s=4.0, direction=1.0, cap_s=150,
                      pattern=r'desync frame=\d+|sync_end trigger=\w+'):
    """`mover` loses a stock on purpose: it runs `direction` (1 right, -1 left) for run_s, then holds that way and down
    for fall_s (off the stage, past the ledge, fast fall). `peer` gets varied input meanwhile (walk left, walk right,
    jump, shield every 0.7 s) so it is not dropped for inactivity. Then both get varied input until either detector
    logs `pattern` or cap_s pass; input is released. Returns the matched lines per instance."""
    import os
    import re
    import time
    runner = ctx.runner
    logs = {on: runner.instances[on]['detector_log'] for on in (mover, peer)}
    starts = {on: os.path.getsize(path) for on, path in logs.items()}

    def vary(on, tick):
        kind = tick % 4
        if kind in (0, 1):
            runner.call(on, 'input.stick', {'stick': 'left', 'x': -1 if kind == 0 else 1, 'y': 0, 'lease_ms': 300})
        else:
            runner.call(on, 'input.button', {'button': 'x' if kind == 2 else 'zl', 'state': 'tap',
                                             'duration_ms': 100, 'lease_ms': 600})

    found, tick = {}, 0
    began = time.monotonic()
    try:
        while time.monotonic() - began < run_s + fall_s:
            ctx.check_stop()
            down = -1 if time.monotonic() - began >= run_s else 0
            runner.call(mover, 'input.stick', {'stick': 'left', 'x': direction, 'y': down, 'lease_ms': 400})
            vary(peer, tick)
            tick += 1
            time.sleep(0.25)
        runner.call(mover, 'input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
        ctx.log({'run_off': mover, 'seconds': round(time.monotonic() - began, 1)})
        until = time.monotonic() + cap_s
        while time.monotonic() < until and not found:
            ctx.check_stop()
            vary(mover, tick + 2)
            vary(peer, tick)
            tick += 1
            time.sleep(0.7)
            for on, path in logs.items():
                hits = [line for line in _new_lines(path, starts[on]).splitlines() if re.search(pattern, line)]
                if hits:
                    found[on] = hits[:4]
    finally:
        for on in logs:
            runner.call(on, 'input.reset')
    time.sleep(2)
    for on, path in logs.items():
        found.setdefault(on, [line for line in _new_lines(path, starts[on]).splitlines() if re.search(pattern, line)][:4])
    ctx.log({'run_off_and_watch': found})
    if not any(found.values()):
        raise RuntimeError(f'no {pattern!r} on {mover} or {peer} within {cap_s} s of the run-off')
    return found


def probe_input(ctx, buttons=('b',), hold_ms=300, settle_s=2.5):
    """Diagnose a screen that ignores a segment: tap each button with input.button, keep its full reply (the
    engine's accepted/observed/frontend/gated status) and a screenshot after it settles."""
    import time
    replies = []
    for index, button in enumerate(buttons):
        reply = ctx.call('input.button', {'player': 0, 'button': button, 'state': 'tap', 'duration_ms': hold_ms,
                                          'lease_ms': hold_ms + 500})
        time.sleep(hold_ms / 1000 + settle_s)
        path = ctx.shot(f'probe-{index}-{button}')
        replies.append({'button': button, 'reply': reply, 'shot': Path(path).name})
    ctx.log({'probe_input': replies})
    return replies


def keepalive_until_end(ctx, peer='b', cap_s=300, pattern=r'sync_end trigger=\w+', walkoff_s=0.0):
    """Both players get varied input every 0.7 s (walk left, walk right, jump, shield) so neither is dropped
    for inactivity (Down + ZL taps alone were dropped in dd-popup-y05), until both detectors log the
    session's end (or cap_s). Then input is released. With
    walkoff_s, `peer` first runs right that long (off the stage: one self-destruct), so a timed match
    ends on time instead of a 0-0 sudden death."""
    import os
    import re
    import time
    logs = {on: ctx.runner.instances[on]['detector_log'] for on in (ctx.on, peer)}
    starts = {on: os.path.getsize(path) for on, path in logs.items()}
    ended = {}
    begun = time.monotonic()
    until = begun + cap_s
    step_index = 0
    try:
        while time.monotonic() < until and len(ended) < len(logs):
            ctx.check_stop()
            # Varied input (Eden's LAN notes: Quickplay drops a player whose input does not vary): a short
            # walk left, a short walk right, a jump, a shield, in turn.
            kind = step_index % 4
            step_index += 1
            for on in logs:
                if on == peer and time.monotonic() - begun < walkoff_s:
                    ctx.runner.call(on, 'input.stick', {'stick': 'left', 'x': 1, 'y': 0, 'lease_ms': 1500})
                elif kind in (0, 1):
                    ctx.runner.call(on, 'input.stick', {'stick': 'left', 'x': -1 if kind == 0 else 1, 'y': 0,
                                                        'lease_ms': 250})
                else:
                    ctx.runner.call(on, 'input.button', {'button': 'x' if kind == 2 else 'zl', 'state': 'tap',
                                                         'duration_ms': 100, 'lease_ms': 600})
            time.sleep(0.7)
            for on, path in logs.items():
                found = [line for line in _new_lines(path, starts[on]).splitlines() if re.search(pattern, line)]
                if found:
                    ended[on] = found[0]
    finally:
        for on in logs:
            ctx.runner.call(on, 'input.reset')
    ctx.log({'session_end': ended})
    if len(ended) < len(logs):
        raise RuntimeError(f'no session end on {sorted(set(logs) - set(ended))} within {cap_s} s')
    return ended


def _arm(ctx, on, point):
    result = ctx.runner.call(on, 'trace.arm', point)
    ctx.log({'armed': point, 'on': on, 'result': result})
    return result


def arm_popup_trace(ctx, on=None, read_back=False):
    """Live points on `on` (default this instance). A capture expression's value is the address read.
    1. pop-up body text .1eef640: label (x1) and the argument vector's begin/end (x2) -> which branch;
    2. plain route .377a250 when .377a120 was called by the pop-up body (saved LR .1eef838): the text and count;
    3. the args route's caller .37a2660 when called by the pop-up body (LR .1eef850): label;
    4. the pane setter .37a2270 for a text starting "A co": the text it receives and its count."""
    on = on or ctx.on
    base = _main_base(ctx.runner.call(on, 'modules.list'))
    points = [
        {'target': 'main+0x1eef640', 'registers': ['x0', 'x1', 'x2', 'x30'], 'backtrace': True,
         'captures': [{'expression': 'x1', 'type': 'c-string(96)'}, {'expression': 'x2', 'type': 'u64'},
                      {'expression': 'x2+8', 'type': 'u64'}]},
        {'target': 'main+0x377a250', 'registers': ['x0', 'x29', 'sp'], 'backtrace': True,
         'condition': f'[sp+0x1158] == 0x{base + 0x1eef838:x}',
         'captures': [{'expression': 'sp+0x120:320', 'type': 'bytes'}]},
        {'target': 'main+0x37a2660', 'registers': ['x0', 'x1', 'x2', 'x30'], 'backtrace': True,
         'condition': f'x30 == 0x{base + 0x1eef850:x}',
         'captures': [{'expression': 'x1', 'type': 'c-string(96)'}]},
        {'target': 'main+0x37a2270', 'registers': ['x0', 'x1', 'x2', 'x3', 'x30'], 'backtrace': True,
         'condition': '[x1] == 0x6f006300200041',
         'captures': [{'expression': 'x1:400', 'type': 'bytes'}]},
    ]
    results = [_arm(ctx, on, point) for point in points]
    return {'main_base': hex(base), 'armed': results}


def _new_lines(path, start):
    with open(path, 'rb') as handle:
        handle.seek(start)
        return handle.read().decode('utf-8', 'replace')


def keepalive_then_drop(ctx, run_s=14, wait_cap=200, peer='b', after_s=6,
                        pattern=r'communication_text|sync_end|peer_end'):
    """Both players shield-tap (ZL) for run_s so neither is dropped for inactivity; then `peer` is paused and
    this instance keeps tapping until its detector logs one of `pattern` (the session's end), then stops
    input (so the pop-up stays) and waits after_s for it to settle. Returns the matched lines."""
    import os
    import re
    import time

    def tap(on):
        ctx.runner.call(on, 'input.button', {'button': 'zl', 'state': 'tap', 'duration_ms': 100, 'lease_ms': 600})

    log = ctx.instance['detector_log']
    start = os.path.getsize(log)
    end = time.monotonic() + run_s
    while time.monotonic() < end:
        ctx.check_stop()
        tap(ctx.on)
        tap(peer)
        time.sleep(0.7)
    ctx.runner.call(peer, 'game.pause', {})
    ctx.log({'paused': peer, 'after_s': run_s})
    found = []
    until = time.monotonic() + wait_cap
    while time.monotonic() < until and not found:
        ctx.check_stop()
        found = [line for line in _new_lines(log, start).splitlines() if re.search(pattern, line)]
        if not found:
            tap(ctx.on)
            time.sleep(0.8)
    time.sleep(after_s)
    found = [line for line in _new_lines(log, start).splitlines() if re.search(pattern, line)]
    ctx.log({'session_end_lines': found})
    if not found:
        raise RuntimeError(f'no {pattern!r} on {ctx.on} within {wait_cap} s after pausing {peer}')
    return found


def read_trace(ctx, on=None, name='trace'):
    """Save every in-memory trace record of `on` (points armed live, no JSONL there) into the row folder."""
    import json
    on = on or ctx.on
    records, after = [], '0'
    for _ in range(20):
        page = ctx.runner.call(on, 'trace.read', {'after': after, 'limit': 512}) or {}
        batch = page.get('records') or []
        records += batch
        nxt = page.get('next')
        if not batch or not nxt or nxt == after:
            break
        after = nxt
    path = Path(ctx.row_dir) / f'{on}-{name}.json'
    path.write_text(json.dumps(records, indent=1))
    ctx.log({'trace_records': len(records), 'on': on, 'path': str(path)})
    return len(records)


def arm_position_trace(ctx):
    """Arm a per-frame point on the playback check that keeps what the position checksum is made of: the
    replay frame index, the recorded checksum ([x22+4]) and the eight 16-byte position vectors at sp+0x70
    (x, y, z, 0 per fighter slot) whose x/y/z bit patterns the game adds into w8 (.135fc84-.135fd5c)."""
    point = {
        'target': f'main+0x{PLAYBACK_CHECK:x}',
        'registers': ['x8', 'x22', 'x27', 'sp'],
        'captures': [
            {'expression': '[x27+0x590]+0x200078', 'type': 'u32'},
            {'expression': 'x22+4', 'type': 'u32'},
            {'expression': 'sp+0x70:128', 'type': 'bytes'},
        ],
        'rate_limit': 1000,
    }
    result = ctx.call('trace.arm', point)
    ctx.log({'armed_position_trace': point, 'result': result})
    return {'result': result}


def arm_rng_trace(ctx):
    """Arm a per-frame point on the playback check: the replay frame index, the computed position checksum (w8)
    and the nine RNG stream states, so a desync's stream can be read off the vanilla playback."""
    base = _main_base(ctx.call('modules.list'))
    slot = base + RNG_MANAGER
    point = {
        'target': f'main+0x{PLAYBACK_CHECK:x}',
        'registers': ['x8', 'x22', 'x27'],
        # A capture expression's value is the address read: player+0x200078, and the stream array
        # (slot -> pointer -> manager -> array).
        'captures': [
            {'expression': '[x27+0x590]+0x200078', 'type': 'u32'},
            {'expression': f'[[[0x{slot:x}]]]:{STREAMS_BYTES}', 'type': 'bytes'},
        ],
        'rate_limit': 1000,
    }
    result = ctx.call('trace.arm', point)
    ctx.log({'armed_rng_trace': point, 'main_base': hex(base), 'result': result})
    return {'main_base': hex(base), 'result': result}


def record_run_off(ctx, mover='b', peer='a', record_on='a', duration_ms=45000, width=1280, fps=10,
                   run_s=4.0, fall_s=4.0, direction=1.0, cap_s=150):
    """Start Eden's recorder on `record_on`, then run that console's opponent off the stage.

    The GIF is the whole guest at `width` (1280 for a README clip). Recording makes the game slower, so the
    take is longer than 10 s and is trimmed afterwards. The file is written only when the recorder finishes.
    """
    import time
    path = Path(ctx.row_dir) / f'{record_on}-popup-raw.gif'
    if path.exists():
        raise RuntimeError(f'recording path already exists: {path}')
    started = ctx.runner.call(record_on, 'recording.start', {
        'path': str(path), 'fps': int(fps), 'width': int(width), 'duration_ms': int(duration_ms),
    })
    ctx.log({'recording_start': started, 'path': str(path), 'width': width, 'duration_ms': duration_ms})
    time.sleep(1.0)
    found = run_off_and_watch(ctx, mover=mover, peer=peer, run_s=run_s, fall_s=fall_s, direction=direction,
                              cap_s=cap_s,
                              pattern=r'desync frame=\d+|communication_text reason=\w+|sync_end trigger=\w+')
    time.sleep(4.0)
    status = ctx.runner.call(record_on, 'recording.status', {})
    ctx.log({'recording_status': status})
    if isinstance(status, dict) and status.get('state') == 'recording':
        ctx.runner.call(record_on, 'recording.stop', {})
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            status = ctx.runner.call(record_on, 'recording.status', {})
            if not isinstance(status, dict) or status.get('state') != 'recording':
                break
            time.sleep(0.5)
    size = path.stat().st_size if path.exists() else 0
    ctx.log({'recording_final': status, 'bytes': size})
    if size < 1000:
        raise RuntimeError(f'recorder left no GIF at {path} ({size} bytes): {status}')
    return {'found': found, 'gif': str(path), 'bytes': size}
