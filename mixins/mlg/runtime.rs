// Auto-MLG's brain: pure standard library, no game types, so it compiles and
// tests wherever — as this crate's library and, injected, as a module of the
// game. A fall only hurts from the height built since the last touch of
// water, and a water bucket poured into the block above the floor resets it
// where the game's own rules run — the server's sim, on a world that has no
// mods at all. The bucket is a real use of a real item: the server sees a
// legal placement inside its reach check, not a spoofed number, so the save
// holds on any server this mod is installed on.
//
// The save is latency-bound: the water must exist before the feet arrive,
// and a use only reaches so far ahead (about six blocks below the feet on a
// server's reach check). Low-latency worlds (a LAN, a nearby dedicated
// server) save most falls; a distant server saves the slower ones.
//
// Clean-room: written for Horizon from the game's own public behavior; it
// copies nothing from any mod.
//
// Copyright (c) 2026 hakergeniusz. MIT.

/// The mod's registry id: the Options > Mods row and the injection gates.
pub const ID: &str = "mlg";

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

/// How far ahead of the feet a floor still counts: past it the hit is the
/// block the feet are in already.
pub const FLOOR_SLACK: f64 = 0.5;

/// The reach of the feet' downward floor search, a little past the trigger
/// so the window's edge is a real measurement, not the ray's cutoff.
pub const FLOOR_DEPTH: f64 = TRIGGER_DISTANCE + 2.0;

/// Placement attempts before the fall is given up to the ground.
pub const USE_TRIES: u32 = 3;

/// Ticks a placed save may wait for the landing before the state gives up
/// and hands the slot back: a long glide in placed water, a teleport.
pub const PLACED_TIMEOUT: u32 = 300;

/// Ticks between the landing and the pickup use: let the landing settle, so
/// the water click is not the same tick the feet stop.
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
        assert!(
            !wants_save(false, -0.1, 5.0, false),
            "settling, not sinking"
        );
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
        let trigger = TRIGGER_DISTANCE;
        let depth = FLOOR_DEPTH;
        let min_fall = MIN_FALL;
        assert!(trigger - 1.62 < 5.5);
        assert!(depth > trigger);
        assert!(
            min_fall >= 2.0,
            "below this the game's own landing hurts nothing"
        );
    }

    #[test]
    fn a_placed_save_times_out_and_settles_before_the_pickup() {
        let timeout = PLACED_TIMEOUT;
        let delay = PICKUP_DELAY;
        assert!(
            timeout > 60,
            "a glide in placed water must not give up early"
        );
        assert!(delay >= 1, "the pickup waits out the landing tick");
    }
}
