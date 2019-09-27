use rand::distributions::{Distribution as _, WeightedIndex};
use rand::Rng as _;
use serde::{Deserialize, Serialize};
use std::{cmp, error::Error, fs, io::Read as _, io::Write as _};
use tcod::{colors, console, input, Console as _};

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

const FOV_ALGO: tcod::map::FovAlgorithm = tcod::map::FovAlgorithm::Diamond; // default FOV algorithm
const FOV_LIGHT_WALLS: bool = true; // light walls or not
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
const PLAYER: usize = 0;

type Map = Vec<Vec<Tile>>;

/// A tile of the map and its properties
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Tile {
    blocked: bool,
    explored: bool,
    block_sight: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            explored: false,
            block_sight: false,
        }
    }
    pub fn wall() -> Self {
        Tile {
            blocked: true,
            explored: false,
            block_sight: true,
        }
    }
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

/// This is a generic object: the player, a monster, an item, the stairs...
/// It's always represented by a character on screen.
#[derive(Debug, Serialize, Deserialize)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: colors::Color,
    name: String,
    blocks: bool,
    alive: bool,
    fighter: Option<Fighter>,
    ai: Option<Ai>,
    item: Option<Item>,
    equipment: Option<Equipment>,
    always_visible: bool,
    level: i32,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: colors::Color, blocks: bool) -> Self {
        Object {
            x,
            y,
            char,
            color,
            name: name.into(),
            blocks,
            alive: false,
            fighter: None,
            ai: None,
            item: None,
            equipment: None,
            always_visible: false,
            level: 1,
        }
    }
    /// set the color and then draw the character that represents this object at its position
    pub fn draw(&self, con: &mut impl tcod::Console) {
        con.set_default_foreground(self.color);
        let char = self
            .fighter
            .filter(|fighter| fighter.looking_right && self.alive)
            .map_or(self.char, |_| (self.char as u8 + 1) as char);
        con.put_char(self.x, self.y, char, console::BackgroundFlag::None);
    }
    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
    /// return the distance to another object
    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }
    /// return the distance to some coordinates
    pub fn distance(&self, x: i32, y: i32) -> f32 {
        (((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
    }
    pub fn take_damage(&mut self, damage: i32, game: &mut Game) -> Option<i32> {
        // apply damage if possible
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }
        // check for death, call the death function
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, game);
                return Some(fighter.xp);
            }
        }
        None
    }
    pub fn attack(&mut self, target: &mut Object, game: &mut Game) {
        // a simple formula for attack damage
        let damage = self.power(game) - target.defense(game);
        if damage > 0 {
            game.log.add(
                format!(
                    "{} attacks {} for {} hit points.",
                    self.name, target.name, damage
                ),
                COLOR_LIGHTEST_GREY,
            );
            if let Some(xp) = target.take_damage(damage, game) {
                // yield experience to the player
                self.fighter.as_mut().unwrap().xp += xp;
            }
        } else {
            game.log.add(
                format!(
                    "{} attacks {} but it has no effect!",
                    self.name, target.name
                ),
                COLOR_LIGHTEST_GREY,
            );
        }
    }
    /// heal by the given amount, without going over the maximum
    pub fn heal(&mut self, amount: i32, game: &Game) {
        let max_hp = self.max_hp(game);
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > max_hp {
                fighter.hp = max_hp;
            }
        }
    }
    /// Equip object and show a message about it
    pub fn equip(&mut self, log: &mut Vec<(String, colors::Color)>) {
        if self.item.is_none() {
            log.add(
                format!("Can't equip {:?} because it's not an Item.", self),
                COLOR_ORANGE,
            );
            return;
        };
        if let Some(ref mut equipment) = self.equipment {
            if !equipment.equipped {
                equipment.equipped = true;
                log.add(
                    format!("Equipped {} on {}.", self.name, equipment.slot),
                    COLOR_GREEN,
                );
            }
        } else {
            log.add(
                format!("Can't equip {:?} because it's not an Equipment.", self),
                COLOR_ORANGE,
            );
        }
    }
    /// Dequip object and show a message about it
    pub fn dequip(&mut self, log: &mut Vec<(String, colors::Color)>) {
        if self.item.is_none() {
            log.add(
                format!("Can't dequip {:?} because it's not an Item.", self),
                COLOR_ORANGE,
            );
            return;
        };
        if let Some(ref mut equipment) = self.equipment {
            if equipment.equipped {
                equipment.equipped = false;
                log.add(
                    format!("Dequipped {} from {}.", self.name, equipment.slot),
                    COLOR_DARK_SKY,
                );
            }
        } else {
            log.add(
                format!("Can't dequip {:?} because it's not an Equipment.", self),
                COLOR_ORANGE,
            );
        }
    }
    pub fn power(&self, game: &Game) -> i32 {
        let base_power = self.fighter.map_or(0, |f| f.base_power);
        let bonus: i32 = self
            .get_all_equipped(game)
            .iter()
            .map(|e| e.power_bonus)
            .sum();
        base_power + bonus
    }
    pub fn defense(&self, game: &Game) -> i32 {
        let base_defense = self.fighter.map_or(0, |f| f.base_defense);
        let bonus: i32 = self
            .get_all_equipped(game)
            .iter()
            .map(|e| e.defense_bonus)
            .sum();
        base_defense + bonus
    }
    pub fn max_hp(&self, game: &Game) -> i32 {
        let base_max_hp = self.fighter.map_or(0, |f| f.base_max_hp);
        let bonus: i32 = self
            .get_all_equipped(game)
            .iter()
            .map(|e| e.max_hp_bonus)
            .sum();
        base_max_hp + bonus
    }
    /// returns a list of equipped items
    pub fn get_all_equipped(&self, game: &Game) -> Vec<Equipment> {
        if self.name == "Player" {
            game.inventory
                .iter()
                .filter(|item| item.equipment.map_or(false, |e| e.equipped))
                .map(|item| item.equipment.unwrap())
                .collect()
        } else {
            vec![] // other objects have no equipment
        }
    }
}

