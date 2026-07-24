# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Find Mario's horizontal speed byte in WRAM, and read its real per-frame
values while walking.

Same philosophy as find_rom_offset.py: observe, don't read the disassembly.
We boot the ROM, start World 1-1, hold Right, and snapshot all of WRAM
(0xC000-0xDFFF) every frame. Mario's horizontal speed accumulator has a
distinctive signature: it climbs by a fixed step each frame while Right is
held (acceleration), saturates at a fixed value (max speed), then falls back
toward zero by a different fixed step once Right is released (friction), and
holds at zero rather than going negative. No other WRAM byte behaves like
that, so scanning for the signature finds the address with no address table
or disassembly lookup needed.

Run: uv run tools/find_mario_speed.py
"""

from sml_boot import boot_to_gameplay

WRAM_START = 0xC000
WRAM_END = 0xE000


def s8(b):
    return b - 256 if b >= 128 else b


def main():
    pb = boot_to_gameplay()

    frames = []
    pb.button_press("right")
    for _ in range(90):
        pb.tick()
        frames.append(bytes(pb.memory[WRAM_START:WRAM_END]))
    pb.button_release("right")
    for _ in range(60):
        pb.tick()
        frames.append(bytes(pb.memory[WRAM_START:WRAM_END]))

    pb.stop()

    n = WRAM_END - WRAM_START
    hold_len = 90
    candidates = []
    for i in range(n):
        vals = [s8(f[i]) for f in frames]
        if all(v == 0 for v in vals):
            continue
        hold = vals[:hold_len]
        release = vals[hold_len:]
        # Rising while held: monotonic non-decreasing, reaches a plateau,
        # starts at 0, and is not just noise (moves at least a few steps).
        rising_steps = sum(1 for a, b in zip(hold, hold[1:]) if b > a)
        plateau = hold[-1]
        if hold[0] != 0 or plateau <= 0 or rising_steps < 3:
            continue
        # Falling back toward zero (or steady) after release, not still rising.
        falls = sum(1 for a, b in zip(release, release[1:]) if b < a)
        if falls < 3:
            continue
        if release[-1] != 0:
            continue
        candidates.append((i + WRAM_START, hold, release))

    print(f"{len(candidates)} candidate address(es)")
    for addr, hold, release in candidates:
        steps = [b - a for a, b in zip(hold, hold[1:]) if b != a]
        accel_step = steps[0] if steps else None
        max_val = max(hold)
        fall_steps = [a - b for a, b in zip(release, release[1:]) if b != a]
        friction_step = fall_steps[0] if fall_steps else None
        print(f"0x{addr:04X}: accel_step={accel_step} max={max_val} friction_step={friction_step}")
        print(f"  hold:    {hold}")
        print(f"  release: {release}")


if __name__ == "__main__":
    main()
