use crate::cfg;
use crate::cmtp::{
    AiOption, Ammo, Character, Equipment, Item, ItemKind, LogMessage, MapObject, Player,
    PlayerAction, PlayerState, Slot, Symbol,
};
use crate::engine;
use crate::engine::game;

pub fn update(world: &mut game::World) {
    if world.player.state != PlayerState::MakingTurn {
        return;
    }
    if (world.player.action != PlayerAction::InteractWithMap) || !world.player_is_alive() {
        return;
    }
    let player_symbol = world.player_sym();
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
        let maybe_existing_ammo = get_existing_ammo(item_id, world);
        if let Some(existing_ammo_id) = maybe_existing_ammo {
            add_ammo_to_existing(existing_ammo_id, item_id, world);
        } else {
            pick_item_up(item_id, world);
        }
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
        world.add_log(
            cfg::COLOR_DARK_RED,
            format!("Your inventory is full, cannot pick up {}.", name),
        );
    } else {
        world.add_log(cfg::COLOR_GREEN, format!("You picked up a {}!", name));
        let player_id = world.player.id;
        let (_, map_obj, item, eqp, _) = world.get_item_mut(object_id).unwrap();
        item.owner = player_id;
        map_obj.hidden = true;
        // automatically equip, if the corresponding equipment slot is unused
        if let Some(&mut Equipment { slot, .. }) = eqp {
            if (slot != Slot::Ammo) && world.get_equipped_in_slot(slot).is_none() {
                engine::equip(object_id, world);
            }
        }
    }
}

fn get_existing_ammo(unknown_item_id: u32, world: &game::World) -> Option<u32> {
    let player_id = world.player.id;
    let item_name = &world.get_item(unknown_item_id).unwrap().1.name;
    let maybe_ammo = world
        .get_item(unknown_item_id)
        .and_then(|(.., ammo)| ammo)
        .map(|ammo| (item_name, ammo.kind));
    if let Some(new_ammo) = maybe_ammo {
        world
            .item_iter()
            .filter(|&(.., item, _, _)| (item.kind == ItemKind::Ammo) && (item.owner == player_id))
            .map(|(id, _, map_obj, .., ammo)| (id, (&map_obj.name, ammo.unwrap().kind)))
            .map(|(id, existing_ammo)| (id, existing_ammo, new_ammo))
            .find(|&(_, existing_ammo, new_ammo)| existing_ammo == new_ammo)
            .map(|(id, ..)| id)
    } else {
        None
    }
}

fn add_ammo_to_existing(existing_ammo_id: u32, new_ammo_id: u32, world: &mut game::World) {
    let name = world.get_item(new_ammo_id).unwrap().1.name.clone();
    let count_of_new = world.get_item(new_ammo_id).unwrap().4.unwrap().count;
    let mut existing_ammo = world.get_item_mut(existing_ammo_id).unwrap().4.unwrap();
    existing_ammo.count += count_of_new;
    world.entity_indexes.remove(&new_ammo_id);
    world.add_log(cfg::COLOR_GREEN, format!("You picked up a {}!", name));
}

/// Advance to the next level
fn next_level(world: &mut game::World) {
    clear_dungeon(world);
    world.add_log(
        cfg::COLOR_GREEN,
        "You take a moment to rest, and recover your strength.",
    );
    let heal_hp = world.max_hp(world.player.id) / 2;
    heal(world.player.id, heal_hp, world);
    world.add_log(
        cfg::COLOR_ORANGE,
        "After a rare moment of peace, you descend deeper into \
         the heart of the mine...",
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
        looking_at: None,
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
        temp_world.player.id = game::new_entity()
            .add_symbol(symbol)
            .add_map_object(map_object)
            .add_character(character)
            .add_ai(AiOption { option: None })
            .create(&mut temp_world);
    }
    // copy inventory
    let inventory = world
        .item_iter()
        .filter(|(.., item, _, _)| item.owner == world.player.id);
    for (_, sym, map_obj, item, equipment, ammo) in inventory {
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
        let item = Item {
            kind: item.kind,
            owner: temp_world.player.id,
        };
        let equipment = equipment.map(|equipment| Equipment {
            slot: equipment.slot,
            equipped: equipment.equipped,
            max_hp_bonus: equipment.max_hp_bonus,
            defense_bonus: equipment.defense_bonus,
            power_bonus: equipment.power_bonus,
        });
        let ammo = ammo.map(|ammo| Ammo {
            kind: ammo.kind,
            count: ammo.count,
        });
        game::new_entity()
            .add_symbol(symbol)
            .add_map_object(map_object)
            .add_item(item)
            .add_equipment(equipment)
            .add_ammo(ammo)
            .create(&mut temp_world);
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
        game::new_entity()
            .add_log_message(log_message)
            .create(&mut temp_world);
    }
    *world = temp_world;
}

/// heal by the given amount, without going over the maximum
fn heal(id: u32, amount: i32, world: &mut game::World) {
    let max_hp = world.max_hp(id);
    let character = world.get_character_mut(id).unwrap().2;
    character.hp += amount;
    if character.hp > max_hp {
        character.hp = max_hp;
    }
}
