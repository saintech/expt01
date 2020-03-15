use crate::cfg;
use crate::cmtp::{
    AiOption, DialogKind, MapCell, MapObject, PlayerAction, PlayerState, Symbol,
};
use crate::engine::asset;
use crate::engine::game;
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
        match (asset::CharactersLoader::load(), asset::ItemsLoader::load()) {
            (Err(err), _) => {
                let msg = format!(
                    "Error in the characters config.\nFix the error or delete the character \
                    (see \"dummy\" character for example\"):\n\n{}",
                    err,
                );
                world.add_dialog_box(DialogKind::MessageBox, msg, vec![], 36);
                world.player.state = PlayerState::InDialog;
            }
            (_, Err(err)) => {
                let msg = format!(
                    "Error in the items config.\nFix the error or delete the item \
                    (see \"dummy\" item for example\"):\n\n{}",
                    err,
                );
                world.add_dialog_box(DialogKind::MessageBox, msg, vec![], 36);
                world.player.state = PlayerState::InDialog;
            }
            (Ok(char_loader), Ok(items_loader)) => {
                if world.entity_indexes.get(&world.player.id).is_none() {
                    spawn_player(world, &char_loader, &items_loader);
                }
                make_map(world, &char_loader, &items_loader, world.player.dungeon_level);
                world.player.action = PlayerAction::None;
            }
        }
    } else if is_exiting_to_main_menu(world) {
        world.player.action = PlayerAction::None;
        save_game(world);
        *world = Default::default();
    }
}

fn make_map(world: &mut game::World, char_loader: &asset::CharactersLoader, items_loader: &asset::ItemsLoader, level: u32) {
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
            place_objects(new_room, world, char_loader, items_loader, level);
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
        game::new_entity()
            .add_map_cell(MapCell {
                block: true,
                explored: false,
                block_sight: true,
                in_fov: false,
            })
            .create(world);
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
        game::new_entity()
            .add_symbol(Symbol { x, y, glyph, color })
            .add_map_object(MapObject {
                name: name.to_string(),
                ..map_object
            })
            .create(world);
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

fn place_objects(
    room: Rect,
    world: &mut game::World,
    char_loader: &asset::CharactersLoader,
    items_loader: &asset::ItemsLoader,
    level: u32,
) {
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
    let (monster_ids, monster_chances) = char_loader.weighted_table(level);
    let monster_choice = WeightedIndex::new(monster_chances).unwrap();
    for _ in 0..num_monsters {
        // choose random spot for this monster
        let x = rng.gen_range(room.x1 + 1, room.x2);
        let y = rng.gen_range(room.y1 + 1, room.y2);
        // only place it if the tile is not blocked
        if !is_blocked(x, y, world) {
            let mut monster = char_loader.get_clone(monster_ids[monster_choice.sample(&mut rng)]);
            monster.character.alive = true;
            monster.symbol.x = x;
            monster.symbol.y = y;
            game::new_entity()
                .add_symbol(monster.symbol)
                .add_map_object(monster.map_object)
                .add_character(monster.character)
                .add_ai(AiOption { option: monster.ai })
                .create(world);
        }
    }
    // maximum number of items per room
    let max_items = from_dungeon_level(
        &[
            Transition { level: 1, value: 1 },
            Transition { level: 4, value: 2 },
        ],
        level,
    );
    // choose random number of items
    let num_items = rng.gen_range(0, max_items + 1);
    let (item_ids, item_chances) = items_loader.weighted_table(level);
    let item_choice = WeightedIndex::new(item_chances).unwrap();
    for _ in 0..num_items {
        // choose random spot for this item
        let x = rng.gen_range(room.x1 + 1, room.x2);
        let y = rng.gen_range(room.y1 + 1, room.y2);
        // only place it if the tile is not blocked
        if !is_blocked(x, y, world) {
            let mut item = items_loader.get_clone(item_ids[item_choice.sample(&mut rng)]);
            item.symbol.x = x;
            item.symbol.y = y;
            game::new_entity()
                .add_symbol(item.symbol)
                .add_map_object(item.map_object)
                .add_item(item.item)
                .add_equipment(item.equipment)
                .create(world);
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
    game::new_entity()
        .add_symbol(Symbol { x, y, glyph, color })
        .add_map_object(map_object)
        .create(world);
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

fn spawn_player(world: &mut game::World, char_loader: &asset::CharactersLoader, items_loader: &asset::ItemsLoader) {
    let mut player = char_loader.get_clone("player");
    player.character.alive = true;
    player.symbol.x = cfg::SCREEN_WIDTH / 2;
    player.symbol.y = cfg::SCREEN_HEIGHT / 2;
    world.player.id = game::new_entity()
        .add_symbol(player.symbol)
        .add_map_object(player.map_object)
        .add_character(player.character)
        .add_ai(AiOption { option: player.ai })
        .create(world);
    // initial equipment: Pipe
    let mut pipe = items_loader.get_clone("pipe");
    pipe.item.owner = world.player.id;
    game::new_entity()
        .add_symbol(pipe.symbol)
        .add_map_object(pipe.map_object)
        .add_item(pipe.item)
        .add_equipment(pipe.equipment)
        .create(world);
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
