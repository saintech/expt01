use crate::cfg;
use crate::cmtp::{
    Character, Equipment, LogMessage, MapObject, OwnedItem, Player, PlayerAction, PlayerState,
    Symbol,
};
use crate::game;

pub fn update(world: &mut game::World) {
    if world.player.state != PlayerState::MakingTurn {
        return;
    }
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player_character = &world.characters[player_indexes.character.unwrap()];
    if (world.player.action != PlayerAction::InteractWithMap) || !player_character.alive {
        return;
    }
    let player_symbol = &world.symbols[player_indexes.symbol.unwrap()];
    let player_pos = (player_symbol.x, player_symbol.y);
    // pick up an item or go to next level
    let item_id = world.entity_indexes.iter().find_map(|(&id, indexes)| {
        indexes
            .symbol
            .filter(|&sy| (world.symbols[sy].x, world.symbols[sy].y) == player_pos)
            .and(indexes.item)
            .and(indexes.map_object)
            .filter(|&mo| !world.map_objects[mo].hidden)
            .and(Some(id))
    });
    let player_on_stairs = world.entity_indexes.values().any(|indexes| {
        indexes
            .symbol
            .filter(|&sy| (world.symbols[sy].x, world.symbols[sy].y) == player_pos)
            .filter(|_| world.map_objects[indexes.map_object.unwrap()].name == "stairs")
            .is_some()
    });
    if let Some(item_id) = item_id {
        pick_item_up(item_id, world);
    } else if player_on_stairs {
        next_level(world);
    };
}

/// add to the player's inventory and remove from the map
fn pick_item_up(object_id: u32, world: &mut game::World) {
    let indexes = &world.entity_indexes[&object_id];
    let name = &world.map_objects[indexes.map_object.unwrap()].name.clone();
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
        world.items[indexes.item.unwrap()].owner = world.player.id;
        world.map_objects[indexes.map_object.unwrap()].hidden = true;
        let slot = indexes.equipment.map(|it| world.equipments[it].slot);
        game::add_log(
            world,
            format!("You picked up a {}!", name),
            cfg::COLOR_GREEN,
        );
        // automatically equip, if the corresponding equipment slot is unused
        if let Some(slot) = slot {
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
            char: world.symbols[indexes.symbol.unwrap()].char,
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
            None,
            None,
            None,
            None,
            None,
        );
    }
    // copy inventory
    let inventory = world.entity_indexes.iter().filter(|&(_, indexes)| {
        indexes
            .item
            .filter(|&it| world.items[it].owner == world.player.id)
            .is_some()
    });
    for (&id, indexes) in inventory {
        let symbol = Symbol {
            x: world.symbols[indexes.symbol.unwrap()].x,
            y: world.symbols[indexes.symbol.unwrap()].y,
            char: world.symbols[indexes.symbol.unwrap()].char,
            color: world.symbols[indexes.symbol.unwrap()].color,
        };
        let map_object = MapObject {
            name: world.map_objects[indexes.map_object.unwrap()].name.clone(),
            block: world.map_objects[indexes.map_object.unwrap()].block,
            always_visible: world.map_objects[indexes.map_object.unwrap()].always_visible,
            hidden: world.map_objects[indexes.map_object.unwrap()].hidden,
        };
        let item = OwnedItem {
            item: world.items[indexes.item.unwrap()].item,
            owner: temp_world.player.id,
        };
        let equipment = indexes.equipment.map(|index| Equipment {
            slot: world.equipments[index].slot,
            equipped: world.equipments[index].equipped,
            max_hp_bonus: world.equipments[index].max_hp_bonus,
            defense_bonus: world.equipments[index].defense_bonus,
            power_bonus: world.equipments[index].power_bonus,
        });
        let entity_indexes = game::EntityIndexes {
            symbol: Some(temp_world.symbols.len()),
            map_cell: None,
            map_object: Some(temp_world.map_objects.len()),
            character: None,
            ai: None,
            item: Some(temp_world.items.len()),
            equipment: indexes.equipment.map(|_| temp_world.equipments.len()),
            log_message: None,
            dialog: None,
        };
        temp_world.symbols.push(symbol);
        temp_world.map_objects.push(map_object);
        temp_world.items.push(item);
        equipment.map(|c| temp_world.equipments.push(c));
        temp_world.entity_indexes.insert(id, entity_indexes);
    }
    // copy logs
    let logs = world
        .entity_indexes
        .iter()
        .filter(|&(_, indexes)| indexes.log_message.is_some());
    for (&id, indexes) in logs {
        let log_message = LogMessage(
            world.log[indexes.log_message.unwrap()].0.clone(),
            world.log[indexes.log_message.unwrap()].1,
        );
        let entity_indexes = game::EntityIndexes {
            symbol: None,
            map_cell: None,
            map_object: None,
            character: None,
            ai: None,
            item: None,
            equipment: None,
            log_message: Some(temp_world.log.len()),
            dialog: None,
        };
        temp_world.log.push(log_message);
        temp_world.entity_indexes.insert(id, entity_indexes);
    }
    *world = temp_world;
}
