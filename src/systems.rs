use crate::cfg::*;
use crate::cmtp::*;
use crate::game::*;
use rand::Rng as _;
use std::f32::INFINITY;
use std::{error::Error, fs, io::Read as _, io::Write as _};
use tcod::{colors, console, input, Console as _};

// *** Input System ***
pub fn update_input_state(world: &mut World, _tcod: &mut Tcod) {
    use input::KeyCode::*;
    world.player.action = match world.player.state {
        PlayerState::InDialog => match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, input::Event::Key(key))) => match (key.code, key.printable) {
                (Escape, _) => PlayerAction::Cancel,
                (Text, printable) => printable_to_action(printable),
                _ => PlayerAction::None,
            },
            _ => PlayerAction::None,
        },

        PlayerState::MakingTurn | PlayerState::TargetingTile(_) => {
            match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
                Some((_, input::Event::Key(key))) => match (key.code, key.printable) {
                    (Escape, _) => PlayerAction::Cancel,
                    (Up, _) | (Number8, _) => PlayerAction::GoToUp,
                    (Down, _) | (NumPad2, _) => PlayerAction::GoToDown,
                    (Left, _) | (NumPad4, _) => PlayerAction::GoToLeft,
                    (Right, _) | (NumPad6, _) => PlayerAction::GoToRight,
                    (Home, _) | (NumPad7, _) => PlayerAction::GoToUpLeft,
                    (PageUp, _) | (NumPad9, _) => PlayerAction::GoToUpRight,
                    (End, _) | (NumPad1, _) => PlayerAction::GoToDownLeft,
                    (PageDown, _) | (NumPad3, _) => PlayerAction::GoToDownRight,
                    (NumPad5, _) => PlayerAction::SkipTurn,
                    (Enter, _) => PlayerAction::InteractWithMap,
                    (F1, _) => PlayerAction::OpenHelp,
                    (Text, 'i') => PlayerAction::OpenInventory,
                    (Text, 'c') => PlayerAction::OpenCharInfo,
                    (Text, 'd') => PlayerAction::DropItem,
                    _ => PlayerAction::None,
                },
                Some((_, input::Event::Mouse(m))) => {
                    match (m.lbutton_pressed, m.rbutton_pressed, m.cx, m.cy) {
                        (false, true, ..) => PlayerAction::Cancel,
                        (false, false, x, y) => PlayerAction::LookAt(x as i32, y as i32),
                        (true, _, x, y) => PlayerAction::ClickAt(x as i32, y as i32),
                    }
                }
                _ => PlayerAction::None,
            }
        }
    }
}

fn printable_to_action(key: char) -> PlayerAction {
    b"123456789abcdefghijklmnopqrstuvwxyz"
        .iter()
        .position(|&val| val as char == key)
        .map_or(PlayerAction::None, |v| PlayerAction::SelectMenuItem(v))
}