/// move by the given amount, if the destination is not blocked
fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    // vector from this object to the target, and distance
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();
    // normalize it to length 1 (preserving direction), then round it and
    // convert to integer so the movement is restricted to the map grid
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}

/// Mutably borrow two *separate* elements from the given slice.
/// Panics when the indexes are equal or out of bounds.
fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert_ne!(first_index, second_index);
    let split_at_index = cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}

/// add to the player's inventory and remove from the map
fn pick_item_up(object_id: usize, objects: &mut Vec<Object>, game: &mut Game) {
    if game.inventory.len() >= 35 {
        game.log.add(
            format!(
                "Your inventory is full, cannot pick up {}.",
                objects[object_id].name
            ),
            COLOR_DARK_RED,
        );
    } else {
        let item = objects.swap_remove(object_id);
        game.log
            .add(format!("You picked up a {}!", item.name), COLOR_GREEN);
        let index = game.inventory.len();
        let slot = item.equipment.map(|e| e.slot);
        game.inventory.push(item);
        // automatically equip, if the corresponding equipment slot is unused
        if let Some(slot) = slot {
            if get_equipped_in_slot(slot, &game.inventory).is_none() {
                game.inventory[index].equip(&mut game.log);
            }
        }
    }
}

fn get_equipped_in_slot(slot: Slot, inventory: &[Object]) -> Option<usize> {
    for (inventory_id, item) in inventory.iter().enumerate() {
        if item
            .equipment
            .as_ref()
            .map_or(false, |e| e.equipped && e.slot == slot)
        {
            return Some(inventory_id);
        }
    }
    None
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // first test the map tile
    if map[x as usize][y as usize].blocked {
        return true;
    }
    // now check for any blocking objects
    objects
        .iter()
        .any(|object| object.blocks && object.pos() == (x, y))
}

// combat-related properties and methods (monster, player, NPC).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Fighter {
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
    Player,
    Monster,
}

impl DeathCallback {
    fn callback(self, object: &mut Object, game: &mut Game) {
        use DeathCallback::*;
        let callback: fn(&mut Object, &mut Game) = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object, game);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Ai {
    Basic,
    Confused {
        previous_ai: Box<Ai>,
        num_turns: i32,
    },
}

fn ai_take_turn(
    monster_id: usize,
    objects: &mut [Object],
    game: &mut Game,
    fov_map: &tcod::map::Map,
) {
    use Ai::*;
    if let Some(ai) = objects[monster_id].ai.take() {
        let new_ai = match ai {
            Basic => ai_basic(monster_id, objects, game, fov_map),
            Confused {
                previous_ai,
                num_turns,
            } => ai_confused(monster_id, objects, game, previous_ai, num_turns),
        };
        objects[monster_id].ai = Some(new_ai);
    }
}

fn ai_basic(monster_id: usize, objects: &mut [Object], game: &mut Game, fov_map: &tcod::Map) -> Ai {
    // a basic monster takes its turn. If you can see it, it can see you
    let (monster_x, monster_y) = objects[monster_id].pos();
    let (player_x, player_y) = objects[PLAYER].pos();
    if (monster_x > player_x) || ((monster_x == player_x) && (monster_y < player_y)) {
        objects[monster_id]
            .fighter
            .as_mut()
            .map(|f| f.looking_right = false);
    } else if (monster_x < player_x) || ((monster_x == player_x) && (monster_y > player_y)) {
        objects[monster_id]
            .fighter
            .as_mut()
            .map(|f| f.looking_right = true);
    }
    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            // move towards player if far away
            move_towards(monster_id, player_x, player_y, &game.map, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            // close enough, attack! (if the player is still alive.)
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, game);
        }
    }
    Ai::Basic
}

