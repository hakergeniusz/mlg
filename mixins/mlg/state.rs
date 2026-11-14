// The save's progress, game-side: which step the machine is on and whose
// hotbar slot was the player's own. Pure bookkeeping — every decision is the
// runtime's, every action the tick helper's.
// Copyright (c) 2026 hakergeniusz. MIT.
use core::cell::RefCell;

/// One fall save, from the bucket's selection to the water's pickup.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Idle,
    /// The bucket slot is chosen; the carried-slot packet goes out this
    /// tick, so the next tick's use is read with the right slot held.
    Selecting { slot: usize, saved: usize },
    /// The use goes out now, at the floor the feet' ray finds.
    Placing { slot: usize, saved: usize, tries: u32 },
    /// Placed; waiting for the landing, then the pickup.
    Placed { slot: usize, saved: usize, ticks: u32, water: [i32; 3] },
    /// Filling the bucket back up from the placed water.
    Picking { slot: usize, saved: usize, tries: u32, water: [i32; 3] },
    /// Handing the hotbar back to the player.
    Done { saved: usize },
}

thread_local! {
    static MODE: RefCell<Mode> = const { RefCell::new(Mode::Idle) };
}

/// The machine's step now.
pub fn get() -> Mode {
    MODE.with(|m| *m.borrow())
}

/// Move the machine to `mode`.
pub fn set(mode: Mode) {
    MODE.with(|m| *m.borrow_mut() = mode);
}

/// Forget the save: the next falling tick starts over.
pub fn reset() {
    set(Mode::Idle);
}

/// One block use at `hit` with the main hand, through the game's own
/// pipeline: the forced target makes the pick the floor's, and the guest's
/// block-use shape keeps an entity in the crosshair from taking the click.
/// Returns whether a hand took the use.
pub fn use_on(game: &mut crate::game::Game<'_>, hit: crate::game::Hit) -> bool {
    let forced = game.forced_hit.take();
    game.forced_hit = Some(hit);
    let guest = game.guest_use;
    game.guest_use =
        Some(crate::game::GuestUse::OnBlock(horizon_client::first_person::Hand::Main));
    let took = game.use_item().map(|used| used.is_some()).unwrap_or(false);
    game.guest_use = guest;
    game.forced_hit = forced;
    took
}
