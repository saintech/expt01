use rand::distributions::{Distribution as _, WeightedIndex};
use rand::Rng as _;
use serde::{Deserialize, Serialize};
use std::{cmp, collections::btree_map::BTreeMap, error::Error, fs, io::Read as _, io::Write as _};
use tcod::{colors, console, input, Console as _};


/***********************
 *       Config        *
 ***********************/

// actual size of the window
const SCREEN_WIDTH: i32 = 68;
const SCREEN_HEIGHT: i32 = 36;

// size of the map
const MAP_WIDTH: i32 = 68;
const MAP_HEIGHT: i32 = 30;

// sizes and coordinates relevant for the GUI
const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT + 1;
const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 3;
const MSG_HEIGHT: i32 = PANEL_HEIGHT - 1;
const INVENTORY_WIDTH: i32 = 40;
const CHARACTER_SCREEN_WIDTH: i32 = 30;
const LEVEL_SCREEN_WIDTH: i32 = 40;

//parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: usize = 30;

const HEAL_AMOUNT: i32 = 40;
const SLINGSHOT_DAMAGE: i32 = 40;
const SLINGSHOT_RANGE: i32 = 5;
const BRICK_RANGE: i32 = 8;
const BRICK_NUM_TURNS: i32 = 10;
const BLASTING_RADIUS: i32 = 3;
const BLASTING_DAMAGE: i32 = 25;

// experience and level-ups
const LEVEL_UP_BASE: i32 = 200;
const LEVEL_UP_FACTOR: i32 = 150;

const FOV_ALGO: tcod::map::FovAlgorithm = tcod::map::FovAlgorithm::Diamond;
// default FOV algorithm
const FOV_LIGHT_WALLS: bool = true;
// light walls or not
const TORCH_RADIUS: i32 = 10;

const GROUND_BITMAP: usize = 0b100010000101000001010001000000001000101000001010000100010000;

const LIMIT_FPS: i32 = 20;

// colors:
const COLOR_LIGHTEST_GREY: colors::Color = colors::Color::new(192, 209, 204);
const COLOR_DARKEST_GREY: colors::Color = colors::Color::new(20, 24, 23);
//const COLOR_LIGHT_SEPIA: colors::Color = colors::Color::new(164, 166, 153);
const COLOR_SEPIA: colors::Color = colors::Color::new(129, 122, 119);
const COLOR_DARK_SEPIA: colors::Color = colors::Color::new(92, 87, 82);
const COLOR_DARKER_SEPIA: colors::Color = colors::Color::new(53, 50, 56);
//const COLOR_LIGHT_SKY: colors::Color = colors::Color::new(165, 195, 214);
//const COLOR_SKY: colors::Color = colors::Color::new(134, 162, 176);
const COLOR_DARK_SKY: colors::Color = colors::Color::new(104, 127, 139);
const COLOR_GREEN: colors::Color = colors::Color::new(79, 119, 84);
const COLOR_DARK_RED: colors::Color = colors::Color::new(127, 78, 77);
const COLOR_ORANGE: colors::Color = colors::Color::new(155, 107, 77);

const COLOR_DARK_WALL: colors::Color = COLOR_DARK_SEPIA;
const COLOR_DARK_WALL_BG: colors::Color = COLOR_DARKER_SEPIA;
const COLOR_LIGHT_WALL: colors::Color = COLOR_SEPIA;
const COLOR_LIGHT_WALL_BG: colors::Color = COLOR_DARKER_SEPIA;
const COLOR_DARK_GROUND: colors::Color = COLOR_DARKER_SEPIA;
const COLOR_DARK_GROUND_BG: colors::Color = COLOR_DARKER_SEPIA;
const COLOR_LIGHT_GROUND: colors::Color = COLOR_DARK_SEPIA;
const COLOR_LIGHT_GROUND_BG: colors::Color = COLOR_DARKER_SEPIA;

// player will always be the first object
const PLAYER_ID: u32 = 1;


/***********************
 *     Components      *
 ***********************/