fn ai_confused(
    monster_id: usize,
    objects: &mut [Object],
    game: &mut Game,
    previous_ai: Box<Ai>,
    num_turns: i32,
) -> Ai {
    if num_turns >= 0 {
        // still confused ...
        // move in a random direction, and decrease the number of turns confused
        move_by(
            monster_id,
            rand::thread_rng().gen_range(-1, 2),
            rand::thread_rng().gen_range(-1, 2),
            &game.map,
            objects,
        );
        Ai::Confused {
            previous_ai: previous_ai,
            num_turns: num_turns - 1,
        }
    } else {
        // restore the previous AI (this one will be deleted)
        game.log.add(
            format!("The {} is no longer confused!", objects[monster_id].name),
            COLOR_ORANGE,
        );
        *previous_ai
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum Item {
    Medkit,
    SlingshotAmmo,
    Brick,
    BlastingCartridge,
    Melee,
    Clothing,
}

enum UseResult {
    UsedUp,
    UsedAndKept,
    Cancelled,
}

fn use_item(inventory_id: usize, objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) {
    use Item::*;
    // just call the "use_function" if it is defined
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use = match item {
            Medkit => use_medkit,
            SlingshotAmmo => shoot_slingshot,
            Brick => throw_brick,
            BlastingCartridge => throw_blasting_cartridge,
            Melee => toggle_equipment,
            Clothing => toggle_equipment,
        };
        match on_use(inventory_id, objects, game, tcod) {
            UseResult::UsedUp => {
                // destroy after use, unless it was cancelled for some reason
                game.inventory.remove(inventory_id);
            }
            UseResult::UsedAndKept => (),
            UseResult::Cancelled => {
                game.log.add("Cancelled", COLOR_LIGHTEST_GREY);
            }
        }
    } else {
        game.log.add(
            format!("The {} cannot be used.", game.inventory[inventory_id].name),
            COLOR_LIGHTEST_GREY,
        );
    }
}

fn drop_item(inventory_id: usize, objects: &mut Vec<Object>, game: &mut Game) {
    let mut item = game.inventory.remove(inventory_id);
    if item.equipment.is_some() {
        item.dequip(&mut game.log);
    }
    item.set_pos(objects[PLAYER].x, objects[PLAYER].y);
    game.log
        .add(format!("You dropped a {}.", item.name), COLOR_DARK_SKY);
    objects.push(item);
}

/// return the position of a tile left-clicked in player's FOV (optionally in a
/// range), or (None,None) if right-clicked.
fn target_tile(
    tcod: &mut Tcod,
    objects: &[Object],
    game: &mut Game,
    max_range: Option<f32>,
) -> Option<(i32, i32)> {
    use input::KeyCode::Escape;
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
        render_all(tcod, objects, game, false);
        let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);
        // accept the target if the player clicked in FOV, and in case a range
        // is specified, if it's in that range
        let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
        let in_range = max_range.map_or(true, |range| objects[PLAYER].distance(x, y) <= range);
        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y));
        }
        let escape = key.map_or(false, |k| k.code == Escape);
        if tcod.mouse.rbutton_pressed || escape {
            return None; // cancel if the player right-clicked or pressed Escape
        }
    }
}

/// returns a clicked monster inside FOV up to a range, or None if right-clicked
fn target_monster(
    tcod: &mut Tcod,
    objects: &[Object],
    game: &mut Game,
    max_range: Option<f32>,
) -> Option<usize> {
    loop {
        match target_tile(tcod, objects, game, max_range) {
            Some((x, y)) => {
                // return the first clicked monster, otherwise continue looping
                for (id, obj) in objects.iter().enumerate() {
                    if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
                        return Some(id);
                    }
                }
            }
            None => return None,
        }
    }
}

/// find closest enemy, up to a maximum range, and in the player's FOV
fn closest_monster(max_range: i32, objects: &mut [Object], tcod: &Tcod) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32; // start with (slightly more than) maximum range
    for (id, object) in objects.iter().enumerate() {
        if (id != PLAYER)
            && object.fighter.is_some()
            && object.ai.is_some()
            && tcod.fov.is_in_fov(object.x, object.y)
        {
            // calculate distance between this object and the player
            let dist = objects[PLAYER].distance_to(object);
            if dist < closest_dist {
                // it's closer, so remember it
                closest_enemy = Some(id);
                closest_dist = dist;
            }
        }
    }
    closest_enemy
}

fn use_medkit(
    _inventory_id: usize,
    objects: &mut [Object],
    game: &mut Game,
    _tcod: &mut Tcod,
) -> UseResult {
    // heal the player
    let player = &mut objects[PLAYER];
    if let Some(fighter) = player.fighter {
        if fighter.hp == player.max_hp(game) {
            game.log
                .add("You are already at full health.", COLOR_ORANGE);
            return UseResult::Cancelled;
        }
        game.log
            .add("Your wounds start to feel better!", COLOR_GREEN);
        player.heal(HEAL_AMOUNT, game);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn shoot_slingshot(
    _inventory_id: usize,
    objects: &mut [Object],
    game: &mut Game,
    tcod: &mut Tcod,
) -> UseResult {
    // find closest enemy (inside a maximum range and damage it)
    let monster_id = closest_monster(SLINGSHOT_RANGE, objects, tcod);
    if let Some(monster_id) = monster_id {
        game.log.add(
            format!(
                "A Steel Ball whizzed to a {}! The damage is {} hit points.",
                objects[monster_id].name, SLINGSHOT_DAMAGE
            ),
            COLOR_LIGHTEST_GREY,
        );
        if let Some(xp) = objects[monster_id].take_damage(SLINGSHOT_DAMAGE, game) {
            objects[PLAYER].fighter.as_mut().unwrap().xp += xp;
        }
        UseResult::UsedUp
    } else {
        // no enemy found within maximum range
        game.log
            .add("No enemy is close enough to shoot.", COLOR_DARK_SKY);
        UseResult::Cancelled
    }
}

fn throw_brick(
    _inventory_id: usize,
    objects: &mut [Object],
    game: &mut Game,
    tcod: &mut Tcod,
) -> UseResult {
    // ask the player for a target to confuse
    game.log.add(
        "Left-click an enemy to throw the brick, or right-click to cancel.",
        COLOR_DARK_SKY,
    );
    let monster_id = target_monster(tcod, objects, game, Some(BRICK_RANGE as f32));
    if let Some(monster_id) = monster_id {
        let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);
        // replace the monster's AI with a "confused" one; after
        // some turns it will restore the old AI
        objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: BRICK_NUM_TURNS,
        });
        game.log.add(
            format!(
                "The eyes of {} look vacant, as he starts to stumble around!",
                objects[monster_id].name
            ),
            COLOR_LIGHTEST_GREY,
        );
        UseResult::UsedUp
    } else {
        // no enemy fonud within maximum range
        game.log
            .add("No enemy is close enough to throw.", COLOR_DARK_SKY);
        UseResult::Cancelled
    }
}

