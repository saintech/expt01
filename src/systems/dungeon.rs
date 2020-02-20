use crate::cfg;
use crate::cmtp::{
    Ai, AiOption, Character, DeathCallback, Equipment, Item, MapCell, MapObject, OwnedItem,
    PlayerAction, PlayerState, Slot, Symbol,
};
use crate::game;
use rand::distributions::{Distribution as _, WeightedIndex};
use rand::Rng as _;
use std::{cmp, fs, io::Write as _};

/// A rectangle on the map, used to characterise a room.
#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }
    fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }
    fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}

fn is_exiting_to_main_menu(world: &game::World) -> bool {
    world.dialogs.is_empty()
        && (world.player.action == PlayerAction::Cancel)
        && (world.player.state == PlayerState::MakingTurn)
}

pub fn update(world: &mut game::World) {
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

fn make_map(world: &mut game::World, level: u32) {
    fill_walls(world);
    let mut rooms = vec![];
    if level == 1 {
        place_hints(world, &mut rooms);
    }
    for _ in rooms.len()..cfg::MAX_ROOMS {
        // random width and height:
        let w = rand::thread_rng().gen_range(cfg::ROOM_MIN_SIZE, cfg::ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(cfg::ROOM_MIN_SIZE, cfg::ROOM_MAX_SIZE + 1);
        // random position without going out of the boundaries of the map
        let x = rand::thread_rng().gen_range(0, cfg::MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, cfg::MAP_HEIGHT - h);
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
                let player_symbol = world.get_character_mut(world.player.id).unwrap().0;
                player_symbol.x = new_x;
                player_symbol.y = new_y;
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

fn fill_walls(world: &mut game::World) {
    for _ in 0..cfg::MAP_WIDTH * cfg::MAP_HEIGHT {
        world.create_entity(
            None,
            Some(MapCell {
                block: true,
                explored: false,
                block_sight: true,
                in_fov: false,
            }),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
    }
}

fn place_hints(world: &mut game::World, rooms: &mut Vec<Rect>) {
    let x = rand::thread_rng().gen_range(0, cfg::MAP_WIDTH - 6);
    let y = rand::thread_rng().gen_range(0, cfg::MAP_HEIGHT - 6);
    let new_room = Rect::new(x, y, 6, 6);
    create_room(new_room, &mut world.map);
    let map_object = MapObject {
        name: String::new(),
        block: false,
        always_visible: false,
        hidden: false,
    };
    let name = "move hint";
    let color = cfg::COLOR_LIGHT_GROUND;
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
    hints.iter().for_each(|&(x, y, glyph)| {
        world.create_entity(
            Some(Symbol { x, y, glyph, color }),
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
            None,
        );
    });
    let player_symbol = world.get_character_mut(world.player.id).unwrap().0;
    player_symbol.x = x + 3;
    player_symbol.y = y + 3;
    rooms.push(new_room);
}

fn create_room(room: Rect, map: &mut Vec<MapCell>) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
            map[index_in_map].block = false;
            map[index_in_map].block_sight = false;
        }
    }
}

struct Transition {
    level: u32,
    value: u32,
}

fn place_objects(room: Rect, world: &mut game::World, level: u32) {
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
                        glyph: '\u{82}',
                        color: cfg::COLOR_ORANGE,
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
                        glyph: '\u{84}',
                        color: cfg::COLOR_DARK_SKY,
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

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Vec<MapCell>) {
    for x in cmp::min(x1, x2)..=cmp::max(x1, x2) {
        let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
        map[index_in_map].block = false;
        map[index_in_map].block_sight = false;
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Vec<MapCell>) {
    for y in cmp::min(y1, y2)..=cmp::max(y1, y2) {
        let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
        map[index_in_map].block = false;
        map[index_in_map].block_sight = false;
    }
}

fn spawn_stairs(world: &mut game::World, x: i32, y: i32) {
    let glyph = '\u{A4}';
    let color = cfg::COLOR_LIGHT_WALL;
    let map_object = MapObject {
        name: String::from("stairs"),
        block: false,
        always_visible: true,
        hidden: false,
    };
    world.create_entity(
        Some(Symbol { x, y, glyph, color }),
        None,
        Some(map_object),
        None,
        None,
        None,
        None,
        None,
        None,
    );
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

fn is_blocked(x: i32, y: i32, world: &game::World) -> bool {
    let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
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

fn spawn_monster(
    world: &mut game::World,
    name: &str,
    symbol: Symbol,
    character: Character,
    ai: Ai,
) {
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
        None,
    );
}

fn spawn_item(world: &mut game::World, item: Item, owner: u32, x: i32, y: i32) -> u32 {
    let (glyph, name, color, equipment) = match item {
        Item::Medkit => ('\u{90}', "Medkit", cfg::COLOR_DARK_RED, None),
        Item::SlingshotAmmo => (
            '\u{91}',
            "Bullet For Slingshot",
            cfg::COLOR_DARK_SEPIA,
            None,
        ),
        Item::BlastingCartridge => ('\u{92}', "Blasting Cartridge", cfg::COLOR_DARK_SEPIA, None),
        Item::Brick => ('\u{93}', "Brick", cfg::COLOR_DARK_SEPIA, None),
        Item::Melee => (
            '\u{95}',
            "Pickaxe",
            cfg::COLOR_DARK_SKY,
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
            cfg::COLOR_DARK_SKY,
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
        Some(Symbol { x, y, glyph, color }),
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
        None,
    )
}

fn spawn_player(world: &mut game::World) {
    let x = cfg::SCREEN_WIDTH / 2;
    let y = cfg::SCREEN_HEIGHT / 2;
    let glyph = '\u{80}';
    let color = cfg::COLOR_GREEN;
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
    world.player.id = world.create_entity(
        Some(Symbol { x, y, glyph, color }),
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
        Some(AiOption { option: None }),
        None,
        None,
        None,
        None,
    );
    // initial equipment: Pipe
    let pipe_id = spawn_item(world, Item::Melee, world.player.id, 0, 0);
    let (sym, map_obj, _, eqp) = world.get_item_mut(pipe_id).unwrap();
    *sym = Symbol {
        x: 0,
        y: 0,
        glyph: '\u{94}',
        color: cfg::COLOR_DARK_SEPIA,
    };
    map_obj.name = String::from("Pipe");
    eqp.unwrap().power_bonus = 2;
    world.add_log(
        cfg::COLOR_ORANGE,
        String::from(
            "Welcome stranger! Prepare to perish in the Abandoned Mines. Press F1 for help.\n",
        ),
    );
    if world.player.dungeon_level == 0 {
        world.player.dungeon_level = 1;
    };
}

fn save_game(world: &game::World) {
    let save_data = serde_json::to_string(world).unwrap();
    let mut file = fs::File::create("savegame").unwrap();
    file.write_all(save_data.as_bytes()).unwrap();
}
