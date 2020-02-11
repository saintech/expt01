use crate::cfg;
use crate::cmtp::{Ai, DialogBox, DialogKind, Item, PlayerAction, PlayerState};
use crate::game;
use std::f32;

fn player_is_alive(world: &game::World) -> bool {
    world
        .entity_indexes
        .get(&world.player.id)
        .map(|player_indexes| &world.characters[player_indexes.character.unwrap()])
        .filter(|player_character| player_character.alive)
        .is_some()
}

fn is_opening_inventory(world: &game::World) -> bool {
    (world.player.state == PlayerState::MakingTurn)
        && ((world.player.action == PlayerAction::OpenInventory)
            || (world.player.action == PlayerAction::DropItem))
        && player_is_alive(world)
}

fn inventory_kind(dialog_box: &DialogBox) -> Option<DialogKind> {
    if let DialogKind::Inventory | DialogKind::DropItem = dialog_box.kind {
        Some(dialog_box.kind)
    } else {
        None
    }
}

fn used_targetable_item(world: &game::World) -> Option<u32> {
    if let PlayerState::TargetingTile(inventory_id) = world.player.state {
        if let PlayerAction::ClickAt(..) | PlayerAction::Cancel = world.player.action {
            Some(inventory_id)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn update(world: &mut game::World, tcod: &mut game::Tcod) {
    let is_opening_inventory = is_opening_inventory(world);
    let opened_menu = world.dialogs.last().and_then(inventory_kind);
    if is_opening_inventory {
        let (dialog_kind, menu_title) = match world.player.action {
            PlayerAction::OpenInventory => (
                DialogKind::Inventory,
                "Press the key next to an item to use it, or Esc to cancel.",
            ),
            PlayerAction::DropItem => (
                DialogKind::DropItem,
                "Press the key next to an item to drop it, or Esc to cancel.'",
            ),
            _ => unreachable!(),
        };
        add_inventory_menu(world, dialog_kind, String::from(menu_title));
        world.player.state = PlayerState::InDialog;
    } else if let Some(dialog_kind) = opened_menu {
        let inventory_id = match world.player.action {
            PlayerAction::SelectMenuItem(i) => get_inventory(world).get(i).copied(),
            PlayerAction::Cancel => {
                world.dialogs.pop();
                if world.dialogs.is_empty() {
                    world.player.state = PlayerState::MakingTurn;
                };
                None
            }
            _ => None,
        };
        if let Some(inventory_id) = inventory_id {
            match dialog_kind {
                DialogKind::Inventory => use_item(inventory_id, world, tcod, false),
                DialogKind::DropItem => drop_item(inventory_id, world),
                _ => unreachable!(),
            }
            world.dialogs.pop();
            if world.dialogs.is_empty() && (world.player.state == PlayerState::InDialog) {
                world.player.state = PlayerState::MakingTurn;
            };
        }
    } else if let Some(inventory_id) = used_targetable_item(world) {
        use_item(inventory_id, world, tcod, true);
        world.player.state = PlayerState::MakingTurn;
    }
}

fn get_inventory(world: &game::World) -> Vec<u32> {
    world
        .entity_indexes
        .iter()
        .filter_map(|(&id, indexes)| {
            indexes
                .item
                .filter(|&it| world.items[it].owner == world.player.id)
                .and(Some(id))
        })
        .collect()
}

fn add_inventory_menu(world: &mut game::World, kind: DialogKind, header: String) {
    let inventory: Vec<_> = get_inventory(world);
    // how a menu with each item of the inventory as an option
    let options = if inventory.len() == 0 {
        vec![String::from("Inventory is empty.")]
    } else {
        inventory
            .iter()
            .map(|id| {
                let name = &world.map_objects[world.entity_indexes[id].map_object.unwrap()].name;
                world.entity_indexes[id]
                    .equipment
                    .filter(|&eq| world.equipments[eq].equipped)
                    .map_or(name.clone(), |eq| {
                        format!("{} (on {})", name, world.equipments[eq].slot)
                    })
            })
            .collect()
    };
    game::add_dialog_box(world, kind, header, options, cfg::INVENTORY_WIDTH);
}

enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
    NeedTargeting,
}

fn use_item(inventory_id: u32, world: &mut game::World, tcod: &mut game::Tcod, by_targeting: bool) {
    // just call the "use_function" if it is defined
    if let Some(item_index) = world.entity_indexes[&inventory_id].item {
        use Item::*;
        let on_use = match world.items[item_index].item {
            Medkit => use_medkit,
            SlingshotAmmo => shoot_slingshot,
            Brick => throw_brick,
            BlastingCartridge => throw_blasting_cartridge,
            Melee => toggle_equipment,
            Clothing => toggle_equipment,
        };
        match on_use(inventory_id, world, tcod, by_targeting) {
            UseResult::UsedUp => {
                let item_indexes = &world.entity_indexes[&inventory_id];
                // destroy after use, unless it was cancelled for some reason
                world.items[item_indexes.item.unwrap()].owner = 0;
                world.entity_indexes.remove(&inventory_id);
            }
            UseResult::UsedAndKept => (),
            UseResult::Cancelled => game::add_log(world, "Cancelled", cfg::COLOR_LIGHTEST_GREY),
            UseResult::NeedTargeting => {
                world.player.state = PlayerState::TargetingTile(inventory_id)
            }
        };
    } else {
        let item_indexes = &world.entity_indexes[&inventory_id];
        let name = world.map_objects[item_indexes.map_object.unwrap()]
            .name
            .clone();
        game::add_log(
            world,
            format!("The {} cannot be used.", name),
            cfg::COLOR_LIGHTEST_GREY,
        );
    }
}

fn use_medkit(
    _inventory_id: u32,
    world: &mut game::World,
    _tcod: &mut game::Tcod,
    _by_targeting: bool,
) -> UseResult {
    // heal the player
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player = &world.characters[player_indexes.character.unwrap()];
    if player.hp == game::max_hp(world.player.id, world) {
        game::add_log(world, "You are already at full health.", cfg::COLOR_ORANGE);
        return UseResult::Cancelled;
    }
    game::add_log(world, "Your wounds start to feel better!", cfg::COLOR_GREEN);
    game::heal(world.player.id, cfg::HEAL_AMOUNT, world);
    UseResult::UsedUp
}

fn shoot_slingshot(
    _inventory_id: u32,
    world: &mut game::World,
    tcod: &mut game::Tcod,
    _by_targeting: bool,
) -> UseResult {
    // find closest enemy (inside a maximum range and damage it)
    let monster_id = closest_monster(cfg::SLINGSHOT_RANGE, world, tcod);
    if let Some(monster_id) = monster_id {
        let indexes = &world.entity_indexes[&monster_id];
        let monster = &mut world.characters[indexes.character.unwrap()];
        if let Some(xp) = game::take_damage(monster, cfg::SLINGSHOT_DAMAGE) {
            let player_indexes = &world.entity_indexes[&world.player.id];
            world.characters[player_indexes.character.unwrap()].xp += xp;
        }
        let monster_name = world.map_objects[indexes.map_object.unwrap()].name.clone();
        game::add_log(
            world,
            format!(
                "A Steel Ball whizzed to a {}! The damage is {} hit points.",
                monster_name,
                cfg::SLINGSHOT_DAMAGE
            ),
            cfg::COLOR_LIGHTEST_GREY,
        );
        UseResult::UsedUp
    } else {
        // no enemy found within maximum range
        game::add_log(
            world,
            "No enemy is close enough to shoot.",
            cfg::COLOR_DARK_SKY,
        );
        UseResult::Cancelled
    }
}

/// find closest enemy, up to a maximum range, and in the player's FOV
fn closest_monster(max_range: i32, world: &game::World, tcod: &game::Tcod) -> Option<u32> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32; // start with (slightly more than) maximum range
    let enemies = world.entity_indexes.iter().filter_map(|(&id, indexes)| {
        indexes
            .character
            .and(indexes.ai)
            .and(indexes.symbol)
            .filter(|&sy| tcod.fov.is_in_fov(world.symbols[sy].x, world.symbols[sy].y))
            .map(|sy| (id, world.symbols[sy].x, world.symbols[sy].y))
    });
    for (id, enemy_x, enemy_y) in enemies {
        let player_indexes = &world.entity_indexes[&world.player.id];
        let player_symbol = &world.symbols[player_indexes.symbol.unwrap()];
        // calculate distance between this object and the player
        let dist = game::distance_to(player_symbol.x, player_symbol.y, enemy_x, enemy_y);
        if dist < closest_dist {
            // it's closer, so remember it
            closest_enemy = Some(id);
            closest_dist = dist;
        }
    }
    closest_enemy
}

fn throw_brick(
    _inventory_id: u32,
    world: &mut game::World,
    _tcod: &mut game::Tcod,
    by_targeting: bool,
) -> UseResult {
    if !by_targeting {
        // ask the player for a target to confuse
        game::add_log(
            world,
            "Left-click an enemy to throw the brick, or right-click to cancel.",
            cfg::COLOR_DARK_SKY,
        );
        UseResult::NeedTargeting
    } else {
        let position = match world.player.action {
            PlayerAction::ClickAt(x, y) => (x, y),
            PlayerAction::Cancel => return UseResult::Cancelled,
            _ => unreachable!(),
        };
        let monster_id = target_monster(world, Some(cfg::BRICK_RANGE as f32), position);
        if let Some(monster_id) = monster_id {
            let indexes = &world.entity_indexes[&monster_id];
            let monster_ai = &mut world.ais[indexes.ai.unwrap()];
            let old_ai = monster_ai.option.take().unwrap_or(Ai::Basic);
            // replace the monster's AI with a "confused" one; after
            // some turns it will restore the old AI
            monster_ai.option = Some(Ai::Confused {
                previous_ai: Box::new(old_ai),
                num_turns: cfg::BRICK_NUM_TURNS,
            });
            let monster_name = world.map_objects[indexes.map_object.unwrap()].name.clone();
            game::add_log(
                world,
                format!(
                    "The eyes of {} look vacant, as he starts to stumble around!",
                    monster_name
                ),
                cfg::COLOR_LIGHTEST_GREY,
            );
            UseResult::UsedUp
        } else {
            game::add_log(
                world,
                "No enemy is close enough to throw.",
                cfg::COLOR_DARK_SKY,
            );
            UseResult::Cancelled
        }
    }
}

fn throw_blasting_cartridge(
    _inventory_id: u32,
    world: &mut game::World,
    _tcod: &mut game::Tcod,
    by_targeting: bool,
) -> UseResult {
    if !by_targeting {
        game::add_log(
            world,
            "Left-click a target tile to throw the charge, or right-click to cancel.",
            cfg::COLOR_DARK_SKY,
        );
        UseResult::NeedTargeting
    } else {
        let (x, y) = match world.player.action {
            PlayerAction::ClickAt(x, y) => (x, y),
            PlayerAction::Cancel => return UseResult::Cancelled,
            _ => unreachable!(),
        };
        if !target_tile(world, None, (x, y)) {
            return UseResult::Cancelled;
        }
        game::add_log(
            world,
            format!(
                "The Blasting Cartridge explodes, crushing everything within {} tiles!",
                cfg::BLASTING_RADIUS
            ),
            cfg::COLOR_ORANGE,
        );
        let mut xp_to_gain = 0;
        let targets: Vec<_> = world
            .entity_indexes
            .iter()
            .filter_map(|(&id, indexes)| {
                indexes
                    .character
                    .and(indexes.symbol)
                    .map(|sy| (world.symbols[sy].x, world.symbols[sy].y))
                    .filter(|&(cx, cy)| {
                        game::distance_to(cx, cy, x, y) <= cfg::BLASTING_RADIUS as f32
                    })
                    .and(Some(id))
            })
            .collect();
        for target_id in targets {
            let indexes = &world.entity_indexes[&target_id];
            let target = &mut world.characters[indexes.character.unwrap()];
            if let Some(xp) = game::take_damage(target, cfg::BLASTING_DAMAGE) {
                if target_id != world.player.id {
                    // Don't reward the player for burning themself!
                    xp_to_gain += xp;
                }
            }
            let target_name = world.map_objects[indexes.map_object.unwrap()].name.clone();
            game::add_log(
                world,
                format!(
                    "The {} gets damaged for {} hit points.",
                    target_name,
                    cfg::BLASTING_DAMAGE
                ),
                cfg::COLOR_LIGHTEST_GREY,
            );
        }
        world.characters[world.entity_indexes[&world.player.id].character.unwrap()].xp +=
            xp_to_gain;
        UseResult::UsedUp
    }
}

fn toggle_equipment(
    inventory_id: u32,
    world: &mut game::World,
    _tcod: &mut game::Tcod,
    _by_targeting: bool,
) -> UseResult {
    let indexes = &world.entity_indexes[&inventory_id];
    let equipment = &world.equipments[indexes.equipment.unwrap()];
    if equipment.equipped {
        dequip(inventory_id, world);
    } else {
        // if the slot is already being used, dequip whatever is there first
        if let Some(current) = game::get_equipped_in_slot(equipment.slot, world) {
            dequip(current, world);
        }
        game::equip(inventory_id, world);
    }
    UseResult::UsedAndKept
}

/// Dequip object and show a message about it
fn dequip(id: u32, world: &mut game::World) {
    let indexes = &world.entity_indexes[&id];
    let name = world.map_objects[indexes.map_object.unwrap()].name.clone();
    if let Some(index) = indexes.equipment {
        if world.equipments[index].equipped {
            world.equipments[index].equipped = false;
            game::add_log(
                world,
                format!("Dequipped {} from {}.", name, world.equipments[index].slot),
                cfg::COLOR_DARK_SKY,
            );
        }
    } else {
        game::add_log(
            world,
            format!("Can't dequip {} because it's not an Equipment.", name),
            cfg::COLOR_ORANGE,
        );
    }
}

/// returns a clicked monster inside FOV up to a range, or None
fn target_monster(world: &game::World, max_range: Option<f32>, (x, y): (i32, i32)) -> Option<u32> {
    if target_tile(world, max_range, (x, y)) {
        world.entity_indexes.iter().find_map(|(&id, indexes)| {
            indexes
                .character
                .and(indexes.symbol)
                .filter(|&sy| (world.symbols[sy].x, world.symbols[sy].y) == (x, y))
                .filter(|_| id != world.player.id)
                .and(Some(id))
        })
    } else {
        None
    }
}

/// return tue if the position of a tile is clicked in player's FOV (optionally in a
/// range).
fn target_tile(world: &game::World, max_range: Option<f32>, (x, y): (i32, i32)) -> bool {
    let max_range = max_range.unwrap_or(f32::INFINITY);
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player_symbol = &world.symbols[player_indexes.symbol.unwrap()];
    let target_index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
    let (player_x, player_y) = (player_symbol.x, player_symbol.y);
    world.map[target_index_in_map].in_fov
        && (game::distance_to(player_x, player_y, x, y) <= max_range)
}

fn drop_item(inventory_id: u32, world: &mut game::World) {
    if world.entity_indexes[&inventory_id].equipment.is_some() {
        dequip(inventory_id, world);
    }
    let indexes = &world.entity_indexes[&inventory_id];
    world.items[indexes.item.unwrap()].owner = 0;
    world.map_objects[indexes.map_object.unwrap()].hidden = false;
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player_x = world.symbols[player_indexes.symbol.unwrap()].x;
    let player_y = world.symbols[player_indexes.symbol.unwrap()].y;
    let symbol = &mut world.symbols[indexes.symbol.unwrap()];
    symbol.x = player_x;
    symbol.y = player_y;
    let name = &world.map_objects[indexes.map_object.unwrap()].name.clone();
    game::add_log(
        world,
        format!("You dropped a {}.", name),
        cfg::COLOR_DARK_SKY,
    );
}
