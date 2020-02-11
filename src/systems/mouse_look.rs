use crate::cfg;
use crate::cmtp::{MapCell, PlayerAction, Symbol};
use crate::game;

pub fn update(world: &mut game::World) {
    if let PlayerAction::LookAt(lx, ly) = world.player.action {
        let ids: Vec<_> = world
            .entity_indexes
            .iter()
            .filter(|&(_, indexes)| {
                if let (Some(s), Some(mo)) = (indexes.symbol, indexes.map_object) {
                    let &Symbol { x, y, .. } = &world.symbols[s];
                    let &MapCell { in_fov, .. } = &world.map[(y * cfg::MAP_WIDTH + x) as usize];
                    ((x, y) == (lx, ly)) && !world.map_objects[mo].hidden && in_fov
                } else {
                    false
                }
            })
            .map(|(&id, _)| id)
            .collect();
        for i in 0..world.player.look_at.len() {
            world.player.look_at[i] = ids.get(i).copied();
        }
    }
}
