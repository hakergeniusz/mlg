// Every game tick, on the window's game only: run the save's state machine —
// choose the bucket slot, select it a tick ahead (the carried-slot packet
// must lead the use on the wire: a server acts with the slot it knows),
// use it at the floor the feet' ray finds, and fill the bucket back up from
// the placed water once the landing settled.
//
// The use is the game's own pipeline with a forced target (`forced_hit`)
// and the guest's block-use shape (`GuestUse::OnBlock`), so an entity under
// the crosshair cannot take the click and the packet carries the real hit.
// Copyright (c) 2026 hakergeniusz. MIT.

// Every game tick: run the save's state machine.
fn mlg_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::mlg as mlg;
    use crate::game::mlg_state as state;

    if !game.native_on(mlg::ID) || !game.native_window() {
        return;
    }
    // Any screen over the world stops the machine: a use from behind a
    // dialog is a click on the dialog, and a fall is nobody's concern while
    // it stands.
    if game.screen != crate::game::Screen::Playing || game.dialog_open() {
        state::reset();
        return;
    }
    let Some(p) = game.player.as_ref() else {
        state::reset();
        return;
    };
    let on_ground = p.ground.on_ground;
    let sink = p.velocity.y;
    let fall = p.fall_distance;
    let in_water = p.fluids.was_touching_water;
    let feet = [p.position.x, p.position.y, p.position.z];

    match state::get() {
        state::Mode::Idle => {
            // The player's own held use wins: a bite or a bow in flight is
            // what they chose, and a use inside a use is not the game's way.
            if game.spectator() || game.right || !mlg::wants_save(on_ground, sink, fall, in_water) {
                return;
            }
            let Some(hit) = game.clip_ray(feet, [0.0, -1.0, 0.0], mlg::FLOOR_DEPTH) else {
                return;
            };
            if !mlg::in_window(mlg::floor_distance(feet[1], hit.point[1])) {
                return;
            }
            // The hotbar only: those are the slots a fall can reach without
            // opening anything.
            let slot = (0..=8).find(|&i| {
                game.inventory[i]
                    .as_ref()
                    .is_some_and(|s| s.item.as_ref() == "minecraft:water_bucket")
            });
            let Some(slot) = slot else {
                return;
            };
            state::set(state::Mode::Selecting { slot, saved: game.selected });
        }
        state::Mode::Selecting { slot, saved } => {
            // The carried-slot packet goes out with this tick's input
            // handling; the use waits for the next.
            game.selected = slot;
            state::set(state::Mode::Placing { slot, saved, tries: 0 });
        }
        state::Mode::Placing { slot, saved, tries } => {
            if on_ground || in_water {
                // The ground arrived before the bucket did: nothing to save.
                state::set(state::Mode::Done { saved });
                return;
            }
            let Some(stack) = game.inventory[slot].as_ref() else {
                state::set(state::Mode::Done { saved });
                return;
            };
            if stack.item.as_ref() != "minecraft:water_bucket" {
                state::set(state::Mode::Done { saved });
                return;
            }
            // A fresh floor every attempt: the fall moved since the last.
            let Some(hit) = game.clip_ray(feet, [0.0, -1.0, 0.0], mlg::FLOOR_DEPTH) else {
                state::set(state::Mode::Done { saved });
                return;
            };
            if !mlg::in_window(mlg::floor_distance(feet[1], hit.point[1])) {
                state::set(state::Mode::Done { saved });
                return;
            }
            let water = hit.beside();
            if state::use_on(game, hit) {
                state::set(state::Mode::Placed { slot, saved, ticks: 0, water });
            } else if tries + 1 < mlg::USE_TRIES {
                state::set(state::Mode::Placing { slot, saved, tries: tries + 1 });
            } else {
                state::set(state::Mode::Done { saved });
            }
        }
        state::Mode::Placed { slot, saved, ticks, water } => {
            let ticks = ticks + 1;
            if ticks >= mlg::PLACED_TIMEOUT {
                state::set(state::Mode::Done { saved });
            } else if on_ground || in_water {
                let emptied = game
                    .inventory[slot]
                    .as_ref()
                    .is_some_and(|s| s.item.as_ref() == "minecraft:bucket");
                if !emptied {
                    state::set(state::Mode::Done { saved });
                } else if ticks >= mlg::PICKUP_DELAY {
                    state::set(state::Mode::Picking { slot, saved, tries: 0, water });
                } else {
                    // Landed but too fresh for the pickup: let it settle.
                    state::set(state::Mode::Placed { slot, saved, ticks, water });
                }
            } else {
                // Still falling in placed water or past it.
                state::set(state::Mode::Placed { slot, saved, ticks, water });
            }
        }
        state::Mode::Picking { slot, saved, tries, water } => {
            // The click targets the water itself: an empty bucket fills
            // from the source block the hit names.
            let hit = crate::game::Hit {
                pos: water,
                face: [0, 1, 0],
                point: [
                    f64::from(water[0]) + 0.5,
                    f64::from(water[1]) + 1.0,
                    f64::from(water[2]) + 0.5,
                ],
            };
            if state::use_on(game, hit) || tries + 1 >= mlg::USE_TRIES {
                state::set(state::Mode::Done { saved });
            } else {
                state::set(state::Mode::Picking { slot, saved, tries: tries + 1, water });
            }
        }
        state::Mode::Done { saved } => {
            game.selected = saved;
            state::reset();
        }
    }
}
