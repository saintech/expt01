use crate::cfg;
use crate::engine::game;
use tcod::map;

pub fn update(world: &mut game::World, fov: &mut map::Map) {
    let map_is_empty = world.map.len() == 0;
    let fov_is_empty = fov.size() == (1, 1);
    match (map_is_empty, fov_is_empty) {
        (true, false) => *fov = map::Map::new(1, 1),
        (false, true) => {
            create_fov(world, fov);
            compute_fov(world, fov);
        }
        (false, false) => compute_fov(world, fov),
        _ => (),
    }
}

/// create the FOV map, according to the generated map
fn create_fov(world: &mut game::World, fov: &mut map::Map) {
    *fov = map::Map::new(cfg::MAP_WIDTH, cfg::MAP_HEIGHT);
    for y in 0..cfg::MAP_HEIGHT {
        for x in 0..cfg::MAP_WIDTH {
            let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
            fov.set(
                x,
                y,
                !world.map[index_in_map].block_sight,
                !world.map[index_in_map].block,
            );
        }
    }
}

fn compute_fov(world: &mut game::World, fov: &mut map::Map) {
    let player_symbol = world.player_sym();
    let (player_x, player_y) = (player_symbol.x, player_symbol.y);
    if world.player.previous_player_position != (player_x, player_y) {
        fov.compute_fov(
            player_x,
            player_y,
            cfg::TORCH_RADIUS,
            cfg::FOV_LIGHT_WALLS,
            cfg::FOV_ALGO,
        );
        for y in 0..cfg::MAP_HEIGHT {
            for x in 0..cfg::MAP_WIDTH {
                let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
                let in_fov = fov.is_in_fov(x, y);
                world.map[index_in_map].in_fov = in_fov;
                if in_fov {
                    world.map[index_in_map].explored = true;
                }
            }
        }
        world.player.previous_player_position = (player_x, player_y);
    }
}