// *** Map Interaction System ***
pub fn update_map_interaction_state(world: &mut World, _tcod: &mut Tcod) {
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
fn pick_item_up(object_id: u32, world: &mut World) {
    let indexes = &world.entity_indexes[&object_id];
    let name = &world.map_objects[indexes.map_object.unwrap()].name.clone();
    let inventory_len = world
        .items
        .iter()
        .filter(|&item| item.owner == world.player.id)
        .count();
    if inventory_len >= 35 {
        add_log(
            world,
            format!("Your inventory is full, cannot pick up {}.", name),
            COLOR_DARK_RED,
        );
    } else {
        world.items[indexes.item.unwrap()].owner = world.player.id;
        world.map_objects[indexes.map_object.unwrap()].hidden = true;
        let slot = indexes.equipment.map(|it| world.equipments[it].slot);
        add_log(world, format!("You picked up a {}!", name), COLOR_GREEN);
        // automatically equip, if the corresponding equipment slot is unused
        if let Some(slot) = slot {
            if get_equipped_in_slot(slot, world).is_none() {
                equip(object_id, world);
            }
        }
    }
}

// *** Player Action System ***
pub fn player_move_or_attack(world: &mut World, _tcod: &mut Tcod) {
    if world.player.state != PlayerState::MakingTurn {
        return;
    }
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player_character = &world.characters[player_indexes.character.unwrap()];
    if !player_character.alive {
        return;
    }
    let (dx, dy) = match world.player.action {
        PlayerAction::GoToUp => (0, -1),
        PlayerAction::GoToDown => (0, 1),
        PlayerAction::GoToLeft => (-1, 0),
        PlayerAction::GoToRight => (1, 0),
        PlayerAction::GoToUpLeft => (-1, -1),
        PlayerAction::GoToUpRight => (1, -1),
        PlayerAction::GoToDownLeft => (-1, 1),
        PlayerAction::GoToDownRight => (1, 1),
        _ => return,
    };
    // the coordinates the player is moving to/attacking
    let x = world.symbols[player_indexes.symbol.unwrap()].x + dx;
    let y = world.symbols[player_indexes.symbol.unwrap()].y + dy;
    if (dy > 0) || ((dy == 0) && (dx < 0)) {
        world.characters[player_indexes.character.unwrap()].looking_right = false;
    } else if (dy < 0) || ((dy == 0) && (dx > 0)) {
        world.characters[player_indexes.character.unwrap()].looking_right = true;
    }
    // try to find an attackable object there
    let target_id = world.entity_indexes.iter().find_map(|(&id, indexes)| {
        if let (Some(_), Some(sy)) = (indexes.character, indexes.symbol) {
            if (world.symbols[sy].x, world.symbols[sy].y) == (x, y) {
                Some(id)
            } else {
                None
            }
        } else {
            None
        }
    });
    // attack if target found, move otherwise
    match target_id {
        Some(target_id) => {
            attack_by(world.player.id, target_id, world);
        }
        None => {
            move_by(world.player.id, dx, dy, world);
        }
    }
}

// *** AI System ***
pub fn update_ai_turn_state(world: &mut World, tcod: &mut Tcod) {
    if world.player.state != PlayerState::MakingTurn {
        return;
    }
    // let monsters take their turn
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player = &world.characters[player_indexes.character.unwrap()];
    if player.alive && player_action_is_turn(world.player.action) {
        let ai_ids: Vec<_> = world
            .entity_indexes
            .iter()
            .filter_map(|(&id, indexes)| indexes.character.and(indexes.ai.and(Some(id))))
            .collect();
        for id in ai_ids {
            ai_take_turn(id, world, &tcod.fov);
        }
    }
}

fn player_action_is_turn(action: PlayerAction) -> bool {
    use PlayerAction::*;
    return match action {
        GoToUp | GoToDown | GoToLeft | GoToRight | GoToUpLeft | GoToUpRight | GoToDownLeft
        | GoToDownRight | SkipTurn => true,
        _ => false,
    };
}

fn ai_take_turn(id: u32, world: &mut World, fov_map: &tcod::Map) {
    let ai_index = world.entity_indexes[&id].ai.unwrap();
    if let Some(ai) = world.ais[ai_index].option.take() {
        let new_ai = match ai {
            Ai::Basic => ai_basic(id, world, fov_map),
            Ai::Confused {
                previous_ai,
                num_turns,
            } => ai_confused(id, world, previous_ai, num_turns),
        };
        world.ais[ai_index].option = Some(new_ai);
    }
}

fn ai_basic(monster_id: u32, world: &mut World, fov_map: &tcod::Map) -> Ai {
    let monster_indexes = &world.entity_indexes[&monster_id];
    let monster_symbol = &world.symbols[monster_indexes.symbol.unwrap()];
    let (monster_x, monster_y) = (monster_symbol.x, monster_symbol.y);
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player_hp = world.characters[player_indexes.character.unwrap()].hp;
    let player_symbol = &world.symbols[player_indexes.symbol.unwrap()];
    let (player_x, player_y) = (player_symbol.x, player_symbol.y);
    if (monster_x > player_x) || ((monster_x == player_x) && (monster_y < player_y)) {
        world.characters[monster_indexes.character.unwrap()].looking_right = false;
    } else {
        world.characters[monster_indexes.character.unwrap()].looking_right = true;
    }
    if fov_map.is_in_fov(monster_x, monster_y) {
        if distance_to(monster_x, monster_y, player_x, player_y) >= 2.0 {
            // move towards player if far away
            move_towards(monster_id, player_x, player_y, world);
        } else if player_hp > 0 {
            // close enough, attack! (if the player is still alive.)
            attack_by(monster_id, world.player.id, world);
        }
    }
    Ai::Basic
}

fn move_towards(id: u32, target_x: i32, target_y: i32, world: &mut World) {
    let object_indexes = &world.entity_indexes[&id];
    let &Symbol { x, y, .. } = &world.symbols[object_indexes.symbol.unwrap()];
    // vector from this object to the target, and distance
    let dx = target_x - x;
    let dy = target_y - y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();
    // normalize it to length 1 (preserving direction), then round it and
    // convert to integer so the movement is restricted to the map grid
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, world);
}

fn ai_confused(monster_id: u32, world: &mut World, previous_ai: Box<Ai>, num_turns: i32) -> Ai {
    let monster_indexes = &world.entity_indexes[&monster_id];
    let monster_name = world.map_objects[monster_indexes.map_object.unwrap()]
        .name
        .clone();
    if num_turns >= 0 {
        // still confused ...
        // move in a random direction, and decrease the number of turns confused
        move_by(
            monster_id,
            rand::thread_rng().gen_range(-1, 2),
            rand::thread_rng().gen_range(-1, 2),
            world,
        );
        Ai::Confused {
            previous_ai: previous_ai,
            num_turns: num_turns - 1,
        }
    } else {
        // restore the previous AI (this one will be deleted)
        add_log(
            world,
            format!("The {} is no longer confused!", monster_name),
            COLOR_ORANGE,
        );
        *previous_ai
    }
}

