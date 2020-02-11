use crate::cmtp::{PlayerAction, PlayerState};
use crate::game;

pub fn update(world: &mut game::World) {
    if world.player.state != PlayerState::MakingTurn {
        return;
    }
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player_character = &world.characters[player_indexes.character.unwrap()];
    if !player_character.alive {
        return;
    }
    let (dx, dy) = match world.player.action {
        PlayerAction::GoToUp => (0, -1),
        PlayerAction::GoToDown => (0, 1),
        PlayerAction::GoToLeft => (-1, 0),
        PlayerAction::GoToRight => (1, 0),
        PlayerAction::GoToUpLeft => (-1, -1),
        PlayerAction::GoToUpRight => (1, -1),
        PlayerAction::GoToDownLeft => (-1, 1),
        PlayerAction::GoToDownRight => (1, 1),
        _ => return,
    };
    // the coordinates the player is moving to/attacking
    let x = world.symbols[player_indexes.symbol.unwrap()].x + dx;
    let y = world.symbols[player_indexes.symbol.unwrap()].y + dy;
    if (dy > 0) || ((dy == 0) && (dx < 0)) {
        world.characters[player_indexes.character.unwrap()].looking_right = false;
    } else if (dy < 0) || ((dy == 0) && (dx > 0)) {
        world.characters[player_indexes.character.unwrap()].looking_right = true;
    }
    // try to find an attackable object there
    let target_id = world.entity_indexes.iter().find_map(|(&id, indexes)| {
        if let (Some(_), Some(sy)) = (indexes.character, indexes.symbol) {
            if (world.symbols[sy].x, world.symbols[sy].y) == (x, y) {
                Some(id)
            } else {
                None
            }
        } else {
            None
        }
    });
    // attack if target found, move otherwise
    match target_id {
        Some(target_id) => {
            game::attack_by(world.player.id, target_id, world);
        }
        None => {
            game::move_by(world.player.id, dx, dy, world);
        }
    }
}
