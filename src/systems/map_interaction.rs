use crate::cfg;
use crate::cmtp::{
    AiOption, Character, Equipment, LogMessage, MapObject, OwnedItem, Player, PlayerAction,
    PlayerState, Symbol,
};
use crate::game;

pub fn update(world: &mut game::World) {
    if world.player.state != PlayerState::MakingTurn {
        return;
    }
    if (world.player.action != PlayerAction::InteractWithMap) || !world.player_is_alive() {
        return;
    }
    let player_symbol = world.get_character(world.player.id).unwrap().0;
    let player_pos = (player_symbol.x, player_symbol.y);
    // pick up an item or go to next level
    let item_id = world
        .item_iter()
        .find(|(_, sym, map_obj, ..)| ((sym.x, sym.y) == player_pos) && !map_obj.hidden)
        .map(|(id, ..)| id);
    let player_on_stairs = world
        .map_obj_iter()
        .any(|(_, sym, map_obj, ..)| ((sym.x, sym.y) == player_pos) && (map_obj.name == "stairs"));
    if let Some(item_id) = item_id {
        pick_item_up(item_id, world);
    } else if player_on_stairs {
        next_level(world);
    };
}

/// add to the player's inventory and remove from the map
fn pick_item_up(object_id: u32, world: &mut game::World) {
    let name = world.get_item(object_id).unwrap().1.name.clone();
    let inventory_len = world
        .items
        .iter()
        .filter(|&item| item.owner == world.player.id)
        .count();
    if inventory_len >= 35 {
        game::add_log(
            world,
            format!("Your inventory is full, cannot pick up {}.", name),
            cfg::COLOR_DARK_RED,
        );
    } else {
        game::add_log(
            world,
            format!("You picked up a {}!", name),
            cfg::COLOR_GREEN,
        );
        let player_id = world.player.id;
        let (_, map_obj, item, eqp) = world.get_item_mut(object_id).unwrap();
        item.owner = player_id;
        map_obj.hidden = true;
        // automatically equip, if the corresponding equipment slot is unused
        if let Some(&mut Equipment { slot, .. }) = eqp {
            if game::get_equipped_in_slot(slot, world).is_none() {
                game::equip(object_id, world);
            }
        }
    }
}

/// Advance to the next level
fn next_level(world: &mut game::World) {
    clear_dungeon(world);
    game::add_log(
        world,
        "You take a moment to rest, and recover your strength.",
        cfg::COLOR_GREEN,
    );
    let heal_hp = game::max_hp(world.player.id, world) / 2;
    game::heal(world.player.id, heal_hp, world);
    game::add_log(
        world,
        "After a rare moment of peace, you descend deeper into \
         the heart of the mine...",
        cfg::COLOR_ORANGE,
    );
    world.player.dungeon_level += 1;
}

fn clear_dungeon(world: &mut game::World) {
    // create new world for storing entities that should be saved
    let mut temp_world = game::World::default();
    temp_world.id_count = world.id_count;
    //copy player
    let player = &world.player;
    temp_world.player = Player {
        id: player.id,
        dungeon_level: player.dungeon_level,
        state: player.state,
        action: player.action,
        look_at: player.look_at,
        previous_player_position: player.previous_player_position,
    };
    // move player entity if exist
    if let Some(indexes) = world.entity_indexes.remove(&world.player.id) {
        let symbol = Symbol {
            x: world.symbols[indexes.symbol.unwrap()].x,
            y: world.symbols[indexes.symbol.unwrap()].y,
            glyph: world.symbols[indexes.symbol.unwrap()].glyph,
            color: world.symbols[indexes.symbol.unwrap()].color,
        };
        let map_object = MapObject {
            name: world.map_objects[indexes.map_object.unwrap()].name.clone(),
            block: world.map_objects[indexes.map_object.unwrap()].block,
            always_visible: world.map_objects[indexes.map_object.unwrap()].always_visible,
            hidden: world.map_objects[indexes.map_object.unwrap()].hidden,
        };
        let character = Character {
            alive: world.characters[indexes.character.unwrap()].alive,
            level: world.characters[indexes.character.unwrap()].level,
            hp: world.characters[indexes.character.unwrap()].hp,
            base_max_hp: world.characters[indexes.character.unwrap()].base_max_hp,
            base_defense: world.characters[indexes.character.unwrap()].base_defense,
            base_power: world.characters[indexes.character.unwrap()].base_power,
            xp: world.characters[indexes.character.unwrap()].xp,
            on_death: world.characters[indexes.character.unwrap()].on_death,
            looking_right: world.characters[indexes.character.unwrap()].looking_right,
        };
        temp_world.player.id = temp_world.create_entity(
            Some(symbol),
            None,
            Some(map_object),
            Some(character),
            Some(AiOption { option: None }),
            None,
            None,
            None,
            None,
        );
    }
    // copy inventory
    let inventory = world
        .item_iter()
        .filter(|(.., item, _)| item.owner == world.player.id);
    for (_, sym, map_obj, item, equipment) in inventory {
        let symbol = Symbol {
            x: sym.x,
            y: sym.y,
            glyph: sym.glyph,
            color: sym.color,
        };
        let map_object = MapObject {
            name: map_obj.name.clone(),
            block: map_obj.block,
            always_visible: map_obj.always_visible,
            hidden: map_obj.hidden,
        };
        let item = OwnedItem {
            item: item.item,
            owner: temp_world.player.id,
        };
        let equipment = equipment.map(|equipment| Equipment {
            slot: equipment.slot,
            equipped: equipment.equipped,
            max_hp_bonus: equipment.max_hp_bonus,
            defense_bonus: equipment.defense_bonus,
            power_bonus: equipment.power_bonus,
        });
        temp_world.create_entity(
            Some(symbol),
            None,
            Some(map_object),
            None,
            Some(AiOption { option: None }),
            Some(item),
            equipment,
            None,
            None,
        );
    }
    // copy logs
    let logs = world
        .entity_indexes
        .values()
        .filter(|indexes| indexes.log_message.is_some());
    for indexes in logs {
        let log_message = LogMessage(
            world.log[indexes.log_message.unwrap()].0.clone(),
            world.log[indexes.log_message.unwrap()].1,
        );
        temp_world.create_entity(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(log_message),
            None,
        );
    }
    *world = temp_world;
}
