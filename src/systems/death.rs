use crate::cfg;
use crate::cmtp::{DeathCallback, PlayerState};
use crate::game;

pub fn update(world: &mut game::World) {
    if world.player.state != PlayerState::MakingTurn {
        return;
    }
    let callbacks = world
        .entity_indexes
        .iter()
        .filter_map(|(&id, indexes)| {
            indexes
                .character
                .filter(|&ch| {
                    !world.characters[ch].alive
                        && (world.characters[ch].on_death != DeathCallback::None)
                })
                .map(|ch| (id, world.characters[ch].on_death))
        })
        .collect::<Vec<_>>();
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
    game::add_log(world, "You died!", cfg::COLOR_DARK_RED);
    // for added effect, transform the player into a corpse!
    let indexes = &world.entity_indexes[&world.player.id];
    let symbol = &mut world.symbols[indexes.symbol.unwrap()];
    symbol.char = '\u{A3}';
    symbol.color = cfg::COLOR_DARK_RED;
    let player = &mut world.characters[indexes.character.unwrap()];
    player.on_death = DeathCallback::None;
}

fn monster_death(monster_id: u32, world: &mut game::World) {
    let indexes = &world.entity_indexes[&monster_id];
    let name = world.map_objects[indexes.map_object.unwrap()].name.clone();
    let xp = world.characters[indexes.character.unwrap()].xp;
    // transform it into a nasty corpse! it doesn't block, can't be
    // attacked and doesn't move
    game::add_log(
        world,
        format!("{} is dead! You gain {} experience points.", name, xp),
        cfg::COLOR_ORANGE,
    );
    let indexes = world.entity_indexes.get_mut(&monster_id).unwrap();
    let symbol = &mut world.symbols[indexes.symbol.unwrap()];
    let map_object = &mut world.map_objects[indexes.map_object.unwrap()];
    symbol.char = '\u{A3}';
    symbol.color = cfg::COLOR_DARK_RED;
    map_object.block = false;
    indexes.character = None;
    map_object.name = format!("remains of {}", map_object.name);
}