fn throw_blasting_cartridge(
    _inventory_id: usize,
    objects: &mut [Object],
    game: &mut Game,
    tcod: &mut Tcod,
) -> UseResult {
    // ask the player for a target tile to throw a fireball at
    game.log.add(
        "Left-click a target tile to throw the charge, or right-click to cancel.",
        COLOR_DARK_SKY,
    );
    let (x, y) = match target_tile(tcod, objects, game, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
    };
    game.log.add(
        format!(
            "The Blasting Cartridge explodes, crushing everything within {} tiles!",
            BLASTING_RADIUS
        ),
        COLOR_ORANGE,
    );
    let mut xp_to_gain = 0;
    for (id, obj) in objects.iter_mut().enumerate() {
        if obj.distance(x, y) <= BLASTING_RADIUS as f32 && obj.fighter.is_some() {
            game.log.add(
                format!(
                    "The {} gets damaged for {} hit points.",
                    obj.name, BLASTING_DAMAGE
                ),
                COLOR_LIGHTEST_GREY,
            );
            if let Some(xp) = obj.take_damage(BLASTING_DAMAGE, game) {
                if id != PLAYER {
                    // Don't reward the player for burning themself!
                    xp_to_gain += xp;
                }
            }
        }
    }
    objects[PLAYER].fighter.as_mut().unwrap().xp += xp_to_gain;
    UseResult::UsedUp
}

fn toggle_equipment(
    inventory_id: usize,
    _objects: &mut [Object],
    game: &mut Game,
    _tcod: &mut Tcod,
) -> UseResult {
    let equipment = match game.inventory[inventory_id].equipment {
        Some(equipment) => equipment,
        None => return UseResult::Cancelled,
    };
    if equipment.equipped {
        game.inventory[inventory_id].dequip(&mut game.log);
    } else {
        // if the slot is already being used, dequip whatever is there first
        if let Some(current) = get_equipped_in_slot(equipment.slot, &game.inventory) {
            game.inventory[current].dequip(&mut game.log);
        }
        game.inventory[inventory_id].equip(&mut game.log);
    }
    UseResult::UsedAndKept
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
/// An object that can be equipped, yielding bonuses.
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

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..=cmp::max(x1, x2) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..=cmp::max(y1, y2) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn place_hints(objects: &mut Vec<Object>, map: &mut Vec<Vec<Tile>>, rooms: &mut Vec<Rect>) {
    let x = rand::thread_rng().gen_range(0, MAP_WIDTH - 6);
    let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - 6);
    let new_room = Rect::new(x, y, 6, 6);
    create_room(new_room, map);
    objects.push(Object::new(
        x + 2,
        y + 2,
        '\u{14}',
        "move hint",
        COLOR_LIGHT_GROUND,
        false,
    ));
    objects.push(Object::new(
        x + 2,
        y + 4,
        '\u{15}',
        "move hint",
        COLOR_LIGHT_GROUND,
        false,
    ));
    objects.push(Object::new(
        x + 4,
        y + 2,
        '\u{16}',
        "move hint",
        COLOR_LIGHT_GROUND,
        false,
    ));
    objects.push(Object::new(
        x + 4,
        y + 4,
        '\u{17}',
        "move hint",
        COLOR_LIGHT_GROUND,
        false,
    ));
    objects.push(Object::new(
        x + 3,
        y + 2,
        '\u{18}',
        "move hint",
        COLOR_LIGHT_GROUND,
        false,
    ));
    objects.push(Object::new(
        x + 3,
        y + 4,
        '\u{19}',
        "move hint",
        COLOR_LIGHT_GROUND,
        false,
    ));
    objects.push(Object::new(
        x + 4,
        y + 3,
        '\u{1A}',
        "move hint",
        COLOR_LIGHT_GROUND,
        false,
    ));
    objects.push(Object::new(
        x + 2,
        y + 3,
        '\u{1B}',
        "move hint",
        COLOR_LIGHT_GROUND,
        false,
    ));
    objects[PLAYER].set_pos(x + 3, y + 3);
    rooms.push(new_room);
}