fn player_is_alive(world: &World) -> bool {
    world
        .entity_indexes
        .get(&world.player.id)
        .map(|player_indexes| &world.characters[player_indexes.character.unwrap()])
        .filter(|player_character| player_character.alive)
        .is_some()
}

fn is_opening_inventory(world: &World) -> bool {
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

fn used_targetable_item(world: &World) -> Option<u32> {
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

// *** Inventory System ***
pub fn update_inventory_state(world: &mut World, tcod: &mut Tcod) {
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

enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
    NeedTargeting,
}

fn use_item(inventory_id: u32, world: &mut World, tcod: &mut Tcod, by_targeting: bool) {
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
            UseResult::Cancelled => add_log(world, "Cancelled", COLOR_LIGHTEST_GREY),
            UseResult::NeedTargeting => {
                world.player.state = PlayerState::TargetingTile(inventory_id)
            }
        };
    } else {
        let item_indexes = &world.entity_indexes[&inventory_id];
        let name = world.map_objects[item_indexes.map_object.unwrap()]
            .name
            .clone();
        add_log(
            world,
            format!("The {} cannot be used.", name),
            COLOR_LIGHTEST_GREY,
        );
    }
}

fn use_medkit(
    _inventory_id: u32,
    world: &mut World,
    _tcod: &mut Tcod,
    _by_targeting: bool,
) -> UseResult {
    // heal the player
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player = &world.characters[player_indexes.character.unwrap()];
    if player.hp == max_hp(world.player.id, world) {
        add_log(world, "You are already at full health.", COLOR_ORANGE);
        return UseResult::Cancelled;
    }
    add_log(world, "Your wounds start to feel better!", COLOR_GREEN);
    heal(world.player.id, HEAL_AMOUNT, world);
    UseResult::UsedUp
}

fn shoot_slingshot(
    _inventory_id: u32,
    world: &mut World,
    tcod: &mut Tcod,
    _by_targeting: bool,
) -> UseResult {
    // find closest enemy (inside a maximum range and damage it)
    let monster_id = closest_monster(SLINGSHOT_RANGE, world, tcod);
    if let Some(monster_id) = monster_id {
        let indexes = &world.entity_indexes[&monster_id];
        let monster = &mut world.characters[indexes.character.unwrap()];
        if let Some(xp) = take_damage(monster, SLINGSHOT_DAMAGE) {
            let player_indexes = &world.entity_indexes[&world.player.id];
            world.characters[player_indexes.character.unwrap()].xp += xp;
        }
        let monster_name = world.map_objects[indexes.map_object.unwrap()].name.clone();
        add_log(
            world,
            format!(
                "A Steel Ball whizzed to a {}! The damage is {} hit points.",
                monster_name, SLINGSHOT_DAMAGE
            ),
            COLOR_LIGHTEST_GREY,
        );
        UseResult::UsedUp
    } else {
        // no enemy found within maximum range
        add_log(world, "No enemy is close enough to shoot.", COLOR_DARK_SKY);
        UseResult::Cancelled
    }
}

/// find closest enemy, up to a maximum range, and in the player's FOV
fn closest_monster(max_range: i32, world: &World, tcod: &Tcod) -> Option<u32> {
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
        let dist = distance_to(player_symbol.x, player_symbol.y, enemy_x, enemy_y);
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
    world: &mut World,
    _tcod: &mut Tcod,
    by_targeting: bool,
) -> UseResult {
    if !by_targeting {
        // ask the player for a target to confuse
        add_log(
            world,
            "Left-click an enemy to throw the brick, or right-click to cancel.",
            COLOR_DARK_SKY,
        );
        UseResult::NeedTargeting
    } else {
        let position = match world.player.action {
            PlayerAction::ClickAt(x, y) => (x, y),
            PlayerAction::Cancel => return UseResult::Cancelled,
            _ => unreachable!(),
        };
        let monster_id = target_monster(world, Some(BRICK_RANGE as f32), position);
        if let Some(monster_id) = monster_id {
            let indexes = &world.entity_indexes[&monster_id];
            let monster_ai = &mut world.ais[indexes.ai.unwrap()];
            let old_ai = monster_ai.option.take().unwrap_or(Ai::Basic);
            // replace the monster's AI with a "confused" one; after
            // some turns it will restore the old AI
            monster_ai.option = Some(Ai::Confused {
                previous_ai: Box::new(old_ai),
                num_turns: BRICK_NUM_TURNS,
            });
            let monster_name = world.map_objects[indexes.map_object.unwrap()].name.clone();
            add_log(
                world,
                format!(
                    "The eyes of {} look vacant, as he starts to stumble around!",
                    monster_name
                ),
                COLOR_LIGHTEST_GREY,
            );
            UseResult::UsedUp
        } else {
            add_log(world, "No enemy is close enough to throw.", COLOR_DARK_SKY);
            UseResult::Cancelled
        }
    }
}

