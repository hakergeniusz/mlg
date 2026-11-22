//! MLG, the native-route native mod: the water bucket placed under you in
//! the last stretch of a fall, and picked back up once you have landed.
//!
//! The brain is pure standard library — no game types — so it compiles and
//! tests wherever. A fall only hurts from the height built since the last
//! touch of water, and a water bucket poured into the block above the floor
//! resets it where the game's own rules run — the server's sim, on a world
//! that has no mods at all. The bucket is a real use of a real item: the
//! click is the game's own pipeline with a forced target (`forced_hit`), so
//! on a world this client only watches it goes out as the player's own
//! remote-use press, and on a hosted world it runs the host's use pipeline —
//! the server sees a legal placement inside its reach check, not a spoofed
//! number, so the save holds on any server this mod is installed on.
//!
//! The save is latency-bound: the water must exist before the feet arrive,
//! and a use only leads the fall by so far (about six blocks below the feet
//! on a server's reach check). Low-latency worlds (a LAN, a nearby dedicated
//! server) save most falls; a distant server saves the slower ones.
//!
//! Clean-room: written for Horizon from the game's own public behavior; it
//! copies nothing from any mod.
//!
//! Copyright (c) 2026 hakergeniusz. MIT.

use core::cell::RefCell;

use mod_api::{Hooks, RegistrationContext, Registrar};

/// Feet-to-floor distance inside which the bucket goes out: the highest
/// placement a server's reach check lets through with margin for the fall
/// made while the use travels (5.5 from the eyes is survival's own).
pub const TRIGGER_DISTANCE: f64 = 6.0;

/// A fall only worth a bucket once it has built this much height: a one
/// block hop spills nothing.
pub const MIN_FALL: f64 = 2.0;

/// Downward speed below which the feet are truly falling, not settling from
/// a step.
pub const MIN_SINK: f64 = 0.35;

/// The reach of the feet' downward floor search, a little past the trigger
/// so the window's edge is a real measurement, not the ray's cutoff.
pub const FLOOR_DEPTH: f64 = TRIGGER_DISTANCE + 2.0;

/// Placement and pickup attempts before the fall is given up to the ground.
pub const USE_TRIES: u32 = 3;

/// Ticks a placed save may wait for the landing before the state gives up
/// and hands the slot back: a long glide in placed water, a teleport.
pub const PLACED_TIMEOUT: u32 = 300;

/// Ticks between the landing and the pickup use — and between pickup
/// attempts, and the grace a landed-but-not-yet-emptied slot gets before
/// the save gives up (the emptied slot rides the server's confirmation, a
/// latency wide): let the landing settle, so the water click is not the
/// same tick the feet stop.
pub const PICKUP_DELAY: u32 = 5;

/// Whether a fall as described should carry a bucket at all: off the ground,
/// truly sinking, already high enough to hurt, and not floating in water.
pub fn wants_save(on_ground: bool, sink: f64, fall: f64, in_water: bool) -> bool {
    !on_ground && !in_water && sink < -MIN_SINK && fall >= MIN_FALL
}

/// Whether a floor this far below the feet is inside the placement window.
pub fn in_window(distance: f64) -> bool {
    (0.0..=TRIGGER_DISTANCE).contains(&distance)
}

/// Whether a fall this far from a hurt would be: the window measured from
/// the hit point the floor ray returned.
pub fn floor_distance(feet_y: f64, hit_point_y: f64) -> f64 {
    feet_y - hit_point_y
}

/// One fall save, from the bucket's selection to the water's pickup.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Idle,
    /// The bucket slot is chosen; the carried-slot packet goes out this
    /// tick, so the next tick's use is read with the right slot held.
    Selecting { slot: usize, saved: usize },
    /// The use goes out now, at the floor the feet' ray finds — retried
    /// while a click is seen to be refused, on a host where the answer is
    /// the pipeline's own.
    Placing { slot: usize, saved: usize, tries: u32 },
    /// Placed; waiting for the landing, then the pickup.
    Placed { slot: usize, saved: usize, ticks: u32, water: [i32; 3] },
    /// Filling the bucket back up from the placed water: the click targets
    /// the water itself.
    Picking { slot: usize, saved: usize, tries: u32, water: [i32; 3] },
    /// A pickup click went out; waiting for the filled bucket to come home.
    Picked { slot: usize, saved: usize, tries: u32, water: [i32; 3], ticks: u32 },
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

/// What a placed save does this tick.
pub enum PlacedDecision {
    /// Hand the hotbar back: the fall is over or the water never came.
    GiveUp,
    /// Landed, the bucket came home empty, and the landing has settled:
    /// start the pickup.
    PickUp,
    /// Keep waiting, one tick older.
    Wait,
}

/// A placed save's tick: landed (`on_ground` or floating in the placed
/// water), whether the held slot emptied (it rides the server's
/// confirmation, so a fresh landing gets the same grace the pickup does),
/// and the timeout for a glide that never comes down.
pub fn placed_step(ticks: u32, on_ground: bool, in_water: bool, emptied: bool) -> PlacedDecision {
    if ticks + 1 >= PLACED_TIMEOUT {
        PlacedDecision::GiveUp
    } else if on_ground || in_water {
        if emptied {
            if ticks + 1 >= PICKUP_DELAY {
                PlacedDecision::PickUp
            } else {
                PlacedDecision::Wait
            }
        } else if ticks + 1 >= PICKUP_DELAY {
            PlacedDecision::GiveUp
        } else {
            PlacedDecision::Wait
        }
    } else {
        PlacedDecision::Wait
    }
}