fn make_map(objects: &mut Vec<Object>, level: u32) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
    // Player is the first element, remove everything else.
    // NOTE: works only when the player is the first object!
    assert_eq!(&objects[PLAYER] as *const _, &objects[0] as *const _);
    objects.truncate(1);
    let mut rooms = vec![];
    if level == 1 {
        place_hints(objects, &mut map, &mut rooms);
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
            create_room(new_room, &mut map);
            // add some content to this room, such as monsters
            place_objects(new_room, &map, objects, level);
            // center coordinates of the new room, will be useful later
            let (new_x, new_y) = new_room.center();
            if rooms.is_empty() {
                // this is the first room, where the player starts at
                objects[PLAYER].set_pos(new_x, new_y);
            } else {
                // all rooms after the first: connect it to the previous room with a tunnel
                // center coordinates of the previous room
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();
                // toss a coin (random bool value -- either true or false)
                if rand::random() {
                    // first move horizontally, then vertically
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    // first move vertically, then horizontally
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }
            // finally, append the new room to the list
            rooms.push(new_room);
        }
    }
    // create stairs at the center of the last room
    let (last_room_x, last_room_y) = rooms[rooms.len() - 1].center();
    let mut stairs = Object::new(
        last_room_x,
        last_room_y,
        '\u{A4}',
        "stairs",
        COLOR_LIGHT_WALL,
        false,
    );
    stairs.always_visible = true;
    objects.push(stairs);
    map
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

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>, level: u32) {
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
        if !is_blocked(x, y, map, objects) {
            let mut monster = match monsters[monster_choice.sample(&mut rng)] {
                "roach" => {
                    let mut roach = Object::new(x, y, '\u{82}', "Roach", COLOR_ORANGE, true);
                    roach.fighter = Some(Fighter {
                        base_max_hp: 20,
                        hp: 20,
                        base_defense: 0,
                        base_power: 4,
                        xp: 35,
                        on_death: DeathCallback::Monster,
                        looking_right: false,
                    });
                    roach.ai = Some(Ai::Basic);
                    roach
                }
                "rat" => {
                    let mut rat = Object::new(x, y, '\u{84}', "Rat", COLOR_DARK_SKY, true);
                    rat.fighter = Some(Fighter {
                        base_max_hp: 30,
                        hp: 30,
                        base_defense: 2,
                        base_power: 8,
                        xp: 100,
                        on_death: DeathCallback::Monster,
                        looking_right: false,
                    });
                    rat.ai = Some(Ai::Basic);
                    rat
                }
                _ => unreachable!(),
            };
            monster.alive = true;
            objects.push(monster);
        }
    }
    // choose random number of items
    let num_items = rng.gen_range(0, max_items + 1);
    for _ in 0..num_items {
        // choose random spot for this item
        let x = rng.gen_range(room.x1 + 1, room.x2);
        let y = rng.gen_range(room.y1 + 1, room.y2);
        // only place it if the tile is not blocked
        if !is_blocked(x, y, map, objects) {
            let item = match items[item_choice.sample(&mut rng)] {
                Item::Medkit => {
                    let mut object = Object::new(x, y, '\u{90}', "Medkit", COLOR_DARK_RED, false);
                    object.item = Some(Item::Medkit);
                    object
                }
                Item::SlingshotAmmo => {
                    let mut object = Object::new(
                        x,
                        y,
                        '\u{91}',
                        "Bullet For Slingshot",
                        COLOR_DARK_SEPIA,
                        false,
                    );
                    object.item = Some(Item::SlingshotAmmo);
                    object
                }
                Item::BlastingCartridge => {
                    let mut object = Object::new(
                        x,
                        y,
                        '\u{92}',
                        "Blasting Cartridge",
                        COLOR_DARK_SEPIA,
                        false,
                    );
                    object.item = Some(Item::BlastingCartridge);
                    object
                }
                Item::Brick => {
                    let mut object = Object::new(x, y, '\u{93}', "Brick", COLOR_DARK_SEPIA, false);
                    object.item = Some(Item::Brick);
                    object
                }
                Item::Melee => {
                    let mut object = Object::new(x, y, '\u{95}', "Pickaxe", COLOR_DARK_SKY, false);
                    object.item = Some(Item::Melee);
                    object.equipment = Some(Equipment {
                        equipped: false,
                        slot: Slot::Hands,
                        max_hp_bonus: 0,
                        defense_bonus: 0,
                        power_bonus: 3,
                    });
                    object
                }
                Item::Clothing => {
                    let mut object = Object::new(x, y, '\u{96}', "Workwear", COLOR_DARK_SKY, false);
                    object.item = Some(Item::Clothing);
                    object.equipment = Some(Equipment {
                        equipped: false,
                        slot: Slot::Body,
                        max_hp_bonus: 0,
                        defense_bonus: 1,
                        power_bonus: 0,
                    });
                    object
                }
            };
            objects.push(item);
        }
    }
}

