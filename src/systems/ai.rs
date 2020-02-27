use crate::cfg;
use crate::cmtp::{Ai, PlayerAction, PlayerState, Symbol};
use crate::engine;
use crate::engine::game;
use rand::Rng as _;

pub fn update(world: &mut game::World) {
    if world.player.state != PlayerState::MakingTurn {
        return;
    }
    // let monsters take their turn
    if world.player_is_alive() && player_action_is_turn(world.player.action) {
        let ai_ids: Vec<_> = world
            .character_iter()
            .filter(|(.., ai)| ai.option.is_some())
            .map(|(id, ..)| id)
            .collect();
        for id in ai_ids {
            let ai_container = world.get_character_mut(id).unwrap().3;
            let ai = ai_container.option.take().unwrap();
            let new_ai = match ai {
                Ai::Basic => ai_basic(id, world),
                Ai::Confused {
                    previous_ai,
                    num_turns,
                } => ai_confused(id, world, previous_ai, num_turns),
            };
            let ai_container = world.get_character_mut(id).unwrap().3;
            ai_container.option.replace(new_ai);
        }
    }
}

fn player_action_is_turn(action: PlayerAction) -> bool {
    use PlayerAction::*;
    return match action {
        GoToUp | GoToDown | GoToLeft | GoToRight | GoToUpLeft | GoToUpRight | GoToDownLeft
        | GoToDownRight | SkipTurn => true,
        _ => false,
    };
}

fn ai_basic(monster_id: u32, world: &mut game::World) -> Ai {
    let monster_symbol = world.get_character(monster_id).unwrap().0;
    let (monster_x, monster_y) = (monster_symbol.x, monster_symbol.y);
    let player_symbol = world.player_sym();
    let (player_x, player_y) = (player_symbol.x, player_symbol.y);
    if (monster_x > player_x) || ((monster_x == player_x) && (monster_y < player_y)) {
        world.get_character_mut(monster_id).unwrap().2.looking_right = false;
    } else {
        world.get_character_mut(monster_id).unwrap().2.looking_right = true;
    }
    if world.check_fov(monster_id) {
        if game::World::distance_to(monster_x, monster_y, player_x, player_y) >= 2.0 {
            // move towards player if far away
            move_towards(monster_id, player_x, player_y, world);
        } else if world.player_char().hp > 0 {
            // close enough, attack! (if the player is still alive.)
            engine::attack_by(monster_id, world.player.id, world);
        }
    }
    Ai::Basic
}

fn move_towards(id: u32, target_x: i32, target_y: i32, world: &mut game::World) {
    let &Symbol { x, y, .. } = world.get_character(id).unwrap().0;
    // vector from this object to the target, and distance
    let dx = target_x - x;
    let dy = target_y - y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();
    // normalize it to length 1 (preserving direction), then round it and
    // convert to integer so the movement is restricted to the map grid
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    engine::move_by(id, dx, dy, world);
}

fn ai_confused(
    monster_id: u32,
    world: &mut game::World,
    previous_ai: Box<Ai>,
    num_turns: i32,
) -> Ai {
    let monster_name = world.get_character(monster_id).unwrap().1.name.clone();
    if num_turns >= 0 {
        // still confused ...
        // move in a random direction, and decrease the number of turns confused
        engine::move_by(
            monster_id,
            rand::thread_rng().gen_range(-1, 2),
            rand::thread_rng().gen_range(-1, 2),
            world,
        );
        Ai::Confused {
            previous_ai: previous_ai,
            num_turns: num_turns - 1,
        }
    } else {
        // restore the previous AI (this one will be deleted)
        world.add_log(
            cfg::COLOR_ORANGE,
            format!("The {} is no longer confused!", monster_name),
        );
        *previous_ai
    }
}
