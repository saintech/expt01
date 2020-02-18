use crate::cfg;
use crate::cmtp::{Ai, DialogBox, DialogKind, Item, PlayerAction, PlayerState};
use crate::game;
use std::f32;

fn is_opening_inventory(world: &game::World) -> bool {
    (world.player.state == PlayerState::MakingTurn)
        && ((world.player.action == PlayerAction::OpenInventory)
            || (world.player.action == PlayerAction::DropItem))
        && world.player_is_alive()
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

pub fn update(world: &mut game::World) {
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
            PlayerAction::SelectMenuItem(i) => world
                .item_iter()
                .filter(|(.., item, _)| item.owner == world.player.id)
                .nth(i)
                .map(|(id, ..)| id),
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
                DialogKind::Inventory => use_item(inventory_id, world, false),
                DialogKind::DropItem => drop_item(inventory_id, world),
                _ => unreachable!(),
            }
            world.dialogs.pop();
            if world.dialogs.is_empty() && (world.player.state == PlayerState::InDialog) {
                world.player.state = PlayerState::MakingTurn;
            };
        }
    } else if let Some(inventory_id) = used_targetable_item(world) {
        use_item(inventory_id, world, true);
        world.player.state = PlayerState::MakingTurn;
    }
}

fn add_inventory_menu(world: &mut game::World, kind: DialogKind, header: String) {
    // how a menu with each item of the inventory as an option
    let mut options: Vec<_> = world
        .item_iter()
        .filter(|(.., item, _)| item.owner == world.player.id)
        .map(|(.., map_obj, _, eqp)| {
            eqp.filter(|eqp| eqp.equipped)
                .map_or(map_obj.name.clone(), |eqp| {
                    format!("{} (on {})", map_obj.name, eqp.slot)
                })
        })
        .collect();
    if options.is_empty() {
        options.push(String::from("Inventory is empty."));
    }
    game::add_dialog_box(world, kind, header, options, cfg::INVENTORY_WIDTH);
}

enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
    NeedTargeting,
}

fn use_item(inventory_id: u32, world: &mut game::World, by_targeting: bool) {
    // just call the "use_function" if it is defined
    if let Some((.., item, _)) = world.get_item(inventory_id) {
        use Item::*;
        let on_use = match item.item {
            Medkit => use_medkit,
            SlingshotAmmo => shoot_slingshot,
            Brick => throw_brick,
            BlastingCartridge => throw_blasting_cartridge,
            Melee => toggle_equipment,
            Clothing => toggle_equipment,
        };
        match on_use(inventory_id, world, by_targeting) {
            UseResult::UsedUp => {
                // destroy after use, unless it was cancelled for some reason
                world.get_item_mut(inventory_id).unwrap().2.owner = 0;
                world.entity_indexes.remove(&inventory_id);
            }
            UseResult::UsedAndKept => (),
            UseResult::Cancelled => game::add_log(world, "Cancelled", cfg::COLOR_LIGHTEST_GREY),
            UseResult::NeedTargeting => {
                world.player.state = PlayerState::TargetingTile(inventory_id)
            }
        };
    } else {
        let name = world.get_item_mut(inventory_id).unwrap().1.name.clone();
        game::add_log(
            world,
            format!("The {} cannot be used.", name),
            cfg::COLOR_LIGHTEST_GREY,
        );
    }
}

fn use_medkit(_inventory_id: u32, world: &mut game::World, _by_targeting: bool) -> UseResult {
    // heal the player
    let player_hp = world.get_character(world.player.id).unwrap().2.hp;
    if player_hp == game::max_hp(world.player.id, world) {
        game::add_log(world, "You are already at full health.", cfg::COLOR_ORANGE);
        return UseResult::Cancelled;
    }
    game::add_log(world, "Your wounds start to feel better!", cfg::COLOR_GREEN);
    game::heal(world.player.id, cfg::HEAL_AMOUNT, world);
    UseResult::UsedUp
}