/// Advance to the next level
fn next_level(tcod: &mut Tcod, objects: &mut Vec<Object>, game: &mut Game) {
    game.log.add(
        "You take a moment to rest, and recover your strength.",
        COLOR_GREEN,
    );
    let heal_hp = objects[PLAYER].max_hp(game) / 2;
    objects[PLAYER].heal(heal_hp, game);
    game.log.add(
        "After a rare moment of peace, you descend deeper into \
         the heart of the mine...",
        COLOR_ORANGE,
    );
    game.dungeon_level += 1;
    game.map = make_map(objects, game.dungeon_level);
    initialise_fov(&game.map, tcod);
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
fn get_names_under_mouse(
    mouse: input::Mouse,
    objects: &[Object],
    fov_map: &tcod::map::Map,
) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);
    // create a list with the names of all objects at the mouse's coordinates and in FOV
    let names = objects
        .iter()
        .filter(|obj| (obj.pos() == (x, y)) && fov_map.is_in_fov(obj.x, obj.y))
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    if names.is_empty() {
        String::from("nothing out of the ordinary")
    } else {
        names
    }
}

fn render_all(tcod: &mut Tcod, objects: &[Object], game: &mut Game, fov_recompute: bool) {
    if fov_recompute {
        // recompute FOV if needed (the player moved or something)
        let player = &objects[PLAYER];
        tcod.fov
            .compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = tcod.fov.is_in_fov(x, y);
            let wall = game.map[x as usize][y as usize].block_sight;
            let wall_bottom = ((y + 1) < MAP_HEIGHT)
                && wall
                && !game.map[x as usize][(y + 1) as usize].block_sight;
            let ground_sprite = (GROUND_BITMAP & 1usize.rotate_left((x * y) as u32)) != 0;
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
            let explored = &mut game.map[x as usize][y as usize].explored;
            if visible {
                // since it's visible, explore it
                *explored = true;
            }
            if *explored {
                // show explored tiles only (any visible tile is explored already)
                tcod.con.put_char_ex(x, y, glyph, fg, bg);
            }
        }
    }
    let mut to_draw: Vec<_> = objects
        .iter()
        .filter(|o| {
            tcod.fov.is_in_fov(o.x, o.y)
                || (o.always_visible && game.map[o.x as usize][o.y as usize].explored)
        })
        .collect();
    // sort so that non-blocknig objects come first
    to_draw.sort_by(|o1, o2| o1.blocks.cmp(&o2.blocks));
    // draw the objects in the list
    for object in &to_draw {
        object.draw(&mut tcod.con);
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
    for &(ref msg, color) in game.log.iter().rev() {
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
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].max_hp(game);
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
        format!("Mine level: {}", game.dungeon_level),
    );
    // display names of objects under the mouse
    tcod.panel.set_default_foreground(COLOR_LIGHTEST_GREY);
    tcod.panel.print_rect(
        1,
        3,
        BAR_WIDTH,
        0,
        String::from("You see: ") + &get_names_under_mouse(tcod.mouse, objects, &tcod.fov),
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
}

fn player_move_or_attack(dx: i32, dy: i32, objects: &mut [Object], game: &mut Game) {
    // the coordinates the player is moving to/attacking
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;
    if (dy > 0) || ((dy == 0) && (dx < 0)) {
        objects[PLAYER].fighter.as_mut().unwrap().looking_right = false;
    } else if (dy < 0) || ((dy == 0) && (dx > 0)) {
        objects[PLAYER].fighter.as_mut().unwrap().looking_right = true;
    }
    // try to find an attackable object there
    let target_id = objects
        .iter()
        .position(|object| object.fighter.is_some() && object.pos() == (x, y));
    // attack if target found, move otherwise
    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
            player.attack(target, game);
        }
        None => {
            move_by(PLAYER, dx, dy, &game.map, objects);
        }
    }
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

fn inventory_menu(inventory: &[Object], header: &str, root: &mut console::Root) -> Option<usize> {
    // how a menu with each item of the inventory as an option
    let options = if inventory.len() == 0 {
        vec![String::from("Inventory is empty.")]
    } else {
        inventory
            .iter()
            .map(|item| {
                // show additional information, in case it's equipped
                match item.equipment {
                    Some(equipment) if equipment.equipped => {
                        format!("{} (on {})", item.name, equipment.slot)
                    }
                    _ => item.name.clone(),
                }
            })
            .collect()
    };
    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);
    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

fn msgbox(text: &str, width: i32, root: &mut console::Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}

