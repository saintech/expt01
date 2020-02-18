use crate::cmtp::{PlayerAction, PlayerState};
use crate::game;

pub fn update(world: &mut game::World) {
    if (world.player.state != PlayerState::MakingTurn) || !world.player_is_alive() {
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
    let player_symbol = world.get_character(world.player.id).unwrap().0;
    let new_pos = (player_symbol.x + dx, player_symbol.y + dy);
    let player = world.get_character_mut(world.player.id).unwrap().2;
    if (dy > 0) || ((dy == 0) && (dx < 0)) {
        player.looking_right = false;
    } else if (dy < 0) || ((dy == 0) && (dx > 0)) {
        player.looking_right = true;
    }
    // try to find an attackable object there
    let target_id = world
        .character_iter()
        .find(|(_, sym, ..)| ((sym.x, sym.y) == new_pos))
        .map(|(id, ..)| id);
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
