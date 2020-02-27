use crate::cfg;
use crate::cmtp::{DeathCallback, PlayerState};
use crate::engine::game;

pub fn update(world: &mut game::World) {
    if world.player.state != PlayerState::MakingTurn {
        return;
    }
    let callbacks: Vec<_> = world
        .character_iter()
        .filter(|(.., char, _)| !char.alive && (char.on_death != DeathCallback::None))
        .map(|(id, .., char, _)| (id, char.on_death))
        .collect();
    for (id, callback) in callbacks {
        use DeathCallback::*;
        let callback: fn(u32, &mut game::World) = match callback {
            Player => player_death,
            Monster => monster_death,
            None => unreachable!(),
        };
        callback(id, world);
    }
}

fn player_death(_id: u32, world: &mut game::World) {
    // the game ended!
    world.add_log(cfg::COLOR_DARK_RED, "You died!");
    // for added effect, transform the player into a corpse!
    let (symbol, _, char, _) = world.get_character_mut(world.player.id).unwrap();
    symbol.glyph = '\u{A3}';
    symbol.color = cfg::COLOR_DARK_RED;
    char.on_death = DeathCallback::None;
}

fn monster_death(monster_id: u32, world: &mut game::World) {
    let name = world.get_character(monster_id).unwrap().1.name.clone();
    let xp = world.get_character(monster_id).unwrap().2.xp;
    // transform it into a nasty corpse! it doesn't block, can't be
    // attacked and doesn't move
    world.add_log(
        cfg::COLOR_ORANGE,
        format!("{} is dead! You gain {} experience points.", name, xp),
    );
    let (symbol, map_obj, ..) = world.get_character_mut(monster_id).unwrap();
    symbol.glyph = '\u{A3}';
    symbol.color = cfg::COLOR_DARK_RED;
    map_obj.block = false;
    map_obj.name = format!("remains of {}", map_obj.name);
    world.entity_indexes.get_mut(&monster_id).unwrap().character = None;
}