fn handle_keys(
    key: input::Key,
    tcod: &mut Tcod,
    objects: &mut Vec<Object>,
    game: &mut Game,
) -> PlayerAction {
    use input::Key;
    use input::KeyCode::*;
    let player_alive = objects[PLAYER].alive;
    match (key, player_alive) {
        (
            Key {
                code: Enter,
                alt: true,
                ..
            },
            _,
        ) => {
            // Alt+Enter: toggle fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            PlayerAction::DidntTakeTurn
        }
        (Key { code: Escape, .. }, _) => PlayerAction::Exit,
        // movement keys
        (Key { code: Up, .. }, true) | (Key { code: NumPad8, .. }, true) => {
            player_move_or_attack(0, -1, objects, game);
            PlayerAction::TookTurn
        }
        (Key { code: Down, .. }, true) | (Key { code: NumPad2, .. }, true) => {
            player_move_or_attack(0, 1, objects, game);
            PlayerAction::TookTurn
        }
        (Key { code: Left, .. }, true) | (Key { code: NumPad4, .. }, true) => {
            player_move_or_attack(-1, 0, objects, game);
            PlayerAction::TookTurn
        }
        (Key { code: Right, .. }, true) | (Key { code: NumPad6, .. }, true) => {
            player_move_or_attack(1, 0, objects, game);
            PlayerAction::TookTurn
        }
        (Key { code: Home, .. }, true) | (Key { code: NumPad7, .. }, true) => {
            player_move_or_attack(-1, -1, objects, game);
            PlayerAction::TookTurn
        }
        (Key { code: PageUp, .. }, true) | (Key { code: NumPad9, .. }, true) => {
            player_move_or_attack(1, -1, objects, game);
            PlayerAction::TookTurn
        }
        (Key { code: End, .. }, true) | (Key { code: NumPad1, .. }, true) => {
            player_move_or_attack(-1, 1, objects, game);
            PlayerAction::TookTurn
        }
        (Key { code: PageDown, .. }, true) | (Key { code: NumPad3, .. }, true) => {
            player_move_or_attack(1, 1, objects, game);
            PlayerAction::TookTurn
        }
        (Key { code: NumPad5, .. }, true) => {
            PlayerAction::TookTurn // do nothing, i.e. wait for the monster to come to you
        }
        (Key { code: Enter, .. }, true) => {
            // pick up an item or go to next level
            let item_id = objects
                .iter()
                .position(|object| object.pos() == objects[PLAYER].pos() && object.item.is_some());
            let player_on_stairs = objects
                .iter()
                .any(|object| object.pos() == objects[PLAYER].pos() && object.name == "stairs");
            if let Some(item_id) = item_id {
                pick_item_up(item_id, objects, game);
            } else if player_on_stairs {
                next_level(tcod, objects, game);
            };
            PlayerAction::DidntTakeTurn
        }
        (Key { printable: 'i', .. }, true) => {
            // show the inventory: if an item is selected, use it
            let inventory_index = inventory_menu(
                &game.inventory,
                "Press the key next to an item to use it, or any other to cancel.",
                &mut tcod.root,
            );
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, objects, game, tcod);
            }
            PlayerAction::DidntTakeTurn
        }
        (Key { printable: 'd', .. }, true) => {
            let inventory_index = inventory_menu(
                &game.inventory,
                "Press the key next to an item to drop it, or any other to cancel.'",
                &mut tcod.root,
            );
            if let Some(inventory_index) = inventory_index {
                drop_item(inventory_index, objects, game);
            }
            PlayerAction::DidntTakeTurn
        }
        (Key { printable: 'c', .. }, true) => {
            // show character information
            let player = &objects[PLAYER];
            let level = player.level;
            let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
            if let Some(fighter) = player.fighter.as_ref() {
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
                    level,
                    fighter.xp,
                    level_up_xp,
                    player.max_hp(game),
                    player.power(game),
                    player.defense(game)
                );
                msgbox(&msg, CHARACTER_SCREEN_WIDTH, &mut tcod.root);
            }
            PlayerAction::DidntTakeTurn
        }
        (Key { code: F1, .. }, true) => {
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
            PlayerAction::DidntTakeTurn
        }
        _ => PlayerAction::DidntTakeTurn,
    }
}