fn throw_blasting_cartridge(
    _inventory_id: u32,
    world: &mut World,
    _tcod: &mut Tcod,
    by_targeting: bool,
) -> UseResult {
    if !by_targeting {
        add_log(
            world,
            "Left-click a target tile to throw the charge, or right-click to cancel.",
            COLOR_DARK_SKY,
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
        add_log(
            world,
            format!(
                "The Blasting Cartridge explodes, crushing everything within {} tiles!",
                BLASTING_RADIUS
            ),
            COLOR_ORANGE,
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
                    .filter(|&(cx, cy)| distance_to(cx, cy, x, y) <= BLASTING_RADIUS as f32)
                    .and(Some(id))
            })
            .collect();
        for target_id in targets {
            let indexes = &world.entity_indexes[&target_id];
            let target = &mut world.characters[indexes.character.unwrap()];
            if let Some(xp) = take_damage(target, BLASTING_DAMAGE) {
                if target_id != world.player.id {
                    // Don't reward the player for burning themself!
                    xp_to_gain += xp;
                }
            }
            let target_name = world.map_objects[indexes.map_object.unwrap()].name.clone();
            add_log(
                world,
                format!(
                    "The {} gets damaged for {} hit points.",
                    target_name, BLASTING_DAMAGE
                ),
                COLOR_LIGHTEST_GREY,
            );
        }
        world.characters[world.entity_indexes[&world.player.id].character.unwrap()].xp +=
            xp_to_gain;
        UseResult::UsedUp
    }
}

fn toggle_equipment(
    inventory_id: u32,
    world: &mut World,
    _tcod: &mut Tcod,
    _by_targeting: bool,
) -> UseResult {
    let indexes = &world.entity_indexes[&inventory_id];
    let equipment = &world.equipments[indexes.equipment.unwrap()];
    if equipment.equipped {
        dequip(inventory_id, world);
    } else {
        // if the slot is already being used, dequip whatever is there first
        if let Some(current) = get_equipped_in_slot(equipment.slot, world) {
            dequip(current, world);
        }
        equip(inventory_id, world);
    }
    UseResult::UsedAndKept
}

/// returns a clicked monster inside FOV up to a range, or None
fn target_monster(world: &World, max_range: Option<f32>, (x, y): (i32, i32)) -> Option<u32> {
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
fn target_tile(world: &World, max_range: Option<f32>, (x, y): (i32, i32)) -> bool {
    let max_range = max_range.unwrap_or(INFINITY);
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player_symbol = &world.symbols[player_indexes.symbol.unwrap()];
    let target_index_in_map = (y * MAP_WIDTH + x) as usize;
    let (player_x, player_y) = (player_symbol.x, player_symbol.y);
    world.map[target_index_in_map].in_fov && (distance_to(player_x, player_y, x, y) <= max_range)
}

fn drop_item(inventory_id: u32, world: &mut World) {
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
    add_log(world, format!("You dropped a {}.", name), COLOR_DARK_SKY);
}

// *** Death System ***
pub fn update_death_state(world: &mut World, _tcod: &mut Tcod) {
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
        let callback: fn(u32, &mut World) = match callback {
            Player => player_death,
            Monster => monster_death,
            None => unreachable!(),
        };
        callback(id, world);
    }
}

fn player_death(_id: u32, world: &mut World) {
    // the game ended!
    add_log(world, "You died!", COLOR_DARK_RED);
    // for added effect, transform the player into a corpse!
    let indexes = &world.entity_indexes[&world.player.id];
    let symbol = &mut world.symbols[indexes.symbol.unwrap()];
    symbol.char = '\u{A3}';
    symbol.color = COLOR_DARK_RED;
    let player = &mut world.characters[indexes.character.unwrap()];
    player.on_death = DeathCallback::None;
}

fn monster_death(monster_id: u32, world: &mut World) {
    let indexes = &world.entity_indexes[&monster_id];
    let name = world.map_objects[indexes.map_object.unwrap()].name.clone();
    let xp = world.characters[indexes.character.unwrap()].xp;
    // transform it into a nasty corpse! it doesn't block, can't be
    // attacked and doesn't move
    add_log(
        world,
        format!("{} is dead! You gain {} experience points.", name, xp),
        COLOR_ORANGE,
    );
    let indexes = world.entity_indexes.get_mut(&monster_id).unwrap();
    let symbol = &mut world.symbols[indexes.symbol.unwrap()];
    let map_object = &mut world.map_objects[indexes.map_object.unwrap()];
    symbol.char = '\u{A3}';
    symbol.color = COLOR_DARK_RED;
    map_object.block = false;
    indexes.character = None;
    map_object.name = format!("remains of {}", map_object.name);
}

