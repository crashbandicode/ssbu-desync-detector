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


def arm_rng_trace(ctx):
    """Arm a per-frame point on the playback check: the replay frame index, the computed position checksum (w8)
    and the nine RNG stream states, so a desync's stream can be read off the vanilla playback."""
    base = _main_base(ctx.call('modules.list'))
    slot = base + RNG_MANAGER
    point = {
        'target': f'main+0x{PLAYBACK_CHECK:x}',
        'registers': ['x8', 'x22', 'x27'],
        'captures': [
            {'expression': '[[x27+0x590]+0x200078]', 'type': 'u32'},
            {'expression': f'[[[[0x{slot:x}]]]]:{STREAMS_BYTES}', 'type': 'bytes'},
        ],
        'rate_limit': 1000,
    }
    result = ctx.call('trace.arm', point)
    ctx.log({'armed_rng_trace': point, 'main_base': hex(base), 'result': result})
    return {'main_base': hex(base), 'result': result}
