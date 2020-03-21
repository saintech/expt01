use crate::cfg;
use crate::cmtp::{Item, ItemKind, PlayerAction, PlayerState, Slot};
use crate::engine;
use crate::engine::game;

pub fn update(world: &mut game::World) {
    if (world.player.state != PlayerState::MakingTurn) || !world.player_is_alive() {
        return;
    }

    let (dx, dy) = {
        use PlayerAction::*;
        match world.player.action {
            GoToUp => (0, -1),
            GoToDown => (0, 1),
            GoToLeft => (-1, 0),
            GoToRight => (1, 0),
            GoToUpLeft => (-1, -1),
            GoToUpRight => (1, -1),
            GoToDownLeft => (-1, 1),
            GoToDownRight => (1, 1),
            LookAt(x, y) if cell_in_fov(world, x, y) => {
                world.player.looking_at = Some((x, y));
                return;
            }
            ClickAt(x, y) if cell_in_fov(world, x, y) => {
                world.player.action = SkipTurn;
                delta_by_click(world, x, y)
            }
            _ => return,
        }
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
        .find(|&(id, sym, ..)| ((sym.x, sym.y) == new_pos) && (id != world.player.id))
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

fn cell_in_fov(world: &game::World, x: i32, y: i32) -> bool {
    if (x >= cfg::MAP_WIDTH) || (y >= cfg::MAP_HEIGHT) {
        return false;
    }
    let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
    world.map[index_in_map].in_fov
}

fn is_ranged_weapon(item: &&Item) -> bool {
    match item.kind {
        ItemKind::Ranged(_) => true,
        _ => false,
    }
}

fn delta_by_click(world: &game::World, x: i32, y: i32) -> (i32, i32) {
    let (dx, dy) = (x - world.player_sym().x, y - world.player_sym().y);
    let player_has_ranged = world
        .get_equipped_in_slot(Slot::Hands)
        .map(|id| world.get_item(id).unwrap().2)
        .filter(is_ranged_weapon)
        .is_some();
    if player_has_ranged {
        (dx, dy)
    } else {
        (dx.signum(), dy.signum())
    }
}