/// What a pickup wait does this tick.
pub enum PickedDecision {
    /// The bucket came home full, or the tries ran out: done.
    GiveUp,
    /// The wait ran out with tries left: another click goes out.
    Retry,
    /// Keep waiting for the server's confirmation, one tick older.
    Wait,
}

/// A pickup wait's tick: whether the held slot is the filled bucket again,
/// and how many clicks have gone out so far.
pub fn picked_step(ticks: u32, tries: u32, filled: bool) -> PickedDecision {
    if filled {
        PickedDecision::GiveUp
    } else if ticks + 1 >= PICKUP_DELAY {
        if tries + 1 >= USE_TRIES {
            PickedDecision::GiveUp
        } else {
            PickedDecision::Retry
        }
    } else {
        PickedDecision::Wait
    }
}

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// The mod registers nothing: the save has no toggle — installing it is
    /// asking for it, and a fall decides the rest.
    fn register(&self, _: &mut dyn Registrar, _: &RegistrationContext) {}

    /// Implemented as a no-op so the game always calls the
    /// `native_client_tick_pre` dispatch the tick mixin lives in: the game
    /// skips dispatches no mod listens to, and the mixin must run.
    fn client_tick_pre(&self, _: &mut dyn mod_api::ClientHost, _: &mut mod_api::ClientTickPreEvent) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_true_fall_wants_a_save() {
        assert!(wants_save(false, -0.6, 5.0, false));
    }

    #[test]
    fn the_ground_water_and_hops_do_not() {
        assert!(!wants_save(true, -0.6, 5.0, false), "standing");
        assert!(!wants_save(false, -0.1, 5.0, false), "settling, not sinking");
        assert!(!wants_save(false, -0.6, 1.5, false), "a hop hurts nothing");
        assert!(!wants_save(false, -0.6, 5.0, true), "already in water");
    }

    #[test]
    fn the_window_is_the_reach_with_margin() {
        assert!(!in_window(-0.1), "the floor above the feet is no floor");
        assert!(in_window(0.0));
        assert!(in_window(TRIGGER_DISTANCE), "the window edge saves");
        assert!(!in_window(TRIGGER_DISTANCE + 0.01), "past the edge is late");
    }

    #[test]
    fn the_floor_distance_measures_feet_to_hit() {
        assert!((floor_distance(64.0, 60.0) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn the_constants_hold_the_reach_story() {
        // Survival's use reach from the eyes is 5.5; the eyes sit 1.62 above
        // the feet, so the window must leave margin for the fall in flight.
        assert!(TRIGGER_DISTANCE - 1.62 < 5.5);
        assert!(FLOOR_DEPTH > TRIGGER_DISTANCE);
        assert!(MIN_FALL >= 2.0, "below this the game's own landing hurts nothing");
    }

    #[test]
    fn a_placed_save_times_out_and_settles_before_the_pickup() {
        assert!(PLACED_TIMEOUT > 60, "a glide in placed water must not give up early");
        assert!(PICKUP_DELAY >= 1, "the pickup waits out the landing tick");
    }

    #[test]
    fn a_placed_save_picks_up_only_after_a_settled_landing() {
        // Still falling: keep waiting, however long.
        assert!(matches!(placed_step(10, false, false, false), PlacedDecision::Wait));
        assert!(matches!(placed_step(10, false, false, true), PlacedDecision::Wait));
        // Landed but too fresh for the pickup: let it settle — emptied or
        // not, the confirmation may still be a latency wide.
        assert!(matches!(placed_step(0, true, false, true), PlacedDecision::Wait));
        assert!(matches!(placed_step(0, true, false, false), PlacedDecision::Wait));
        // Landed, settled, the bucket came home empty: pick the water up.
        assert!(matches!(
            placed_step(PICKUP_DELAY - 1, true, false, true),
            PlacedDecision::PickUp
        ));
        // Landed, settled, but the water never placed: hand the slot back.
        assert!(matches!(
            placed_step(PICKUP_DELAY - 1, true, false, false),
            PlacedDecision::GiveUp
        ));
        // Landed in the placed water: the same pickup, the water counts.
        assert!(matches!(
            placed_step(PICKUP_DELAY - 1, false, true, true),
            PlacedDecision::PickUp
        ));
        // A glide that never lands gives the slot back in the end.
        assert!(matches!(
            placed_step(PLACED_TIMEOUT - 1, false, false, false),
            PlacedDecision::GiveUp
        ));
    }

    #[test]
    fn a_pickup_wait_fills_retries_then_gives_up() {
        // The bucket came home full: done, whatever the tick.
        assert!(matches!(picked_step(0, 0, true), PickedDecision::GiveUp));
        // Still waiting for the server's confirmation.
        assert!(matches!(picked_step(0, 0, false), PickedDecision::Wait));
        // The wait ran out with tries left: another click goes out.
        assert!(matches!(
            picked_step(PICKUP_DELAY - 1, 0, false),
            PickedDecision::Retry
        ));
        // The last try spent: done, filled or not.
        assert!(matches!(
            picked_step(PICKUP_DELAY - 1, USE_TRIES - 1, false),
            PickedDecision::GiveUp
        ));
    }

    #[test]
    fn the_state_round_trips() {
        assert_eq!(get(), Mode::Idle, "the machine starts idle");
        set(Mode::Selecting { slot: 3, saved: 1 });
        assert_eq!(get(), Mode::Selecting { slot: 3, saved: 1 });
        reset();
        assert_eq!(get(), Mode::Idle);
    }
}