fn get_lvl_up_player(world: &mut World) -> Option<&mut Character> {
    if world.player.state == PlayerState::MakingTurn {
        world
            .entity_indexes
            .get(&world.player.id)
            .map(|player_indexes| player_indexes.character.unwrap())
            .map(move |player_index| &mut world.characters[player_index])
            .filter(|player_character| {
                let level_up_xp = LEVEL_UP_BASE + player_character.level * LEVEL_UP_FACTOR;
                player_character.xp >= level_up_xp
            })
    } else {
        None
    }
}

fn is_lvl_up_menu(dialog_box: &&DialogBox) -> bool {
    dialog_box.kind == DialogKind::LevelUp
}

// *** Character System ***
pub fn update_character_state(world: &mut World, _tcod: &mut Tcod) {
    let lvl_up_is_open = world.dialogs.last().filter(is_lvl_up_menu).is_some();
    let lvl_up_player = get_lvl_up_player(world);
    if let Some(player) = lvl_up_player {
        // it is! level up
        let new_level = player.level + 1;
        let base_max_hp = player.base_max_hp;
        let base_power = player.base_power;
        let base_defense = player.base_defense;
        let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
        player.level += 1;
        player.xp -= level_up_xp;
        add_log(
            world,
            format!(
                "Your battle skills grow stronger! You reached level {}!",
                new_level,
            ),
            COLOR_ORANGE,
        );
        let header = String::from("Level up! Choose a stat to raise:\n");
        let options = vec![
            format!("Constitution (+20 HP, from {})", base_max_hp),
            format!("Strength (+1 attack, from {})", base_power),
            format!("Agility (+1 defense, from {})", base_defense),
        ];
        add_dialog_box(
            world,
            DialogKind::LevelUp,
            header,
            options,
            LEVEL_SCREEN_WIDTH,
        );
        world.player.state = PlayerState::InDialog;
    } else if lvl_up_is_open {
        let player_indexes = &world.entity_indexes[&world.player.id];
        let player = &mut world.characters[player_indexes.character.unwrap()];
        let should_close_dialog = match world.player.action {
            PlayerAction::SelectMenuItem(0) => {
                player.base_max_hp += 20;
                player.hp += 20;
                true
            }
            PlayerAction::SelectMenuItem(1) => {
                player.base_power += 1;
                true
            }
            PlayerAction::SelectMenuItem(2) => {
                player.base_defense += 1;
                true
            }
            _ => false,
        };
        if should_close_dialog {
            world.dialogs.pop();
            if world.dialogs.is_empty() {
                world.player.state = PlayerState::MakingTurn;
            };
        }
    }
}

// *** Stats Menu System ***
pub fn update_stats_menu_state(world: &mut World, _tcod: &mut Tcod) {
    let should_open_stats = (world.player.state == PlayerState::MakingTurn)
        && (world.player.action == PlayerAction::OpenCharInfo);
    if !should_open_stats {
        return;
    }
    if let Some(player) = world
        .entity_indexes
        .get(&world.player.id)
        .map(|indexes| &world.characters[indexes.character.unwrap()])
    {
        // show character information
        let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
        let msg = format!(
            "Character information\n\
             \n\
             Level: {}\n\
             Experience: {}\n\
             Experience to level up: {}\n\
             \n\
             Maximum HP: {}\n\
             Attack: {}\n\
             Defense: {}",
            player.level,
            player.xp,
            level_up_xp,
            max_hp(world.player.id, world),
            power(world.player.id, world),
            defense(world.player.id, world),
        );
        add_dialog_box(
            world,
            DialogKind::MessageBox,
            msg,
            vec![],
            CHARACTER_SCREEN_WIDTH,
        );
    }
}

// *** Help Menu System ***
pub fn update_help_menu_state(world: &mut World, _tcod: &mut Tcod) {
    let should_open_help = (world.player.state == PlayerState::MakingTurn)
        && (world.player.action == PlayerAction::OpenHelp);
    if !should_open_help {
        return;
    }
    let msg = "           How To Play\n\
               \n\
               \n\
               Save And Exit........Esc\n\
               Look.................Mouse\n\
               Pick Up, Downstairs..Enter\n\
               Inventory............I\n\
               Character Info.......C\n\
               Drop Item............D\n\
               Move Character.......Arrows, Home,\n\
               \x20                    End, Page Up,\n\
               \x20                    Page Down,\n\
               \x20                    Numpad";
    add_dialog_box(world, DialogKind::MessageBox, String::from(msg), vec![], 36);
}

fn save_game(world: &World) {
    let save_data = serde_json::to_string(world).unwrap();
    let mut file = fs::File::create("savegame").unwrap();
    file.write_all(save_data.as_bytes()).unwrap();
}