fn shoot_slingshot(_inventory_id: u32, world: &mut game::World, _by_targeting: bool) -> UseResult {
    // find closest enemy (inside a maximum range and damage it)
    let monster_id = closest_monster(cfg::SLINGSHOT_RANGE, world);
    if let Some(monster_id) = monster_id {
        let monster = world.get_character_mut(monster_id).unwrap().2;
        if let Some(xp) = game::take_damage(monster, cfg::SLINGSHOT_DAMAGE) {
            world.get_character_mut(world.player.id).unwrap().2.xp += xp;
        }
        let monster_name = world.get_character(monster_id).unwrap().1.name.clone();
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
fn closest_monster(max_range: i32, world: &game::World) -> Option<u32> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32; // start with (slightly more than) maximum range
    let enemies = world
        .character_iter()
        .filter(|(id, ..)| (*id != world.player.id) && world.check_fov(*id))
        .map(|(id, sym, ..)| (id, sym.x, sym.y));
    for (id, enemy_x, enemy_y) in enemies {
        let player_symbol = world.get_character(world.player.id).unwrap().0;
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

fn throw_brick(_inventory_id: u32, world: &mut game::World, by_targeting: bool) -> UseResult {
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
        let monster_id = target_monster(world, cfg::BRICK_RANGE, position);
        if let Some(monster_id) = monster_id {
            let monster_ai = world.get_character_mut(monster_id).unwrap().3;
            let old_ai = monster_ai.option.take().unwrap_or(Ai::Basic);
            // replace the monster's AI with a "confused" one; after
            // some turns it will restore the old AI
            monster_ai.option = Some(Ai::Confused {
                previous_ai: Box::new(old_ai),
                num_turns: cfg::BRICK_NUM_TURNS,
            });
            let monster_name = world.get_character(monster_id).unwrap().1.name.clone();
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
        if !target_tile(world, f32::INFINITY, (x, y)) {
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
            .character_iter()
            .filter(|(_, sym, ..)| {
                game::distance_to(sym.x, sym.y, x, y) <= cfg::BLASTING_RADIUS as f32
            })
            .map(|(id, ..)| id)
            .collect();
        for target_id in targets {
            let target = world.get_character_mut(target_id).unwrap().2;
            if let Some(xp) = game::take_damage(target, cfg::BLASTING_DAMAGE) {
                if target_id != world.player.id {
                    // Don't reward the player for burning themself!
                    xp_to_gain += xp;
                }
            }
            let target_name = world.get_character(target_id).unwrap().1.name.clone();
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
        world.get_character_mut(world.player.id).unwrap().2.xp += xp_to_gain;
        UseResult::UsedUp
    }
}

fn toggle_equipment(inventory_id: u32, world: &mut game::World, _by_targeting: bool) -> UseResult {
    let equipment = world.get_item(inventory_id).unwrap().3.unwrap();
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
    let name = world.get_item(id).unwrap().1.name.clone();
    if let Some(equipment) = world.get_item_mut(id).unwrap().3 {
        if equipment.equipped {
            equipment.equipped = false;
            let slot = equipment.slot;
            game::add_log(
                world,
                format!("Dequipped {} from {}.", name, slot),
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
fn target_monster(world: &game::World, max_range: f32, (x, y): (i32, i32)) -> Option<u32> {
    if target_tile(world, max_range, (x, y)) {
        world
            .character_iter()
            .find(|(id, sym, ..)| ((sym.x, sym.y) == (x, y)) && (*id != world.player.id))
            .map(|(id, ..)| id)
    } else {
        None
    }
}

/// return tue if the position of a tile is clicked in player's FOV (optionally in a
/// range).
fn target_tile(world: &game::World, max_range: f32, (x, y): (i32, i32)) -> bool {
    let player_symbol = world.get_character(world.player.id).unwrap().0;
    let (player_x, player_y) = (player_symbol.x, player_symbol.y);
    let target_index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
    world.map[target_index_in_map].in_fov
        && (game::distance_to(player_x, player_y, x, y) <= max_range)
}

fn drop_item(inventory_id: u32, world: &mut game::World) {
    let maybe_equipment = world.get_item(inventory_id).unwrap().3;
    if maybe_equipment.is_some() {
        dequip(inventory_id, world);
    }
    let player_symbol = world.get_character(world.player.id).unwrap().0;
    let (player_x, player_y) = (player_symbol.x, player_symbol.y);
    let (symbol, map_obj, item, _) = world.get_item_mut(inventory_id).unwrap();
    item.owner = 0;
    map_obj.hidden = false;
    symbol.x = player_x;
    symbol.y = player_y;
    let name = map_obj.name.clone();
    game::add_log(
        world,
        format!("You dropped a {}.", name),
        cfg::COLOR_DARK_SKY,
    );
}
