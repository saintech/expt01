use crate::cmtp::PlayerAction;
use crate::engine::game;

pub fn update(world: &mut game::World) {
    if let PlayerAction::LookAt(lx, ly) = world.player.action {
        let ids: Vec<_> = world
            .map_obj_iter()
            .filter(|(_, sym, map_obj, _, cell)| {
                ((sym.x, sym.y) == (lx, ly)) && !map_obj.hidden && cell.in_fov
            })
            .map(|(id, ..)| id)
            .collect();
        for i in 0..world.player.look_at.len() {
            world.player.look_at[i] = ids.get(i).copied();
        }
    }
}