// *** Render System ***
pub fn render_all(world: &mut World, tcod: &mut Tcod) {
    if !world.map.is_empty() {
        render_map(&world.map, &mut tcod.con);
        render_map_objects(world, &mut tcod.con);
        // blit the contents of "con" to the root console
        console::blit(
            &tcod.con,
            (0, 0),
            (MAP_WIDTH, MAP_HEIGHT),
            &mut tcod.root,
            (0, 0),
            1.0,
            1.0,
        );
    } else {
        render_main_menu_bg(&mut tcod.root);
    }
    if let Some(player_indexes) = world.entity_indexes.get(&world.player.id) {
        render_panel(world, player_indexes, &mut tcod.panel);
        // blit the contents of `panel` to the root console
        console::blit(
            &tcod.panel,
            (0, 0),
            (SCREEN_WIDTH, PANEL_HEIGHT),
            &mut tcod.root,
            (0, PANEL_Y),
            1.0,
            1.0,
        );
    }
    render_dialogs(world, &mut tcod.root);
    tcod.root.flush();
}

fn render_map(map: &Vec<MapCell>, con: &mut impl console::Console) {
    con.set_default_background(COLOR_DARK_GROUND_BG);
    con.clear();
    for i in 0..map.len() {
        let (x, y) = ((i as i32) % MAP_WIDTH, (i as i32) / MAP_WIDTH);
        let visible = map[i].in_fov;
        let wall = map[i].block_sight;
        let wall_bottom =
            ((y + 1) < MAP_HEIGHT) && wall && !map[((y + 1) * MAP_WIDTH + x) as usize].block_sight;
        let ground_sprite = (GROUND_BITMAP & 1usize.rotate_left(i as u32)) != 0;
        let (fg, bg, glyph) = match (visible, wall, wall_bottom, ground_sprite) {
            // outside of field of view:
            (false, true, false, _) => (COLOR_DARK_WALL, COLOR_DARK_WALL_BG, '\u{A0}'),
            (false, true, true, _) => (COLOR_DARK_WALL, COLOR_DARK_WALL_BG, '\u{A1}'),
            (false, false, _, false) => (COLOR_DARK_GROUND, COLOR_DARK_GROUND_BG, ' '),
            (false, false, _, true) => (COLOR_DARK_GROUND, COLOR_DARK_GROUND_BG, '\u{A2}'),
            // inside fov:
            (true, true, false, _) => (COLOR_LIGHT_WALL, COLOR_LIGHT_WALL_BG, '\u{A0}'),
            (true, true, true, _) => (COLOR_LIGHT_WALL, COLOR_LIGHT_WALL_BG, '\u{A1}'),
            (true, false, _, false) => (COLOR_LIGHT_GROUND, COLOR_LIGHT_GROUND_BG, ' '),
            (true, false, _, true) => (COLOR_LIGHT_GROUND, COLOR_LIGHT_GROUND_BG, '\u{A2}'),
        };
        if map[i].explored {
            // show explored tiles only (any visible tile is explored already)
            con.put_char_ex(x, y, glyph, fg, bg);
        }
    }
}

fn render_map_objects(world: &World, con: &mut impl console::Console) {
    let mut to_draw: Vec<_> = world
        .entity_indexes
        .values()
        .filter(|&indexes| {
            if let (Some(mo), Some(sy)) = (indexes.map_object, indexes.symbol) {
                let symbol = &world.symbols[sy];
                let index_in_map = (symbol.y * MAP_WIDTH + symbol.x) as usize;
                (world.map[index_in_map].in_fov && !world.map_objects[mo].hidden)
                    || (world.map[index_in_map].explored && world.map_objects[mo].always_visible)
            } else {
                false
            }
        })
        .collect();
    // sort so that non-blocking objects come first
    to_draw.sort_by(|&i1, &i2| {
        let (mi1, mi2) = (i1.map_object.unwrap(), i2.map_object.unwrap());
        world.map_objects[mi1]
            .block
            .cmp(&world.map_objects[mi2].block)
    });
    // draw the objects in the list
    for indexes in to_draw {
        let Symbol { x, y, char, color } = world.symbols[indexes.symbol.unwrap()];
        con.set_default_foreground(color);
        let char = indexes
            .character
            .and_then(|index| Some(&world.characters[index]))
            .filter(|&ch| ch.looking_right && ch.alive)
            .and(Some((char as u8 + 1) as char))
            .unwrap_or(char);
        con.put_char(x, y, char, console::BackgroundFlag::None);
    }
}

