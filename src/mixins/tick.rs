// Every game tick: run the save's state machine — choose the bucket slot,
// select it a tick ahead (the carried-slot packet must lead the use on the
// wire: a server acts with the slot it knows), use it at the floor the feet'
// ray finds, and fill the bucket back up from the placed water once the
// landing settled.
//
// Any screen over the world stops the machine: a use from behind a dialog is
// a click on the dialog, and a fall is nobody's concern while it stands.
//
// The use is the game's own click pipeline with a forced target
// (`forced_hit`): on a world this client only watches, the player's own
// remote-use press (the packet carries the hit and the server acts with the
// slot it knows); on a hosted world, the item's own use with the guest's
// item shape — a guest's use names one hand and touches no entity, so an
// entity under the crosshair cannot take the click.
// Copyright (c) 2026 hakergeniusz. MIT.
fn mlg_tick(game: &mut crate::game::Game<'_>) {
    // One block use at `hit` with the main hand, through the game's own
    // click pipeline at the forced target. Whether the click took, where the
    // pipeline answers at all.
    fn use_on(game: &mut crate::game::Game<'_>, hit: crate::game::Hit) -> bool {
        let saved_hit = game.forced_hit;
        let took;
        if game.remote.is_some() {
            game.forced_hit = Some(hit);
            game.remote_use(mod_api::InteractionAttempt::Press);
            took = true;
        } else {
            game.forced_hit = Some(hit);
            let saved_guest = game.guest_use;
            game.guest_use =
                Some(crate::game::GuestUse::Item(horizon_client::first_person::Hand::Main));
            took = game.use_hand(None).unwrap_or(false);
            game.guest_use = saved_guest;
        }
        game.forced_hit = saved_hit;
        took
    }

    // A book, a sign editor, or a prompt over the world: the clicks of the
    // tick belong to the screen, not to a fall.
    if game.screen != crate::game::Screen::Playing
        || game.book.is_some()
        || game.sign_edit.is_some()
        || game.link_prompt.is_some()
        || game.command_prompt.is_some()
        || game.code_of_conduct.is_some()
    {
        mlg::reset();
        return;
    }
    let Some(p) = game.player.as_ref() else {
        mlg::reset();
        return;
    };
    let on_ground = p.ground.on_ground;
    let sink = p.velocity.y;
    let fall = p.fall_distance;
    let in_water = p.fluids.was_touching_water;
    let feet = [p.position.x, p.position.y, p.position.z];

    match mlg::get() {
        mlg::Mode::Idle => {
            // The player's own held use wins: a bite or a bow in flight is
            // what they chose, and a use inside a use is not the game's way.
            if game.spectator()
                || game.right
                || !mlg::wants_save(on_ground, sink, fall, in_water)
            {
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
            mlg::set(mlg::Mode::Selecting { slot, saved: game.selected });
        }
        mlg::Mode::Selecting { slot, saved } => {
            // The carried-slot packet goes out with this tick's input
            // handling; the use waits for the next.
            game.selected = slot;
            mlg::set(mlg::Mode::Placing { slot, saved, tries: 0 });
        }
        mlg::Mode::Placing { slot, saved, tries } => {
            if on_ground || in_water {
                // The ground arrived before the bucket did: nothing to save.
                mlg::set(mlg::Mode::Done { saved });
                return;
            }
            let Some(stack) = game.inventory[slot].as_ref() else {
                mlg::set(mlg::Mode::Done { saved });
                return;
            };
            if stack.item.as_ref() != "minecraft:water_bucket" {
                mlg::set(mlg::Mode::Done { saved });
                return;
            }
            // A fresh floor every attempt: the fall moved since the last.
            let Some(hit) = game.clip_ray(feet, [0.0, -1.0, 0.0], mlg::FLOOR_DEPTH) else {
                mlg::set(mlg::Mode::Done { saved });
                return;
            };
            if !mlg::in_window(mlg::floor_distance(feet[1], hit.point[1])) {
                mlg::set(mlg::Mode::Done { saved });
                return;
            }
            let water = [
                hit.pos[0] + hit.face[0],
                hit.pos[1] + hit.face[1],
                hit.pos[2] + hit.face[2],
            ];
            if !use_on(game, hit) && tries + 1 < mlg::USE_TRIES {
                // A refused click, on a host where the answer is the
                // pipeline's own: try again next tick.
                mlg::set(mlg::Mode::Placing { slot, saved, tries: tries + 1 });
                return;
            }
            mlg::set(mlg::Mode::Placed { slot, saved, ticks: 0, water });
        }
        mlg::Mode::Placed { slot, saved, ticks, water } => {
            let emptied = game.inventory[slot]
                .as_ref()
                .is_some_and(|s| s.item.as_ref() == "minecraft:bucket");
            match mlg::placed_step(ticks, on_ground, in_water, emptied) {
                mlg::PlacedDecision::GiveUp => mlg::set(mlg::Mode::Done { saved }),
                mlg::PlacedDecision::PickUp => {
                    mlg::set(mlg::Mode::Picking { slot, saved, tries: 0, water })
                }
                mlg::PlacedDecision::Wait => {
                    mlg::set(mlg::Mode::Placed { slot, saved, ticks: ticks + 1, water })
                }
            }
        }
        mlg::Mode::Picking { slot, saved, tries, water } => {
            // The click targets the water itself: an empty bucket fills from
            // the source block the hit names.
            let hit = crate::game::Hit {
                pos: water,
                face: [0, 1, 0],
                point: [
                    f64::from(water[0]) + 0.5,
                    f64::from(water[1]) + 1.0,
                    f64::from(water[2]) + 0.5,
                ],
                inside: false,
            };
            use_on(game, hit);
            mlg::set(mlg::Mode::Picked { slot, saved, tries, water, ticks: 0 });
        }
        mlg::Mode::Picked { slot, saved, tries, water, ticks } => {
            let filled = game.inventory[slot]
                .as_ref()
                .is_some_and(|s| s.item.as_ref() == "minecraft:water_bucket");
            match mlg::picked_step(ticks, tries, filled) {
                mlg::PickedDecision::GiveUp => mlg::set(mlg::Mode::Done { saved }),
                mlg::PickedDecision::Retry => {
                    mlg::set(mlg::Mode::Picking { slot, saved, tries: tries + 1, water })
                }
                mlg::PickedDecision::Wait => mlg::set(mlg::Mode::Picked {
                    slot,
                    saved,
                    tries,
                    water,
                    ticks: ticks + 1,
                }),
            }
        }
        mlg::Mode::Done { saved } => {
            game.selected = saved;
            mlg::reset();
        }
    }
}
