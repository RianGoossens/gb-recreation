# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Shared PyBoy boot sequences for the ROM-observation tools.

Every extraction/observation script needs to reach either the title screen
or a controllable Mario in World 1-1 before it can read anything. The exact
frame counts (how long the title screen takes to settle, how long Start
needs to be held, how long the level takes to load) are the same for all of
them, so they live here once instead of being copy-pasted into each script.

Not a runnable tool itself: import it from another `uv run` script, e.g.
`from sml_boot import boot_to_gameplay`.
"""

from pyboy import PyBoy

ROM = "super_mario_land.gb"
BOOT_FRAMES = 600
START_PRESS_FRAMES = 10
GAMEPLAY_SETTLE_FRAMES = 300


def boot_to_title(rom=ROM):
    """Boot the ROM and wait for the title screen to be up."""
    pb = PyBoy(rom, window="null")
    for _ in range(BOOT_FRAMES):
        pb.tick()
    return pb


def boot_to_gameplay(rom=ROM):
    """Boot the ROM, press Start, and wait for a controllable Mario in 1-1."""
    pb = boot_to_title(rom)
    pb.button_press("start")
    for _ in range(START_PRESS_FRAMES):
        pb.tick()
    pb.button_release("start")
    for _ in range(GAMEPLAY_SETTLE_FRAMES):
        pb.tick()
    return pb