fn render_panel(world: &World, player_indexes: &EntityIndexes, con: &mut impl console::Console) {
    // prepare to render the GUI panel
    con.set_default_background(COLOR_DARKEST_GREY);
    con.clear();
    // print the game messages, one line at a time
    let mut y = MSG_HEIGHT;
    for &LogMessage(ref msg, color) in world.log.iter().rev() {
        let msg_height = con.get_height_rect(MSG_X, MSG_HEIGHT - y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        con.set_default_foreground(color);
        con.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }
    // show the player's stats
    let hp = world.characters[player_indexes.character.unwrap()].hp;
    let max_hp = max_hp(world.player.id, world);
    render_bar(
        con,
        1,
        2,
        BAR_WIDTH,
        "HP",
        hp,
        max_hp,
        COLOR_DARK_RED,
        COLOR_DARKER_SEPIA,
    );
    con.print_ex(
        1,
        1,
        console::BackgroundFlag::None,
        console::TextAlignment::Left,
        format!("Mine level: {}", world.player.dungeon_level),
    );
    // display names of objects under the mouse
    con.set_default_foreground(COLOR_LIGHTEST_GREY);
    con.print_rect(
        1,
        3,
        BAR_WIDTH,
        0,
        String::from("You see: ") + &get_names_under_mouse(world),
    );
}

fn render_main_menu_bg(con: &mut impl console::Console) {
    let img = tcod::image::Image::from_file("menu_background.png")
        .ok()
        .expect("Background image not found");
    tcod::image::blit_2x(&img, (0, 0), (-1, -1), con, (0, 0));
    con.set_default_foreground(COLOR_DARK_RED);
    con.print_ex(
        SCREEN_WIDTH / 2,
        SCREEN_HEIGHT / 2 - 4,
        console::BackgroundFlag::None,
        console::TextAlignment::Center,
        "EXPERIMENT 01: ABANDONED MINES",
    );
    con.print_ex(
        SCREEN_WIDTH / 2,
        SCREEN_HEIGHT - 2,
        console::BackgroundFlag::None,
        console::TextAlignment::Center,
        "by saintech",
    );
}

fn render_bar(
    panel: &mut impl console::Console,
    x: i32,
    y: i32,
    total_width: i32,
    name: &str,
    value: i32,
    maximum: i32,
    bar_color: colors::Color,
    back_color: colors::Color,
) {
    // render a bar (HP, experience, etc). First calculate the width of the bar
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;
    // render the background first
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, console::BackgroundFlag::Set);
    // now render the bar on top
    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, console::BackgroundFlag::Set);
    }
    // finally, some centered text with the values
    panel.set_default_foreground(COLOR_LIGHTEST_GREY);
    panel.print_ex(
        x + total_width / 2,
        y,
        console::BackgroundFlag::None,
        console::TextAlignment::Center,
        &format!("{}: {}/{}", name, value, maximum),
    );
}

fn render_dialogs(world: &World, destination_console: &mut impl console::Console) {
    for DialogBox {
        header,
        options,
        width,
        ..
    } in &world.dialogs
    {
        let keys = b"123456789abcdefghijklmnopqrstuvwxyz";
        assert!(
            options.len() <= 35,
            "Cannot have a menu with more than 35 options."
        );
        // calculate total height for the header (after auto-wrap) and one line per option
        let header_height = if header.is_empty() {
            -1
        } else {
            destination_console.get_height_rect(0, 0, width - 2, SCREEN_HEIGHT - 2, header)
        };
        let height = if options.len() > 0 {
            header_height + options.len() as i32 + 3
        } else {
            header_height + 2
        };
        // create an off-screen console that represents the menu's window
        let mut window = console::Offscreen::new(*width, height);
        window.set_default_background(COLOR_DARK_SKY);
        window.set_default_foreground(COLOR_DARKER_SEPIA);
        window.clear();
        // print the header, with auto-wrap
        window.print_rect(1, 1, width - 1, height, header);
        // print all the options
        for (index, option_text) in options.iter().enumerate() {
            let menu_letter = keys[index] as char;
            let text = format!("[{}] {}", menu_letter, option_text);
            window.print(1, header_height + 2 + index as i32, text);
        }
        let x = SCREEN_WIDTH / 2 - width / 2;
        let y = SCREEN_HEIGHT / 2 - height / 2;
        tcod::console::blit(
            &mut window,
            (0, 0),
            (*width, height),
            destination_console,
            (x, y),
            1.0,
            1.0,
        );
    }
}

fn compute_fov(world: &mut World, tcod: &mut Tcod) {
    let player_indexes = &world.entity_indexes[&world.player.id];
    let player_symbol = &world.symbols[player_indexes.symbol.unwrap()];
    if world.player.previous_player_position != (player_symbol.x, player_symbol.y) {
        tcod.fov.compute_fov(
            player_symbol.x,
            player_symbol.y,
            TORCH_RADIUS,
            FOV_LIGHT_WALLS,
            FOV_ALGO,
        );
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let index_in_map = (y * MAP_WIDTH + x) as usize;
                let in_fov = tcod.fov.is_in_fov(x, y);
                world.map[index_in_map].in_fov = in_fov;
                if in_fov {
                    world.map[index_in_map].explored = true;
                }
            }
        }
        world.player.previous_player_position = (player_symbol.x, player_symbol.y);
    }
}

