# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Drive Mario rightward through World 1-1 and get past what is in the way.

Holding Right alone dies to the first enemy. Releasing Right and waiting
does not help: an enemy already within range keeps closing in while Mario
stands still, so waiting only delays contact. Jumping at it (a stomp
attempt) is what works.

An earlier version of this released Right whenever any sprite came within
90 pixels, which is over half the screen width, so Right was released
almost permanently and Mario never built enough speed to clear anything.
Measured against real scroll (`sml_scroll.ScrollTracker`), that walker
covered 137 pixels in 5000 frames and then sat against a pipe around world
column 17 for the rest of the run, with the screen byte-identical between
frame 500 and frame 2999. It looked like it was travelling because the
position estimate it was judged by was itself broken.

So this version keeps Right held and reacts by jumping only:

* a sprite within `DANGER_RADIUS` (small, a stomp needs Mario nearly on top
  of the enemy anyway), or
* the world not scrolling for `STUCK_FRAMES` while grounded, which is what
  walking into a pipe looks like.

The stuck case needs a true scroll reading to work at all, which is why
this takes a `ScrollTracker`. Mario's own WRAM bytes cannot provide it: at
the camera lock his screen X pins at 81 whether he is walking or jammed
against a wall.

Mario's own sprite always occupies OAM slots 3-6 (the sprite attribute
table, the block of memory holding the position and tile of each of the
Game Boy's 40 hardware sprites), confirmed across every OAM dump taken for
this project, so those slots are excluded when scanning for danger.

Not a runnable tool: import `ReactiveWalker` from another `uv run` script.
"""

OAM_BASE = 0xFE00
MARIO_OAM_SLOTS = {3, 4, 5, 6}
MARIO_X = 0xC202
GROUNDED = 0xC20A

DANGER_RADIUS = 28
STOMP_COOLDOWN = 20
JUMP_HOLD = 12
STUCK_FRAMES = 12


def nearby_danger(pb, mario_x, radius=DANGER_RADIUS):
    for i in range(40):
        if i in MARIO_OAM_SLOTS:
            continue
        base = OAM_BASE + i * 4
        y, x = pb.memory[base], pb.memory[base + 1]
        if y == 0 or y >= 160:
            continue
        if abs(x - mario_x) <= radius:
            return True
    return False


class ReactiveWalker:
    """Holds Right, and jumps at enemies and at whatever stops the scroll.

    Call `step(pb, tracker)` once per frame, before `pb.tick()`.
    """

    def __init__(self, pb, radius=DANGER_RADIUS, stuck_frames=STUCK_FRAMES):
        pb.button_press("right")
        self.radius = radius
        self.stuck_frames = stuck_frames
        self.jump_cooldown = 0
        self.a_held = 0
        self.last_scroll = 0
        self.stalled = 0

    def step(self, pb, tracker):
        if tracker.scroll == self.last_scroll:
            self.stalled += 1
        else:
            self.stalled = 0
            self.last_scroll = tracker.scroll

        grounded = pb.memory[GROUNDED]
        blocked = self.stalled >= self.stuck_frames
        threatened = nearby_danger(pb, pb.memory[MARIO_X], self.radius)
        if (blocked or threatened) and grounded and self.jump_cooldown <= 0:
            pb.button_press("a")
            self.a_held = JUMP_HOLD
            self.jump_cooldown = STOMP_COOLDOWN

        if self.a_held > 0:
            self.a_held -= 1
            if self.a_held == 0:
                pb.button_release("a")
        if self.jump_cooldown > 0:
            self.jump_cooldown -= 1
