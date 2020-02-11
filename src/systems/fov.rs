use crate::cfg;
use crate::game;

pub fn update(world: &mut game::World, tcod: &mut game::Tcod) {
    let map_is_empty = world.map.len() == 0;
    let fov_is_empty = tcod.fov.size() == (1, 1);
    match (map_is_empty, fov_is_empty) {
        (true, false) => tcod.fov = tcod::map::Map::new(1, 1),
        (false, true) => {
            create_fov(world, tcod);
            compute_fov(world, tcod);
        }
        (false, false) => compute_fov(world, tcod),
        _ => (),
    }
}

/// create the FOV map, according to the generated map
fn create_fov(world: &mut game::World, tcod: &mut game::Tcod) {
    tcod.fov = tcod::map::Map::new(cfg::MAP_WIDTH, cfg::MAP_HEIGHT);
    for y in 0..cfg::MAP_HEIGHT {
        for x in 0..cfg::MAP_WIDTH {
            let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
            tcod.fov.set(
                x,
                y,
                !world.map[index_in_map].block_sight,
                !world.map[index_in_map].block,
            );
        }
    }
}

fn compute_fov(world: &mut game::World, tcod: &mut game::Tcod) {
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player_symbol = &world.symbols[player_indexes.symbol.unwrap()];
    if world.player.previous_player_position != (player_symbol.x, player_symbol.y) {
        tcod.fov.compute_fov(
            player_symbol.x,
            player_symbol.y,
            cfg::TORCH_RADIUS,
            cfg::FOV_LIGHT_WALLS,
            cfg::FOV_ALGO,
        );
        for y in 0..cfg::MAP_HEIGHT {
            for x in 0..cfg::MAP_WIDTH {
                let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
                let in_fov = tcod.fov.is_in_fov(x, y);
                world.map[index_in_map].in_fov = in_fov;
                if in_fov {
                    world.map[index_in_map].explored = true;
                }
            }
        }
        world.player.previous_player_position = (player_symbol.x, player_symbol.y);
    }
}