// *** Mouse Look System ***
pub fn update_mouse_look_system(world: &mut World, _tcod: &mut Tcod) {
    if let PlayerAction::LookAt(lx, ly) = world.player.action {
        let ids: Vec<_> = world
            .entity_indexes
            .iter()
            .filter(|&(_, indexes)| {
                if let (Some(s), Some(mo)) = (indexes.symbol, indexes.map_object) {
                    let &Symbol { x, y, .. } = &world.symbols[s];
                    let &MapCell { in_fov, .. } = &world.map[(y * MAP_WIDTH + x) as usize];
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

/// return a string with the names of all objects under the mouse
fn get_names_under_mouse(world: &World) -> String {
    let mut names: Vec<_> = world
        .player
        .look_at
        .iter()
        .flatten()
        .filter_map(|id| world.entity_indexes.get(id))
        .map(|indexes| world.map_objects[indexes.map_object.unwrap()].name.clone())
        .collect();
    let max_len = world.player.look_at.len();
    match names.len() {
        0 => String::from("nothing out of the ordinary"),
        l if l == max_len => {
            names.truncate(max_len - 1);
            names.join(", ") + " and more..."
        }
        _ => names.join(", "),
    }
}

// *** FOV System ***
pub fn update_fov_state(world: &mut World, tcod: &mut Tcod) {
    let map_is_empty = world.map.len() == 0;
    let fov_is_empty = tcod.fov.size() == (1, 1);
    match (map_is_empty, fov_is_empty) {
        (true, false) => tcod.fov = tcod::map::Map::new(1, 1),
        (false, true) => {
            create_fov(world, tcod);
            compute_fov(world, tcod);
        }
        (false, false) => compute_fov(world, tcod),
        _ => (),
    }
}

fn is_main_menu(dialog_box: &&DialogBox) -> bool {
    dialog_box.kind == DialogKind::MainMenu
}

// *** Main Menu System ***
pub fn update_main_menu_state(world: &mut World, _tcod: &mut Tcod) {
    let world_is_empty = (world.id_count == 0) && world.entity_indexes.is_empty();
    let menu_is_open = world.dialogs.last().filter(is_main_menu).is_some();
    if world_is_empty {
        let choices: Vec<_> = ["Play a new game", "Continue last game", "Quit"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        add_dialog_box(world, DialogKind::MainMenu, String::from(""), choices, 24)
    } else if menu_is_open {
        world.player.state = PlayerState::InDialog;
        match world.player.action {
            // "Play a new game"
            PlayerAction::SelectMenuItem(0) => {
                *world = Default::default();
                world.player.state = PlayerState::MakingTurn;
            }
            // "Continue last game"
            PlayerAction::SelectMenuItem(1) => {
                if load_game(world).is_ok() {
                    world.player.state = PlayerState::MakingTurn;
                } else {
                    let msg = "\nNo saved game to load.\n";
                    add_dialog_box(world, DialogKind::MessageBox, String::from(msg), vec![], 24);
                }
            }
            // "Quit"
            PlayerAction::SelectMenuItem(2) | PlayerAction::Cancel => {
                world.must_be_destroyed = true;
            }
            _ => (),
        }
    }
}

fn is_exiting_to_main_menu(world: &World) -> bool {
    world.dialogs.is_empty()
        && (world.player.action == PlayerAction::Cancel)
        && (world.player.state == PlayerState::MakingTurn)
}

// *** Dungeon System ***
pub fn update_dungeon_state(world: &mut World, _tcod: &mut Tcod) {
    if world.map.is_empty() && (world.player.state == PlayerState::MakingTurn) {
        if world.entity_indexes.get(&world.player.id).is_none() {
            spawn_player(world);
        }
        make_map(world, world.player.dungeon_level);
        world.player.action = PlayerAction::None;
    } else if is_exiting_to_main_menu(world) {
        world.player.action = PlayerAction::None;
        save_game(world);
        *world = Default::default();
    }
}

fn load_game(world: &mut World) -> Result<(), Box<dyn Error>> {
    let mut json_save_state = String::new();
    let mut file = fs::File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let result = serde_json::from_str::<World>(&json_save_state)?;
    *world = result;
    Ok(())
}

fn is_message_box(dialog_box: &&DialogBox) -> bool {
    dialog_box.kind == DialogKind::MessageBox
}

// *** Message Box System ***
pub fn update_message_box_state(world: &mut World, _tcod: &mut Tcod) {
    let message_box_is_open = world.dialogs.last().filter(is_message_box).is_some();
    if message_box_is_open {
        world.player.state = PlayerState::InDialog;
        if world.player.action == PlayerAction::Cancel {
            world.dialogs.pop();
            if world.dialogs.is_empty() {
                world.player.state = PlayerState::MakingTurn;
            }
        }
    }
}
