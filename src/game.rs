use crate::cfg::*;
use crate::cmtp::*;
use rand::distributions::{Distribution as _, WeightedIndex};
use rand::Rng as _;
use serde::{Deserialize, Serialize};
use std::{cmp, collections::btree_map::BTreeMap};
use tcod::{colors, console, input, Console as _};

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityIndexes {
    pub symbol: Option<usize>,
    pub map_cell: Option<usize>,
    pub map_object: Option<usize>,
    pub character: Option<usize>,
    pub ai: Option<usize>,
    pub item: Option<usize>,
    pub equipment: Option<usize>,
    pub log_message: Option<usize>,
}

pub struct Tcod {
    pub root: console::Root,
    pub con: console::Offscreen,
    pub panel: console::Offscreen,
    pub fov: tcod::map::Map,
    pub key: input::Key,
    pub mouse: input::Mouse,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct World {
    pub id_count: u32,
    pub entity_indexes: BTreeMap<u32, EntityIndexes>,
    pub player: Player,
    pub symbols: Vec<Symbol>,
    pub map: Vec<MapCell>,
    pub map_objects: Vec<MapObject>,
    pub characters: Vec<Character>,
    pub ais: Vec<AiOption>,
    pub items: Vec<OwnedItem>,
    pub equipments: Vec<Equipment>,
    pub log: Vec<LogMessage>,
}

impl World {
    pub fn create_entity(
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

pub fn add_log(world: &mut World, message: impl Into<String>, color: colors::Color) {
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
pub fn distance_to(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> f32 {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
}

pub fn take_damage(target: &mut Character, damage: i32) -> Option<i32> {
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

pub fn attack_by(attacker_id: u32, target_id: u32, world: &mut World) {
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
pub fn heal(id: u32, amount: i32, world: &mut World) {
    let max_hp = max_hp(id, world);
    let player_indexes = &world.entity_indexes[&id];
    let character = &mut world.characters[player_indexes.character.unwrap()];
    character.hp += amount;
    if character.hp > max_hp {
        character.hp = max_hp;
    }
}

/// Equip object and show a message about it
pub fn equip(id: u32, world: &mut World) {
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
pub fn dequip(id: u32, world: &mut World) {
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
fn get_all_equipped(owner: u32, world: &World) -> impl Iterator<Item = &Equipment> {
    world.entity_indexes.values().filter_map(move |indexes| {
        indexes
            .item
            .filter(|&it| world.items[it].owner == owner)
            .and(indexes.equipment)
            .filter(|&eq| world.equipments[eq].equipped)
            .map(|eq| &world.equipments[eq])
    })
}

pub fn power(id: u32, world: &World) -> i32 {
    let base_power = world.entity_indexes[&id]
        .character
        .map_or(0, |ch| world.characters[ch].base_power);
    let bonus: i32 = get_all_equipped(id, world).map(|eq| eq.power_bonus).sum();
    base_power + bonus
}

pub fn defense(id: u32, world: &World) -> i32 {
    let base_defense = world.entity_indexes[&id]
        .character
        .map_or(0, |ch| world.characters[ch].base_defense);
    let bonus: i32 = get_all_equipped(id, world).map(|eq| eq.defense_bonus).sum();
    base_defense + bonus
}

pub fn max_hp(id: u32, world: &World) -> i32 {
    let base_max_hp = world.entity_indexes[&id]
        .character
        .map_or(0, |ch| world.characters[ch].base_max_hp);
    let bonus: i32 = get_all_equipped(id, world).map(|eq| eq.max_hp_bonus).sum();
    base_max_hp + bonus
}

/// move by the given amount, if the destination is not blocked
pub fn move_by(id: u32, dx: i32, dy: i32, world: &mut World) {
    let indexes = &world.entity_indexes[&id];
    let symbol = &world.symbols[indexes.symbol.unwrap()];
    let &Symbol { x, y, .. } = symbol;
    if !is_blocked(x + dx, y + dy, world) {
        world.symbols[indexes.symbol.unwrap()].x = x + dx;
        world.symbols[indexes.symbol.unwrap()].y = y + dy;
    }
}

pub fn get_equipped_in_slot(slot: Slot, world: &mut World) -> Option<u32> {
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
pub fn next_level(world: &mut World, tcod: &mut Tcod) {
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

pub fn make_map(world: &mut World, level: u32) {
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
            Some(MapObject {
                name: name.to_string(),
                ..map_object
            }),
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

pub fn spawn_player(world: &mut World) {
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

pub fn spawn_item(world: &mut World, item: Item, owner: u32, x: i32, y: i32) -> u32 {
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

pub fn menu(
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

pub fn inventory_menu(world: &mut World, header: &str, root: &mut console::Root) -> Option<u32> {
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

pub fn msgbox(text: &str, width: i32, root: &mut console::Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}

/// create the FOV map, according to the generated map
pub fn initialise_fov(world: &mut World, tcod: &mut Tcod) {
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