fn level_up(objects: &mut [Object], game: &mut Game, tcod: &mut Tcod) {
    let player = &mut objects[PLAYER];
    let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
    // see if the player's experience is enough to level-up
    if player.fighter.as_ref().map_or(0, |f| f.xp) >= level_up_xp {
        // it is! level up
        player.level += 1;
        game.log.add(
            format!(
                "Your battle skills grow stronger! You reached level {}!",
                player.level
            ),
            COLOR_ORANGE,
        );
        let fighter = player.fighter.as_mut().unwrap();
        let mut choice = None;
        while choice.is_none() {
            // keep asking until a choice is made
            choice = menu(
                "Level up! Choose a stat to raise:\n",
                &[
                    format!("Constitution (+20 HP, from {})", fighter.base_max_hp),
                    format!("Strength (+1 attack, from {})", fighter.base_power),
                    format!("Agility (+1 defense, from {})", fighter.base_defense),
                ],
                LEVEL_SCREEN_WIDTH,
                &mut tcod.root,
            );
        }
        fighter.xp -= level_up_xp;
        match choice.unwrap() {
            0 => {
                fighter.base_max_hp += 20;
                fighter.hp += 20;
            }
            1 => {
                fighter.base_power += 1;
            }
            2 => {
                fighter.base_defense += 1;
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

fn player_death(player: &mut Object, game: &mut Game) {
    // the game ended!
    game.log.add("You died!", COLOR_DARK_RED);
    // for added effect, transform the player into a corpse!
    player.char = '\u{A3}';
    player.color = COLOR_DARK_RED;
}

fn monster_death(monster: &mut Object, game: &mut Game) {
    // transform it into a nasty corpse! it doesn't block, can't be
    // attacked and doesn't move
    game.log.add(
        format!(
            "{} is dead! You gain {} experience points.",
            monster.name,
            monster.fighter.unwrap().xp
        ),
        COLOR_ORANGE,
    );
    monster.char = '\u{A3}';
    monster.color = COLOR_DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}

struct Tcod {
    root: console::Root,
    con: console::Offscreen,
    panel: console::Offscreen,
    fov: tcod::map::Map,
    mouse: input::Mouse,
}

#[derive(Serialize, Deserialize)]
struct Game {
    map: Map,
    log: Vec<(String, colors::Color)>,
    inventory: Vec<Object>,
    dungeon_level: u32,
}

trait MessageLog {
    fn add(&mut self, message: impl Into<String>, color: colors::Color);
}

impl MessageLog for Vec<(String, colors::Color)> {
    fn add(&mut self, message: impl Into<String>, color: colors::Color) {
        self.push((message.into(), color));
    }
}

fn new_game(tcod: &mut Tcod) -> (Vec<Object>, Game) {
    let mut player = Object::new(0, 0, '\u{80}', "Player", COLOR_GREEN, true);
    player.alive = true;
    player.fighter = Some(Fighter {
        base_max_hp: 30,
        hp: 30,
        base_defense: 1,
        base_power: 2,
        xp: 0,
        on_death: DeathCallback::Player,
        looking_right: false,
    });
    // the list of objects with just the player
    let mut objects = vec![player];
    let level = 1;
    let mut game = Game {
        // generate map (at this point it's not drawn to the screen)
        map: make_map(&mut objects, level),
        // create the list of game messages and their colors, starts empty
        log: vec![],
        inventory: vec![],
        dungeon_level: level,
    };
    // initial equipment: Wooden Board
    let mut pipe = Object::new(0, 0, '\u{94}', "Pipe", COLOR_DARK_SEPIA, false);
    pipe.item = Some(Item::Melee);
    pipe.equipment = Some(Equipment {
        equipped: false,
        slot: Slot::Hands,
        max_hp_bonus: 0,
        defense_bonus: 0,
        power_bonus: 2,
    });
    game.inventory.push(pipe);
    initialise_fov(&game.map, tcod);
    game.log.add(
        "Welcome stranger! Prepare to perish in the Abandoned Mines. Press F1 for help.\n",
        COLOR_ORANGE,
    );
    (objects, game)
}

/// create the FOV map, according to the generated map
fn initialise_fov(map: &Map, tcod: &mut Tcod) {
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(
                x,
                y,
                !map[x as usize][y as usize].block_sight,
                !map[x as usize][y as usize].blocked,
            );
        }
    }
    // unexplored areas start black (which is the default background color)
    tcod.con.clear();
}

fn play_game(objects: &mut Vec<Object>, game: &mut Game, tcod: &mut Tcod) {
    // force FOV "recompute" first time through the game loop
    let mut previous_player_position = (-1, -1);
    while !tcod.root.window_closed() {
        // clear the screen of the previous frame
        tcod.con.clear();
        let mut key = Default::default();
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, input::Event::Key(k))) => key = k,
            Some((_, input::Event::Mouse(m))) => tcod.mouse = m,
            _ => key = Default::default(),
        }
        // render the screen
        let fov_recompute = previous_player_position != (objects[PLAYER].pos());
        render_all(tcod, &objects, game, fov_recompute);
        tcod.root.flush();
        // level up if needed
        level_up(objects, game, tcod);
        // handle keys and exit game if needed
        previous_player_position = objects[PLAYER].pos();
        let player_action = handle_keys(key, tcod, objects, game);
        if player_action == PlayerAction::Exit {
            save_game(objects, game).unwrap();
            break;
        }
        // let monsters take their turn
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, objects, game, &tcod.fov);
                }
            }
        }
    }
}

fn save_game(objects: &[Object], game: &Game) -> Result<(), Box<dyn Error>> {
    let save_data = serde_json::to_string(&(objects, game))?;
    let mut file = fs::File::create("savegame")?;
    file.write_all(save_data.as_bytes())?;
    Ok(())
}

fn load_game() -> Result<(Vec<Object>, Game), Box<dyn Error>> {
    let mut json_save_state = String::new();
    let mut file = fs::File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let result = serde_json::from_str::<(Vec<Object>, Game)>(&json_save_state)?;
    Ok(result)
}

fn main_menu(tcod: &mut Tcod) {
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
                // new game
                let (mut objects, mut game) = new_game(tcod);
                play_game(&mut objects, &mut game, tcod);
            }
            Some(1) => {
                // load game
                match load_game() {
                    Ok((mut objects, mut game)) => {
                        initialise_fov(&game.map, tcod);
                        play_game(&mut objects, &mut game, tcod);
                    }
                    Err(_e) => {
                        msgbox("\nNo saved game to load.\n", 24, &mut tcod.root);
                        continue;
                    }
                }
            }
            Some(2) => {
                // quit
                break;
            }
            _ => {}
        }
    }
}

fn main() {
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
    tcod::system::set_fps(LIMIT_FPS);
    let mut tcod = Tcod {
        root: root,
        con: console::Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        panel: console::Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        fov: tcod::map::Map::new(MAP_WIDTH, MAP_HEIGHT),
        mouse: Default::default(),
    };
    tcod.con.set_default_background(COLOR_DARK_GROUND_BG);
    main_menu(&mut tcod);
}