#[derive(Debug, Serialize, Deserialize, Default)]
struct Player {
    dungeon_level: u32,
    action: PlayerAction,
    previous_player_position: (i32, i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum PlayerAction {
    StartGame,
    Exit,
    TookTurn,
    DidntTakeTurn,
}

impl Default for PlayerAction {
    fn default() -> Self {
        PlayerAction::StartGame
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Symbol {
    x: i32,
    y: i32,
    char: char,
    color: colors::Color,
}

/// A tile of the map and its properties
#[derive(Debug, Serialize, Deserialize)]
struct MapCell {
    block: bool,
    explored: bool,
    block_sight: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct MapObject {
    name: String,
    block: bool,
    always_visible: bool,
    hidden: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Character {
    alive: bool,
    level: i32,
    hp: i32,
    base_max_hp: i32,
    base_defense: i32,
    base_power: i32,
    xp: i32,
    on_death: DeathCallback,
    looking_right: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum DeathCallback {
    None,
    Player,
    Monster,
}

#[derive(Debug, Serialize, Deserialize)]
enum Ai {
    Basic,
    Confused {
        // TODO: fix this unsized stuff
        previous_ai: Box<Ai>,
        num_turns: i32,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct AiOption {
    option: Option<Ai>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum Item {
    Medkit,
    SlingshotAmmo,
    Brick,
    BlastingCartridge,
    Melee,
    Clothing,
}

#[derive(Debug, Serialize, Deserialize)]
struct OwnedItem {
    item: Item,
    owner: u32,
}

/// An object that can be equipped, yielding bonuses.
#[derive(Debug, Serialize, Deserialize)]
struct Equipment {
    slot: Slot,
    equipped: bool,
    max_hp_bonus: i32,
    defense_bonus: i32,
    power_bonus: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum Slot {
    Body,
    Hands,
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Slot::Body => write!(f, "body"),
            Slot::Hands => write!(f, "hands"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LogMessage(String, colors::Color);


/***********************
 *        Game         *
 ***********************/

#[derive(Debug, Serialize, Deserialize)]
struct EntityIndexes {
    symbol: Option<usize>,
    map_cell: Option<usize>,
    map_object: Option<usize>,
    character: Option<usize>,
    ai: Option<usize>,
    item: Option<usize>,
    equipment: Option<usize>,
    log_message: Option<usize>,
}

struct Tcod {
    root: console::Root,
    con: console::Offscreen,
    panel: console::Offscreen,
    fov: tcod::map::Map,
    key: input::Key,
    mouse: input::Mouse,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct World {
    id_count: u32,
    entity_indexes: BTreeMap<u32, EntityIndexes>,
    player: Player,
    symbols: Vec<Symbol>,
    map: Vec<MapCell>,
    map_objects: Vec<MapObject>,
    characters: Vec<Character>,
    ais: Vec<AiOption>,
    items: Vec<OwnedItem>,
    equipments: Vec<Equipment>,
    log: Vec<LogMessage>,
}

impl World {
    fn create_entity(
        &mut self,
        symbol: Option<Symbol>,
        map_cell: Option<MapCell>,
        map_object: Option<MapObject>,
        character: Option<Character>,
        ai: Option<AiOption>,
        item: Option<OwnedItem>,
        equipment: Option<Equipment>,
        log_message: Option<LogMessage>,
    ) -> u32 {
        let entity_indexes = EntityIndexes {
            symbol: symbol.as_ref().map(|_| self.symbols.len()),
            map_cell: map_cell.as_ref().map(|_| self.map.len()),
            map_object: map_object.as_ref().map(|_| self.map_objects.len()),
            character: character.as_ref().map(|_| self.characters.len()),
            ai: ai.as_ref().map(|_| self.ais.len()),
            item: item.as_ref().map(|_| self.items.len()),
            equipment: equipment.as_ref().map(|_| self.equipments.len()),
            log_message: log_message.as_ref().map(|_| self.log.len()),
        };
        symbol.map(|c| self.symbols.push(c));
        map_cell.map(|c| self.map.push(c));
        map_object.map(|c| self.map_objects.push(c));
        character.map(|c| self.characters.push(c));
        ai.map(|c| self.ais.push(c));
        item.map(|c| self.items.push(c));
        equipment.map(|c| self.equipments.push(c));
        log_message.map(|c| self.log.push(c));
        self.id_count += 1;
        self.entity_indexes.insert(self.id_count, entity_indexes);
        self.id_count
    }
}

fn add_log(world: &mut World, message: impl Into<String>, color: colors::Color) {
    world.create_entity(
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(LogMessage(message.into(), color)),
    );
}

/// return the distance to another object
fn distance_to(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> f32 {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
}

fn take_damage(target: &mut Character, damage: i32) -> Option<i32> {
    // apply damage if possible
    if damage > 0 {
        target.hp -= damage;
    }
    // check for death, call the death function
    if target.hp <= 0 {
        target.alive = false;
        //fighter.on_death.callback(self, game);
        Some(target.xp)
    } else {
        None
    }
}

fn attack_by(attacker_id: u32, target_id: u32, world: &mut World) {
    let attacker_object =
        &world.map_objects[world.entity_indexes[&attacker_id].map_object.unwrap()];
    let attacker_name = attacker_object.name.clone();
    let target_object = &world.map_objects[world.entity_indexes[&target_id].map_object.unwrap()];
    let target_name = target_object.name.clone();
    // a simple formula for attack damage
    let damage = power(attacker_id, world) - defense(target_id, world);
    if damage > 0 {
        add_log(
            world,
            format!(
                "{} attacks {} for {} hit points.",
                attacker_name, target_name, damage
            ),
            COLOR_LIGHTEST_GREY,
        );
        let attacker_index = world.entity_indexes[&attacker_id].character.unwrap();
        let target_index = world.entity_indexes[&target_id].character.unwrap();
        if let Some(xp) = take_damage(&mut world.characters[target_index], damage) {
            // yield experience to the player
            world.characters[attacker_index].xp += xp;
        }
    } else {
        add_log(
            world,
            format!(
                "{} attacks {} but it has no effect!",
                attacker_name, target_name
            ),
            COLOR_LIGHTEST_GREY,
        );
    }
}

/// heal by the given amount, without going over the maximum
fn heal(id: u32, amount: i32, world: &mut World) {
    let max_hp = max_hp(id, world);
    let player_indexes = &world.entity_indexes[&id];
    let character = &mut world.characters[player_indexes.character.unwrap()];
    character.hp += amount;
    if character.hp > max_hp {
        character.hp = max_hp;
    }
}

/// Equip object and show a message about it
fn equip(id: u32, world: &mut World) {
    let indexes = &world.entity_indexes[&id];
    let name = world.map_objects[indexes.map_object.unwrap()].name.clone();
    if let Some(index) = indexes.equipment {
        if !world.equipments[index].equipped {
            world.equipments[index].equipped = true;
            add_log(
                world,
                format!("Equipped {} on {}.", name, world.equipments[index].slot),
                COLOR_GREEN,
            );
        }
    } else {
        add_log(
            world,
            format!("Can't equip {} because it's not an Equipment.", name),
            COLOR_ORANGE,
        );
    }
}

/// Dequip object and show a message about it
fn dequip(id: u32, world: &mut World) {
    let indexes = &world.entity_indexes[&id];
    let name = world.map_objects[indexes.map_object.unwrap()].name.clone();
    if let Some(index) = indexes.equipment {
        if world.equipments[index].equipped {
            world.equipments[index].equipped = false;
            add_log(
                world,
                format!("Dequipped {} from {}.", name, world.equipments[index].slot),
                COLOR_DARK_SKY,
            );
        }
    } else {
        add_log(
            world,
            format!("Can't dequip {} because it's not an Equipment.", name),
            COLOR_ORANGE,
        );
    }
}

/// returns a list of equipped items
fn get_all_equipped(owner: u32, world: &World) -> impl Iterator<Item=&Equipment> {
    world.entity_indexes.values().filter_map(move |indexes| {
        indexes
            .item
            .filter(|&it| world.items[it].owner == owner)
            .and(indexes.equipment)
            .filter(|&eq| world.equipments[eq].equipped)
            .map(|eq| &world.equipments[eq])
    })
}

fn power(id: u32, world: &World) -> i32 {
    let base_power = world.entity_indexes[&id]
        .character
        .map_or(0, |ch| world.characters[ch].base_power);
    let bonus: i32 = get_all_equipped(id, world).map(|eq| eq.power_bonus).sum();
    base_power + bonus
}

fn defense(id: u32, world: &World) -> i32 {
    let base_defense = world.entity_indexes[&id]
        .character
        .map_or(0, |ch| world.characters[ch].base_defense);
    let bonus: i32 = get_all_equipped(id, world).map(|eq| eq.defense_bonus).sum();
    base_defense + bonus
}

fn max_hp(id: u32, world: &World) -> i32 {
    let base_max_hp = world.entity_indexes[&id]
        .character
        .map_or(0, |ch| world.characters[ch].base_max_hp);
    let bonus: i32 = get_all_equipped(id, world).map(|eq| eq.max_hp_bonus).sum();
    base_max_hp + bonus
}

/// move by the given amount, if the destination is not blocked
fn move_by(id: u32, dx: i32, dy: i32, world: &mut World) {
    let indexes = &world.entity_indexes[&id];
    let symbol = &world.symbols[indexes.symbol.unwrap()];
    let &Symbol { x, y, .. } = symbol;
    if !is_blocked(x + dx, y + dy, world) {
        world.symbols[indexes.symbol.unwrap()].x = x + dx;
        world.symbols[indexes.symbol.unwrap()].y = y + dy;
    }
}

fn get_equipped_in_slot(slot: Slot, world: &mut World) -> Option<u32> {
    world.entity_indexes.iter().find_map(|(&id, indexes)| {
        indexes
            .equipment
            .filter(|&eq| world.equipments[eq].equipped && world.equipments[eq].slot == slot)
            .and(Some(id))
    })
}

fn is_blocked(x: i32, y: i32, world: &World) -> bool {
    let index_in_map = (y * MAP_WIDTH + x) as usize;
    // first test the map tile
    if world.map[index_in_map].block {
        return true;
    }
    // now check for any blocking objects
    world.entity_indexes.values().any(|indexes| {
        if let (Some(sy), Some(mo)) = (indexes.symbol, indexes.map_object) {
            return world.map_objects[mo].block
                && (world.symbols[sy].x, world.symbols[sy].y) == (x, y);
        };
        false
    })
}

/// Advance to the next level
fn next_level(world: &mut World, tcod: &mut Tcod) {
    clear_map(world);
    add_log(
        world,
        "You take a moment to rest, and recover your strength.",
        COLOR_GREEN,
    );
    let heal_hp = max_hp(PLAYER_ID, world) / 2;
    heal(PLAYER_ID, heal_hp, world);
    add_log(
        world,
        "After a rare moment of peace, you descend deeper into \
         the heart of the mine...",
        COLOR_ORANGE,
    );
    world.player.dungeon_level += 1;
    make_map(world, world.player.dungeon_level);
    initialise_fov(world, tcod);
    tcod.con.clear();
}

fn clear_map(world: &mut World) {
    // create new world for storing entities that should be saved
    let mut temp_world = World::default();
    // copy player character
    spawn_player(&mut temp_world);
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    let player = &mut world.characters[player_indexes.character.unwrap()];
    let temp_player_indexes = &temp_world.entity_indexes[&PLAYER_ID];
    let temp_player = &mut temp_world.characters[temp_player_indexes.character.unwrap()];
    temp_player.level = player.level;
    temp_player.hp = player.hp;
    temp_player.base_max_hp = player.base_max_hp;
    temp_player.base_defense = player.base_defense;
    temp_player.base_power = player.base_power;
    temp_player.xp = player.xp;
    // copy inventory
    let inventory = world.entity_indexes.iter().filter(|&(_, indexes)| {
        indexes
            .item
            .filter(|&it| world.items[it].owner == PLAYER_ID)
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
            owner: world.items[indexes.item.unwrap()].owner,
        };
        let equipment = indexes.equipment.map(|index| Equipment {
            slot: world.equipments[index].slot,
            equipped: world.equipments[index].equipped,
            max_hp_bonus: world.equipments[index].max_hp_bonus,
            defense_bonus: world.equipments[index].defense_bonus,
            power_bonus: world.equipments[index].power_bonus,
        });
        let entity_indexes = EntityIndexes {
            symbol: Some(temp_world.symbols.len()),
            map_cell: None,
            map_object: Some(temp_world.map_objects.len()),
            character: None,
            ai: None,
            item: Some(temp_world.items.len()),
            equipment: indexes.equipment.map(|_| temp_world.equipments.len()),
            log_message: None,
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
        let entity_indexes = EntityIndexes {
            symbol: None,
            map_cell: None,
            map_object: None,
            character: None,
            ai: None,
            item: None,
            equipment: None,
            log_message: Some(temp_world.log.len()),
        };
        temp_world.log.push(log_message);
        temp_world.entity_indexes.insert(id, entity_indexes);
    }
    // replace world data
    world.entity_indexes = temp_world.entity_indexes;
    world.symbols = temp_world.symbols;
    world.map = temp_world.map;
    world.map_objects = temp_world.map_objects;
    world.characters = temp_world.characters;
    world.ais = temp_world.ais;
    world.items = temp_world.items;
    world.equipments = temp_world.equipments;
    world.log = temp_world.log;
}

/// A rectangle on the map, used to characterise a room.
#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }
    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }
    pub fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}

fn make_map(world: &mut World, level: u32) {
    fill_walls(world);
    let mut rooms = vec![];
    if level == 1 {
        place_hints(world, &mut rooms);
    }
    for _ in rooms.len()..MAX_ROOMS {
        // random width and height:
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        // random position without going out of the boundaries of the map
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);
        let new_room = Rect::new(x, y, w, h);
        // run through the other rooms and see if they intersect with this one
        let failed = rooms
            .iter()
            .any(|other_room| new_room.intersects_with(other_room));
        if !failed {
            // this means there are no intersections, so this room is valid
            // "paint" it to the map's tiles
            create_room(new_room, &mut world.map);
            // add some content to this room, such as monsters
            place_objects(new_room, world, level);
            // center coordinates of the new room, will be useful later
            let (new_x, new_y) = new_room.center();
            if rooms.is_empty() {
                // this is the first room, where the player starts at
                let player_indexes = &world.entity_indexes[&PLAYER_ID];
                world.symbols[player_indexes.symbol.unwrap()].x = new_x;
                world.symbols[player_indexes.symbol.unwrap()].y = new_y;
            } else {
                // all rooms after the first: connect it to the previous room with a tunnel
                // center coordinates of the previous room
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();
                // toss a coin (random bool value -- either true or false)
                if rand::random() {
                    // first move horizontally, then vertically
                    create_h_tunnel(prev_x, new_x, prev_y, &mut world.map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut world.map);
                } else {
                    // first move vertically, then horizontally
                    create_v_tunnel(prev_y, new_y, prev_x, &mut world.map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut world.map);
                }
            }
            // finally, append the new room to the list
            rooms.push(new_room);
        }
    }
    // create stairs at the center of the last room
    let (last_room_x, last_room_y) = rooms[rooms.len() - 1].center();
    spawn_stairs(world, last_room_x, last_room_y);
}

fn fill_walls(world: &mut World) {
    for _ in 0..MAP_WIDTH * MAP_HEIGHT {
        world.create_entity(
            None,
            Some(MapCell {
                block: true,
                explored: false,
                block_sight: true,
            }),
            None,
            None,
            None,
            None,
            None,
            None,
        );
    }
}

fn place_hints(world: &mut World, rooms: &mut Vec<Rect>) {
    let x = rand::thread_rng().gen_range(0, MAP_WIDTH - 6);
    let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - 6);
    let new_room = Rect::new(x, y, 6, 6);
    create_room(new_room, &mut world.map);
    let map_object = MapObject {
        name: String::new(),
        block: false,
        always_visible: false,
        hidden: false,
    };
    let name = "move hint";
    let color = COLOR_LIGHT_GROUND;
    let hints = vec![
        (x + 2, y + 2, '\u{14}'),
        (x + 2, y + 4, '\u{15}'),
        (x + 4, y + 2, '\u{16}'),
        (x + 4, y + 4, '\u{17}'),
        (x + 3, y + 2, '\u{18}'),
        (x + 3, y + 4, '\u{19}'),
        (x + 4, y + 3, '\u{1A}'),
        (x + 2, y + 3, '\u{1B}'),
    ];
    hints.iter().for_each(|&(x, y, char)| {
        world.create_entity(
            Some(Symbol { x, y, char, color }),
            None,
            Some(MapObject { name: name.to_string(), ..map_object }),
            None,
            None,
            None,
            None,
            None,
        );
    });
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    world.symbols[player_indexes.symbol.unwrap()].x = x + 3;
    world.symbols[player_indexes.symbol.unwrap()].y = y + 3;
    rooms.push(new_room);
}

fn create_room(room: Rect, map: &mut Vec<MapCell>) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            let index_in_map = (y * MAP_WIDTH + x) as usize;
            map[index_in_map].block = false;
            map[index_in_map].block_sight = false;
        }
    }
}

fn place_objects(room: Rect, world: &mut World, level: u32) {
    let mut rng = rand::thread_rng();
    // maxumum number of monsters per room
    let max_monsters = from_dungeon_level(
        &[
            Transition { level: 1, value: 2 },
            Transition { level: 4, value: 3 },
            Transition { level: 6, value: 5 },
        ],
        level,
    );
    // choose random number of monsters
    let num_monsters = rng.gen_range(0, max_monsters + 1);
    // monster random table
    let rat_chance = from_dungeon_level(
        &[
            Transition {
                level: 3,
                value: 15,
            },
            Transition {
                level: 5,
                value: 30,
            },
            Transition {
                level: 7,
                value: 60,
            },
        ],
        level,
    );
    let monsters = ["roach", "rat"];
    let monster_chances = &[80, rat_chance];
    let monster_choice = WeightedIndex::new(monster_chances).unwrap();
    // maximum number of items per room
    let max_items = from_dungeon_level(
        &[
            Transition { level: 1, value: 1 },
            Transition { level: 4, value: 2 },
        ],
        level,
    );
    let items = [
        Item::Medkit,
        Item::SlingshotAmmo,
        Item::BlastingCartridge,
        Item::Brick,
        Item::Melee,
        Item::Clothing,
    ];
    // item random table
    let item_chances = &[
        35,
        from_dungeon_level(
            &[Transition {
                level: 4,
                value: 25,
            }],
            level,
        ),
        from_dungeon_level(
            &[Transition {
                level: 6,
                value: 25,
            }],
            level,
        ),
        from_dungeon_level(
            &[Transition {
                level: 2,
                value: 10,
            }],
            level,
        ),
        from_dungeon_level(&[Transition { level: 4, value: 5 }], level),
        from_dungeon_level(
            &[Transition {
                level: 8,
                value: 15,
            }],
            level,
        ),
    ];
    let item_choice = WeightedIndex::new(item_chances).unwrap();
    for _ in 0..num_monsters {
        // choose random spot for this monster
        let x = rng.gen_range(room.x1 + 1, room.x2);
        let y = rng.gen_range(room.y1 + 1, room.y2);
        // only place it if the tile is not blocked
        if !is_blocked(x, y, world) {
            let (name, sy, ch, ai) = match monsters[monster_choice.sample(&mut rng)] {
                "roach" => (
                    "Roach",
                    Symbol {
                        x,
                        y,
                        char: '\u{82}',
                        color: COLOR_ORANGE,
                    },
                    Character {
                        alive: true,
                        level: 1,
                        base_max_hp: 20,
                        hp: 20,
                        base_defense: 0,
                        base_power: 4,
                        xp: 35,
                        on_death: DeathCallback::Monster,
                        looking_right: false,
                    },
                    Ai::Basic,
                ),
                "rat" => (
                    "Rat",
                    Symbol {
                        x,
                        y,
                        char: '\u{84}',
                        color: COLOR_DARK_SKY,
                    },
                    Character {
                        alive: true,
                        level: 1,
                        base_max_hp: 30,
                        hp: 30,
                        base_defense: 2,
                        base_power: 8,
                        xp: 100,
                        on_death: DeathCallback::Monster,
                        looking_right: false,
                    },
                    Ai::Basic,
                ),
                _ => unreachable!(),
            };
            spawn_monster(world, name, sy, ch, ai);
        }
    }
    // choose random number of items
    let num_items = rng.gen_range(0, max_items + 1);
    for _ in 0..num_items {
        // choose random spot for this item
        let x = rng.gen_range(room.x1 + 1, room.x2);
        let y = rng.gen_range(room.y1 + 1, room.y2);
        // only place it if the tile is not blocked
        if !is_blocked(x, y, world) {
            spawn_item(world, items[item_choice.sample(&mut rng)], 0, x, y);
        }
    }
}

struct Transition {
    level: u32,
    value: u32,
}

/// Returns a value that depends on level. the table specifies what
/// value occurs after each level, default is 0.
fn from_dungeon_level(table: &[Transition], level: u32) -> u32 {
    table
        .iter()
        .rev()
        .find(|transition| level >= transition.level)
        .map_or(0, |transition| transition.value)
}

fn spawn_player(world: &mut World) {
    let x = SCREEN_WIDTH / 2;
    let y = SCREEN_HEIGHT / 2;
    let char = '\u{80}';
    let color = COLOR_GREEN;
    let name = String::from("Player");
    let block = true;
    let always_visible = false;
    let hidden = false;
    let alive = true;
    let level = 1;
    let hp = 30;
    let base_max_hp = 30;
    let base_defense = 1;
    let base_power = 2;
    let xp = 0;
    let on_death = DeathCallback::Player;
    let looking_right = false;
    let player_id = world.create_entity(
        Some(Symbol { x, y, char, color }),
        None,
        Some(MapObject {
            name,
            block,
            always_visible,
            hidden,
        }),
        Some(Character {
            alive,
            level,
            hp,
            base_max_hp,
            base_defense,
            base_power,
            xp,
            on_death,
            looking_right,
        }),
        None,
        None,
        None,
        None,
    );
    world.player.action = PlayerAction::DidntTakeTurn;
    if world.player.dungeon_level == 0 {
        world.player.dungeon_level = 1;
    }
    assert_eq!(
        PLAYER_ID, player_id,
        "the player must be the first entity with ID 1"
    );
}

fn spawn_monster(world: &mut World, name: &str, symbol: Symbol, character: Character, ai: Ai) {
    let name = String::from(name);
    let block = true;
    let always_visible = false;
    let hidden = false;
    world.create_entity(
        Some(symbol),
        None,
        Some(MapObject {
            name,
            block,
            always_visible,
            hidden,
        }),
        Some(character),
        Some(AiOption { option: Some(ai) }),
        None,
        None,
        None,
    );
}

fn spawn_item(world: &mut World, item: Item, owner: u32, x: i32, y: i32) -> u32 {
    let (char, name, color, equipment) = match item {
        Item::Medkit => ('\u{90}', "Medkit", COLOR_DARK_RED, None),
        Item::SlingshotAmmo => ('\u{91}', "Bullet For Slingshot", COLOR_DARK_SEPIA, None),
        Item::BlastingCartridge => ('\u{92}', "Blasting Cartridge", COLOR_DARK_SEPIA, None),
        Item::Brick => ('\u{93}', "Brick", COLOR_DARK_SEPIA, None),
        Item::Melee => (
            '\u{95}',
            "Pickaxe",
            COLOR_DARK_SKY,
            Some(Equipment {
                equipped: false,
                slot: Slot::Hands,
                max_hp_bonus: 0,
                defense_bonus: 0,
                power_bonus: 3,
            }),
        ),
        Item::Clothing => (
            '\u{96}',
            "Workwear",
            COLOR_DARK_SKY,
            Some(Equipment {
                equipped: false,
                slot: Slot::Body,
                max_hp_bonus: 0,
                defense_bonus: 1,
                power_bonus: 0,
            }),
        ),
    };
    let name = String::from(name);
    let block = false;
    let always_visible = false;
    let hidden = false;
    world.create_entity(
        Some(Symbol { x, y, char, color }),
        None,
        Some(MapObject {
            name,
            block,
            always_visible,
            hidden,
        }),
        None,
        None,
        Some(OwnedItem { item, owner }),
        equipment,
        None,
    )
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Vec<MapCell>) {
    for x in cmp::min(x1, x2)..=cmp::max(x1, x2) {
        let index_in_map = (y * MAP_WIDTH + x) as usize;
        map[index_in_map].block = false;
        map[index_in_map].block_sight = false;
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Vec<MapCell>) {
    for y in cmp::min(y1, y2)..=cmp::max(y1, y2) {
        let index_in_map = (y * MAP_WIDTH + x) as usize;
        map[index_in_map].block = false;
        map[index_in_map].block_sight = false;
    }
}

fn spawn_stairs(world: &mut World, x: i32, y: i32) {
    let char = '\u{A4}';
    let color = COLOR_LIGHT_WALL;
    let map_object = MapObject {
        name: String::from("stairs"),
        block: false,
        always_visible: true,
        hidden: false,
    };
    world.create_entity(
        Some(Symbol { x, y, char, color }),
        None,
        Some(map_object),
        None,
        None,
        None,
        None,
        None,
    );
}

fn menu(
    header: &str,
    options: &[impl AsRef<str>],
    width: i32,
    root: &mut console::Root,
) -> Option<usize> {
    let keys = b"123456789abcdefghijklmnopqrstuvwxyz";
    assert!(
        options.len() <= 35,
        "Cannot have a menu with more than 35 options."
    );
    // calculate total height for the header (after auto-wrap) and one line per option
    let header_height = if header.is_empty() {
        -1
    } else {
        root.get_height_rect(0, 0, width - 2, SCREEN_HEIGHT - 2, header)
    };
    let height = if options.len() > 0 {
        header_height + options.len() as i32 + 3
    } else {
        header_height + 2
    };
    // create an off-screen console that represents the menu's window
    let mut window = console::Offscreen::new(width, height);
    window.set_default_background(COLOR_DARK_SKY);
    window.set_default_foreground(COLOR_DARKER_SEPIA);
    window.clear();
    // print the header, with auto-wrap
    window.print_rect(1, 1, width - 1, height, header);
    // print all the options
    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = keys[index] as char;
        let text = format!("[{}] {}", menu_letter, option_text.as_ref());
        window.print(1, header_height + 2 + index as i32, text);
    }
    // blit the contents of "window" to the root console
    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    tcod::console::blit(&mut window, (0, 0), (width, height), root, (x, y), 1.0, 1.0);
    // present the root console to the player and wait for a key-press
    root.flush();
    let key = root.wait_for_keypress(true);
    // convert the ASCII code to an index; if it corresponds to an option, return it
    keys[0..options.len()]
        .iter()
        .position(|&val| val as char == key.printable.to_ascii_lowercase())
}

fn inventory_menu(world: &mut World, header: &str, root: &mut console::Root) -> Option<u32> {
    let inventory: Vec<_> = world
        .entity_indexes
        .iter()
        .filter_map(|(&id, indexes)| {
            indexes
                .item
                .filter(|&it| world.items[it].owner == PLAYER_ID)
                .and(Some(id))
        })
        .collect();
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
    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);
    if inventory.len() > 0 {
        inventory_index.map(|i| inventory[i])
    } else {
        None
    }
}

fn msgbox(text: &str, width: i32, root: &mut console::Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}

/// create the FOV map, according to the generated map
fn initialise_fov(world: &mut World, tcod: &mut Tcod) {
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let index_in_map = (y * MAP_WIDTH + x) as usize;
            tcod.fov.set(
                x,
                y,
                !world.map[index_in_map].block_sight,
                !world.map[index_in_map].block,
            );
        }
    }
}


/***********************
 *       Systems       *
 ***********************/

// *** Input System ***
fn update_input_state(_world: &mut World, tcod: &mut Tcod) {
    tcod.key = Default::default();
    match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
        Some((_, input::Event::Key(k))) => tcod.key = k,
        Some((_, input::Event::Mouse(m))) => tcod.mouse = m,
        _ => (),
    }
}

// *** Map Interaction System ***
fn update_map_interaction_state(world: &mut World, tcod: &mut Tcod) {
    if world.player.action == PlayerAction::StartGame {
        return;
    }
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    let player_character = &world.characters[player_indexes.character.unwrap()];
    if (tcod.key.code != input::KeyCode::Enter) || !player_character.alive {
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
        next_level(world, tcod);
    };
}

/// add to the player's inventory and remove from the map
fn pick_item_up(object_id: u32, world: &mut World) {
    let indexes = &world.entity_indexes[&object_id];
    let name = &world.map_objects[indexes.map_object.unwrap()].name.clone();
    let inventory_len = world
        .items
        .iter()
        .filter(|&item| item.owner == PLAYER_ID)
        .count();
    if inventory_len >= 35 {
        add_log(
            world,
            format!("Your inventory is full, cannot pick up {}.", name),
            COLOR_DARK_RED,
        );
    } else {
        world.items[indexes.item.unwrap()].owner = PLAYER_ID;
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
fn player_move_or_attack(world: &mut World, tcod: &mut Tcod) {
    if world.player.action == PlayerAction::StartGame {
        return;
    }
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    let player_character = &world.characters[player_indexes.character.unwrap()];
    if (tcod.key.code == input::KeyCode::NumPad5) && player_character.alive {
        world.player.action = PlayerAction::TookTurn;
        return;
    }
    let (dx, dy) = key_to_delta(tcod.key);
    if ((dx, dy) == (0, 0)) || !player_character.alive {
        world.player.action = PlayerAction::DidntTakeTurn;
        return;
    }
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
            attack_by(PLAYER_ID, target_id, world);
        }
        None => {
            move_by(PLAYER_ID, dx, dy, world);
        }
    }
    world.player.action = PlayerAction::TookTurn;
}

fn key_to_delta(key: input::Key) -> (i32, i32) {
    use input::Key;
    use input::KeyCode::*;
    match key {
        Key { code: Up, .. } | Key { code: NumPad8, .. } => (0, -1),
        Key { code: Down, .. } | Key { code: NumPad2, .. } => (0, 1),
        Key { code: Left, .. } | Key { code: NumPad4, .. } => (-1, 0),
        Key { code: Right, .. } | Key { code: NumPad6, .. } => (1, 0),
        Key { code: Home, .. } | Key { code: NumPad7, .. } => (-1, -1),
        Key { code: PageUp, .. } | Key { code: NumPad9, .. } => (1, -1),
        Key { code: End, .. } | Key { code: NumPad1, .. } => (-1, 1),
        Key { code: PageDown, .. } | Key { code: NumPad3, .. } => (1, 1),
        _ => (0, 0),
    }
}

// *** AI System ***
fn update_ai_turn_state(world: &mut World, tcod: &mut Tcod) {
    if world.player.action == PlayerAction::StartGame {
        return;
    }
    // let monsters take their turn
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    let player = &world.characters[player_indexes.character.unwrap()];
    if player.alive && world.player.action == PlayerAction::TookTurn {
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
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
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
            attack_by(monster_id, PLAYER_ID, world);
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
    let monster_name = world.map_objects[monster_indexes.map_object.unwrap()].name.clone();
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

// *** Drop Action System ***
fn update_drop_action_state(world: &mut World, tcod: &mut Tcod) {
    if world.player.action == PlayerAction::StartGame {
        return;
    }
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    let player = &world.characters[player_indexes.character.unwrap()];
    if (tcod.key.printable != 'd') || !player.alive {
        return;
    }
    let inventory_id = inventory_menu(
        world,
        "Press the key next to an item to drop it, or any other to cancel.'",
        &mut tcod.root,
    );
    if let Some(inventory_id) = inventory_id {
        drop_item(inventory_id, world);
    }
}

fn drop_item(inventory_id: u32, world: &mut World) {
    if world.entity_indexes[&inventory_id].equipment.is_some() {
        dequip(inventory_id, world);
    }
    let indexes = &world.entity_indexes[&inventory_id];
    world.items[indexes.item.unwrap()].owner = 0;
    world.map_objects[indexes.map_object.unwrap()].hidden = false;
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    let player_x = world.symbols[player_indexes.symbol.unwrap()].x;
    let player_y = world.symbols[player_indexes.symbol.unwrap()].y;
    let symbol = &mut world.symbols[indexes.symbol.unwrap()];
    symbol.x = player_x;
    symbol.y = player_y;
    let name = &world.map_objects[indexes.map_object.unwrap()].name.clone();
    add_log(world, format!("You dropped a {}.", name), COLOR_DARK_SKY);
}

// *** Inventory System ***
fn update_inventory_state(world: &mut World, tcod: &mut Tcod) {
    if world.player.action == PlayerAction::StartGame {
        return;
    }
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    let player_character = &world.characters[player_indexes.character.unwrap()];
    if (tcod.key.printable != 'i') || !player_character.alive {
        return;
    }
    // show the inventory: if an item is selected, use it
    let inventory_id = inventory_menu(
        world,
        "Press the key next to an item to use it, or any other to cancel.",
        &mut tcod.root,
    );
    if let Some(inventory_id) = inventory_id {
        use_item(inventory_id, world, tcod);
    }
    world.player.action = PlayerAction::TookTurn;
}

enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}

fn use_item(inventory_id: u32, world: &mut World, tcod: &mut Tcod) {
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
        match on_use(inventory_id, world, tcod) {
            UseResult::UsedUp => {
                let item_indexes = &world.entity_indexes[&inventory_id];
                // destroy after use, unless it was cancelled for some reason
                world.items[item_indexes.item.unwrap()].owner = 0;
                world.entity_indexes.remove(&inventory_id);
            }
            UseResult::UsedAndKept => (),
            UseResult::Cancelled => add_log(world, "Cancelled", COLOR_LIGHTEST_GREY),
        }
    } else {
        let item_indexes = &world.entity_indexes[&inventory_id];
        let name = world.map_objects[item_indexes.map_object.unwrap()].name.clone();
        add_log(
            world,
            format!("The {} cannot be used.", name),
            COLOR_LIGHTEST_GREY,
        );
    }
}

fn use_medkit(_inventory_id: u32, world: &mut World, _tcod: &mut Tcod) -> UseResult {
    // heal the player
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    let player = &world.characters[player_indexes.character.unwrap()];
    if player.hp == max_hp(PLAYER_ID, world) {
        add_log(world, "You are already at full health.", COLOR_ORANGE);
        return UseResult::Cancelled;
    }
    add_log(world, "Your wounds start to feel better!", COLOR_GREEN);
    heal(PLAYER_ID, HEAL_AMOUNT, world);
    UseResult::UsedUp
}

fn shoot_slingshot(_inventory_id: u32, world: &mut World, tcod: &mut Tcod) -> UseResult {
    // find closest enemy (inside a maximum range and damage it)
    let monster_id = closest_monster(SLINGSHOT_RANGE, world, tcod);
    if let Some(monster_id) = monster_id {
        let indexes = &world.entity_indexes[&monster_id];
        let monster = &mut world.characters[indexes.character.unwrap()];
        if let Some(xp) = take_damage(monster, SLINGSHOT_DAMAGE) {
            let player_indexes = &world.entity_indexes[&PLAYER_ID];
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
        let player_indexes = &world.entity_indexes[&PLAYER_ID];
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

fn throw_brick(_inventory_id: u32, world: &mut World, tcod: &mut Tcod) -> UseResult {
    // ask the player for a target to confuse
    add_log(
        world,
        "Left-click an enemy to throw the brick, or right-click to cancel.",
        COLOR_DARK_SKY,
    );
    let monster_id = target_monster(tcod, world, Some(BRICK_RANGE as f32));
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

fn throw_blasting_cartridge(_inventory_id: u32, world: &mut World, tcod: &mut Tcod) -> UseResult {
    add_log(
        world,
        "Left-click a target tile to throw the charge, or right-click to cancel.",
        COLOR_DARK_SKY,
    );
    let (x, y) = match target_tile(tcod, world, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
    };
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
            if target_id != PLAYER_ID {
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
    world.characters[world.entity_indexes[&PLAYER_ID].character.unwrap()].xp += xp_to_gain;
    UseResult::UsedUp
}

fn toggle_equipment(inventory_id: u32, world: &mut World, _tcod: &mut Tcod) -> UseResult {
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

/// returns a clicked monster inside FOV up to a range, or None if right-clicked
fn target_monster(tcod: &mut Tcod, world: &mut World, max_range: Option<f32>) -> Option<u32> {
    loop {
        match target_tile(tcod, world, max_range) {
            Some((x, y)) => {
                // return the first clicked monster, otherwise continue looping
                return world.entity_indexes.iter().find_map(|(&id, indexes)| {
                    indexes
                        .character
                        .and(indexes.symbol)
                        .filter(|&sy| (world.symbols[sy].x, world.symbols[sy].y) == (x, y))
                        .filter(|_| id != PLAYER_ID)
                        .and(Some(id))
                });
            }
            None => return None,
        }
    }
}

/// return the position of a tile left-clicked in player's FOV (optionally in a
/// range), or (None,None) if right-clicked.
fn target_tile(tcod: &mut Tcod, world: &mut World, max_range: Option<f32>) -> Option<(i32, i32)> {
    loop {
        // render the screen. this erases the inventory and shows the names of
        // objects under the mouse.
        tcod.root.flush();
        let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e| e.1);
        let mut key = None;
        match event {
            Some(input::Event::Mouse(m)) => tcod.mouse = m,
            Some(input::Event::Key(k)) => key = Some(k),
            None => {}
        }
        render_all(world, tcod);
        let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);
        // accept the target if the player clicked in FOV, and in case a range
        // is specified, if it's in that range
        let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
        let in_range = max_range.map_or(true, |range| {
            let player_indexes = &world.entity_indexes[&PLAYER_ID];
            let player_symbol = &world.symbols[player_indexes.symbol.unwrap()];
            let (player_x, player_y) = (player_symbol.x, player_symbol.y);
            distance_to(player_x, player_y, x, y) <= range
        });
        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y));
        }
        let escape = key.map_or(false, |k| k.code == input::KeyCode::Escape);
        if tcod.mouse.rbutton_pressed || escape {
            return None; // cancel if the player right-clicked or pressed Escape
        }
    }
}

// *** Death System ***
fn update_death_state(world: &mut World, _tcod: &mut Tcod) {
    if world.player.action != PlayerAction::TookTurn {
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
    let indexes = &world.entity_indexes[&PLAYER_ID];
    let symbol = &mut world.symbols[indexes.symbol.unwrap()];
    symbol.char = '\u{A3}';
    symbol.color = COLOR_DARK_RED;
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

// *** Character System ***
fn update_character_state(world: &mut World, tcod: &mut Tcod) {
    if world.player.action == PlayerAction::StartGame {
        return;
    }
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    let player = &world.characters[player_indexes.character.unwrap()];
    let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
    // see if the player's experience is enough to level-up
    if player.xp >= level_up_xp {
        // it is! level up
        let new_level = player.level + 1;
        let base_max_hp = player.base_max_hp;
        let base_power = player.base_power;
        let base_defense = player.base_defense;
        add_log(
            world,
            format!(
                "Your battle skills grow stronger! You reached level {}!",
                new_level,
            ),
            COLOR_ORANGE,
        );
        let mut choice = None;
        while choice.is_none() {
            // keep asking until a choice is made
            choice = menu(
                "Level up! Choose a stat to raise:\n",
                &[
                    format!("Constitution (+20 HP, from {})", base_max_hp),
                    format!("Strength (+1 attack, from {})", base_power),
                    format!("Agility (+1 defense, from {})", base_defense),
                ],
                LEVEL_SCREEN_WIDTH,
                &mut tcod.root,
            );
        }
        let player_indexes = &world.entity_indexes[&PLAYER_ID];
        let player = &mut world.characters[player_indexes.character.unwrap()];
        player.level += 1;
        player.xp -= level_up_xp;
        match choice.unwrap() {
            0 => {
                player.base_max_hp += 20;
                player.hp += 20;
            }
            1 => {
                player.base_power += 1;
            }
            2 => {
                player.base_defense += 1;
            }
            _ => unreachable!(),
        }
    }
}

// *** Stats Menu System ***
fn update_stats_menu_state(world: &mut World, tcod: &mut Tcod) {
    if world.player.action == PlayerAction::StartGame {
        return;
    }
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    let player = &world.characters[player_indexes.character.unwrap()];
    if (tcod.key.printable != 'c') || !player.alive {
        return;
    }
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
        max_hp(PLAYER_ID, world),
        power(PLAYER_ID, world),
        defense(PLAYER_ID, world),
    );
    msgbox(&msg, CHARACTER_SCREEN_WIDTH, &mut tcod.root);
}

// *** Help Menu System ***
fn update_help_menu_state(_world: &mut World, tcod: &mut Tcod) {
    if tcod.key.code != tcod::input::KeyCode::F1 {
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
    msgbox(&msg, 36, &mut tcod.root);
}

// *** Save System ***
fn update_saved_game_state(world: &mut World, tcod: &mut Tcod) {
    if (world.player.action == PlayerAction::StartGame) || (tcod.key.code != input::KeyCode::Escape)
    {
        return;
    }
    let save_data = serde_json::to_string(world).unwrap();
    let mut file = fs::File::create("savegame").unwrap();
    file.write_all(save_data.as_bytes()).unwrap();
}

// *** Render System ***
fn render_all(world: &mut World, tcod: &mut Tcod) {
    if world.player.action == PlayerAction::StartGame {
        return;
    }
    let player_indexes = &world.entity_indexes[&PLAYER_ID];
    let player_symbol = &world.symbols[player_indexes.symbol.unwrap()];
    if world.player.previous_player_position != (player_symbol.x, player_symbol.y) {
        tcod.fov.compute_fov(
            player_symbol.x,
            player_symbol.y,
            TORCH_RADIUS,
            FOV_LIGHT_WALLS,
            FOV_ALGO,
        );
        world.player.previous_player_position = (player_symbol.x, player_symbol.y);
    }
    for i in 0..world.map.len() {
        let (x, y) = ((i as i32) % MAP_WIDTH, (i as i32) / MAP_WIDTH);
        let visible = tcod.fov.is_in_fov(x, y);
        let wall = world.map[i].block_sight;
        let wall_bottom = ((y + 1) < MAP_HEIGHT)
            && wall
            && !world.map[((y + 1) * MAP_WIDTH + x) as usize].block_sight;
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
        let explored = &mut world.map[i].explored;
        if visible {
            // since it's visible, explore it
            *explored = true;
        }
        if *explored {
            // show explored tiles only (any visible tile is explored already)
            tcod.con.put_char_ex(x, y, glyph, fg, bg);
        }
    }
    let mut to_draw: Vec<_> = world
        .entity_indexes
        .values()
        .filter(|&indexes| {
            if let (Some(mo), Some(sy)) = (indexes.map_object, indexes.symbol) {
                let symbol = &world.symbols[sy];
                let index_in_map = (symbol.y * MAP_WIDTH + symbol.x) as usize;
                (tcod.fov.is_in_fov(symbol.x, symbol.y) && !world.map_objects[mo].hidden)
                    || (world.map_objects[mo].always_visible && world.map[index_in_map].explored)
            } else {
                false
            }
        })
        .collect();
    // sort so that non-blocknig objects come first
    to_draw.sort_by(|&i1, &i2| {
        let (mi1, mi2) = (i1.map_object.unwrap(), i2.map_object.unwrap());
        world.map_objects[mi1]
            .block
            .cmp(&world.map_objects[mi2].block)
    });
    // draw the objects in the list
    for indexes in to_draw {
        let Symbol { x, y, char, color } = world.symbols[indexes.symbol.unwrap()];
        tcod.con.set_default_foreground(color);
        let char = indexes
            .character
            .and_then(|index| Some(&world.characters[index]))
            .filter(|&ch| ch.looking_right && ch.alive)
            .and(Some((char as u8 + 1) as char))
            .unwrap_or(char);
        tcod.con.put_char(x, y, char, console::BackgroundFlag::None);
    }
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
    // prepare to render the GUI panel
    tcod.panel.set_default_background(COLOR_DARKEST_GREY);
    tcod.panel.clear();
    // print the game messages, one line at a time
    let mut y = MSG_HEIGHT;
    for &LogMessage(ref msg, color) in world.log.iter().rev() {
        let msg_height = tcod
            .panel
            .get_height_rect(MSG_X, MSG_HEIGHT - y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_foreground(color);
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }
    // show the player's stats
    let hp = world.characters[player_indexes.character.unwrap()].hp;
    let max_hp = max_hp(PLAYER_ID, world);
    render_bar(
        &mut tcod.panel,
        1,
        2,
        BAR_WIDTH,
        "HP",
        hp,
        max_hp,
        COLOR_DARK_RED,
        COLOR_DARKER_SEPIA,
    );
    tcod.panel.print_ex(
        1,
        1,
        console::BackgroundFlag::None,
        console::TextAlignment::Left,
        format!("Mine level: {}", world.player.dungeon_level),
    );
    // display names of objects under the mouse
    tcod.panel.set_default_foreground(COLOR_LIGHTEST_GREY);
    tcod.panel.print_rect(
        1,
        3,
        BAR_WIDTH,
        0,
        String::from("You see: ") + &get_names_under_mouse(tcod.mouse, world, &tcod.fov),
    );
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
    tcod.root.flush();
}

fn render_bar(
    panel: &mut console::Offscreen,
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

/// return a string with the names of all objects under the mouse
fn get_names_under_mouse(mouse: input::Mouse, world: &World, fov_map: &tcod::map::Map) -> String {
    let (mx, my) = (mouse.cx as i32, mouse.cy as i32);
    // create a list with the names of all objects at the mouse's coordinates and in FOV
    let names = world
        .entity_indexes
        .values()
        .filter(|&indexes| {
            if let (Some(sy), Some(mo)) = (indexes.symbol, indexes.map_object) {
                let &Symbol { x, y, .. } = &world.symbols[sy];
                ((x, y) == (mx, my)) && fov_map.is_in_fov(x, y) && !world.map_objects[mo].hidden
            } else {
                false
            }
        })
        .map(|indexes| world.map_objects[indexes.map_object.unwrap()].name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    if names.is_empty() {
        String::from("nothing out of the ordinary")
    } else {
        names
    }
}

// *** Game Init System ***
fn update_initial_state(world: &mut World, tcod: &mut Tcod) {
    if (world.player.action != PlayerAction::StartGame) && (tcod.key.code != input::KeyCode::Escape)
    {
        return;
    }
    tcod.con.set_default_background(COLOR_DARK_GROUND_BG);
    let img = tcod::image::Image::from_file("menu_background.png")
        .ok()
        .expect("Background image not found");
    while !tcod.root.window_closed() {
        // show the background image, at twice the regular console resolution
        tcod::image::blit_2x(&img, (0, 0), (-1, -1), &mut tcod.root, (0, 0));
        tcod.root.set_default_foreground(COLOR_DARK_RED);
        tcod.root.print_ex(
            SCREEN_WIDTH / 2,
            SCREEN_HEIGHT / 2 - 4,
            console::BackgroundFlag::None,
            console::TextAlignment::Center,
            "EXPERIMENT 01: ABANDONED MINES",
        );
        tcod.root.print_ex(
            SCREEN_WIDTH / 2,
            SCREEN_HEIGHT - 2,
            console::BackgroundFlag::None,
            console::TextAlignment::Center,
            "by saintech",
        );
        // show options and wait for the player's choice
        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut tcod.root);
        match choice {
            Some(0) => {
                new_game(world, tcod);
                world.player.action = PlayerAction::DidntTakeTurn;
                break;
            }
            Some(1) => {
                if load_game(world).is_ok() {
                    initialise_fov(world, tcod);
                    break;
                } else {
                    msgbox("\nNo saved game to load.\n", 24, &mut tcod.root);
                    continue;
                }
            }
            Some(2) => {
                // quit
                world.player.action = PlayerAction::Exit;
                break;
            }
            _ => {}
        }
    }
    world.player.previous_player_position = (-1, -1);
    tcod.con.clear();
}

fn new_game(world: &mut World, tcod: &mut Tcod) {
    world.id_count = Default::default();
    world.entity_indexes = Default::default();
    world.player = Default::default();
    world.symbols = Default::default();
    world.map = Default::default();
    world.map_objects = Default::default();
    world.characters = Default::default();
    world.ais = Default::default();
    world.items = Default::default();
    world.equipments = Default::default();
    world.log = Default::default();
    spawn_player(world);
    make_map(world, world.player.dungeon_level);
    // initial equipment: Pipe
    let pipe_id = spawn_item(world, Item::Melee, PLAYER_ID, 0, 0);
    let pipe_indexes = &world.entity_indexes[&pipe_id];
    world.symbols[pipe_indexes.symbol.unwrap()] = Symbol {
        x: 0,
        y: 0,
        char: '\u{94}',
        color: COLOR_DARK_SEPIA,
    };
    world.map_objects[pipe_indexes.map_object.unwrap()].name = String::from("Pipe");
    world.equipments[pipe_indexes.equipment.unwrap()].power_bonus = 2;
    initialise_fov(world, tcod);
    add_log(
        world,
        String::from(
            "Welcome stranger! Prepare to perish in the Abandoned Mines. Press F1 for help.\n",
        ),
        COLOR_ORANGE,
    );
}

fn load_game(world: &mut World) -> Result<(), Box<dyn Error>> {
    let mut json_save_state = String::new();
    let mut file = fs::File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let result = serde_json::from_str::<World>(&json_save_state)?;
    world.id_count = result.id_count;
    world.entity_indexes = result.entity_indexes;
    world.player = result.player;
    world.symbols = result.symbols;
    world.map = result.map;
    world.map_objects = result.map_objects;
    world.characters = result.characters;
    world.ais = result.ais;
    world.items = result.items;
    world.equipments = result.equipments;
    world.log = result.log;
    Ok(())
}

fn main() {
    tcod::system::set_fps(LIMIT_FPS);
    let spritesheet = if tcod::system::get_current_resolution() >= (1920, 1080) {
        "spritesheet-14px-2x.png"
    } else {
        "spritesheet-14px.png"
    };
    let root = console::Root::initializer()
        .font(spritesheet, console::FontLayout::AsciiInRow)
        .font_type(console::FontType::Default)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("saintech's experiments: Expt01")
        .init();
    let mut tcod = Tcod {
        root: root,
        con: console::Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        panel: console::Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        fov: tcod::map::Map::new(MAP_WIDTH, MAP_HEIGHT),
        key: Default::default(),
        mouse: Default::default(),
    };
    let mut world = World::default();
    while !tcod.root.window_closed() && (world.player.action != PlayerAction::Exit) {
        update_input_state(&mut world, &mut tcod);
        update_map_interaction_state(&mut world, &mut tcod);
        player_move_or_attack(&mut world, &mut tcod);
        update_ai_turn_state(&mut world, &mut tcod);
        update_inventory_state(&mut world, &mut tcod);
        update_drop_action_state(&mut world, &mut tcod);
        update_death_state(&mut world, &mut tcod);
        update_character_state(&mut world, &mut tcod);
        update_stats_menu_state(&mut world, &mut tcod);
        update_help_menu_state(&mut world, &mut tcod);
        update_saved_game_state(&mut world, &mut tcod);
        render_all(&mut world, &mut tcod);
        update_initial_state(&mut world, &mut tcod);
    }
}
