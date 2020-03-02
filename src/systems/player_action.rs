use crate::cfg;
use crate::cmtp::{PlayerAction, PlayerState};
use crate::engine;
use crate::engine::game;

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
        PlayerAction::LookAt(x, y) => {
            world.player.looking_at = check_cell_in_fov(world, x, y);
            return;
        }
        _ => return,
    };
    // the coordinates the player is moving to/attacking
    let player_symbol = world.player_sym();
    let new_pos = (player_symbol.x + dx, player_symbol.y + dy);
    if (dy > 0) || ((dy == 0) && (dx < 0)) {
        world.player_char_mut().looking_right = false;
    } else if (dy < 0) || ((dy == 0) && (dx > 0)) {
        world.player_char_mut().looking_right = true;
    }
    // try to find an attackable object there
    let target_id = world
        .character_iter()
        .find(|(_, sym, ..)| ((sym.x, sym.y) == new_pos))
        .map(|(id, ..)| id);
    // attack if target found, move otherwise
    match target_id {
        Some(target_id) => {
            engine::attack_by(world.player.id, target_id, world);
        }
        None => {
            engine::move_by(world.player.id, dx, dy, world);
        }
    }
}

fn check_cell_in_fov(world: &game::World, x: i32, y: i32) -> Option<(i32, i32)> {
    if (x >= cfg::MAP_WIDTH) || (y >= cfg::MAP_HEIGHT) {
        return None;
    }
    let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
    let cell = &world.map[index_in_map];
    if cell.in_fov {
        Some((x, y))
    } else {
        None
    }
}
