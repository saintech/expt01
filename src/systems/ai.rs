use crate::cfg;
use crate::cmtp::{Ai, PlayerAction, PlayerState, Symbol};
use crate::game;
use rand::Rng as _;

pub fn update(world: &mut game::World) {
    if world.player.state != PlayerState::MakingTurn {
        return;
    }
    // let monsters take their turn
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player = &world.characters[player_indexes.character.unwrap()];
    if player.alive && player_action_is_turn(world.player.action) {
        let ai_ids: Vec<_> = world
            .entity_indexes
            .iter()
            .filter_map(|(&id, indexes)| indexes.character.and(indexes.ai.and(Some(id))))
            .collect();
        for id in ai_ids {
            ai_take_turn(id, world);
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

fn ai_take_turn(id: u32, world: &mut game::World) {
    let ai_index = world.entity_indexes[&id].ai.unwrap();
    if let Some(ai) = world.ais[ai_index].option.take() {
        let new_ai = match ai {
            Ai::Basic => ai_basic(id, world),
            Ai::Confused {
                previous_ai,
                num_turns,
            } => ai_confused(id, world, previous_ai, num_turns),
        };
        world.ais[ai_index].option = Some(new_ai);
    }
}

fn ai_basic(monster_id: u32, world: &mut game::World) -> Ai {
    let monster_indexes = &world.entity_indexes[&monster_id];
    let monster_symbol = &world.symbols[monster_indexes.symbol.unwrap()];
    let (monster_x, monster_y) = (monster_symbol.x, monster_symbol.y);
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player_hp = world.characters[player_indexes.character.unwrap()].hp;
    let player_symbol = &world.symbols[player_indexes.symbol.unwrap()];
    let (player_x, player_y) = (player_symbol.x, player_symbol.y);
    if (monster_x > player_x) || ((monster_x == player_x) && (monster_y < player_y)) {
        world.characters[monster_indexes.character.unwrap()].looking_right = false;
    } else {
        world.characters[monster_indexes.character.unwrap()].looking_right = true;
    }
    let is_in_fov = world.map[(monster_y * cfg::MAP_WIDTH + monster_x) as usize].in_fov;
    if is_in_fov {
        if game::distance_to(monster_x, monster_y, player_x, player_y) >= 2.0 {
            // move towards player if far away
            move_towards(monster_id, player_x, player_y, world);
        } else if player_hp > 0 {
            // close enough, attack! (if the player is still alive.)
            game::attack_by(monster_id, world.player.id, world);
        }
    }
    Ai::Basic
}

fn move_towards(id: u32, target_x: i32, target_y: i32, world: &mut game::World) {
    let object_indexes = &world.entity_indexes[&id];
    let &Symbol { x, y, .. } = &world.symbols[object_indexes.symbol.unwrap()];
    // vector from this object to the target, and distance
    let dx = target_x - x;
    let dy = target_y - y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();
    // normalize it to length 1 (preserving direction), then round it and
    // convert to integer so the movement is restricted to the map grid
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    game::move_by(id, dx, dy, world);
}

fn ai_confused(
    monster_id: u32,
    world: &mut game::World,
    previous_ai: Box<Ai>,
    num_turns: i32,
) -> Ai {
    let monster_indexes = &world.entity_indexes[&monster_id];
    let monster_name = world.map_objects[monster_indexes.map_object.unwrap()]
        .name
        .clone();
    if num_turns >= 0 {
        // still confused ...
        // move in a random direction, and decrease the number of turns confused
        game::move_by(
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
        game::add_log(
            world,
            format!("The {} is no longer confused!", monster_name),
            cfg::COLOR_ORANGE,
        );
        *previous_ai
    }
}
