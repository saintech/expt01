use tcod::colors;

// actual size of the window
pub const SCREEN_WIDTH: i32 = 68;
pub const SCREEN_HEIGHT: i32 = 36;
// size of the map
pub const MAP_WIDTH: i32 = 68;
pub const MAP_HEIGHT: i32 = 30;

// sizes and coordinates relevant for the GUI
pub const BAR_WIDTH: i32 = 20;
pub const PANEL_HEIGHT: i32 = 7;
pub const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT + 1;
pub const MSG_X: i32 = BAR_WIDTH + 2;
pub const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 3;
pub const MSG_HEIGHT: i32 = PANEL_HEIGHT - 1;
pub const INVENTORY_WIDTH: i32 = 40;
pub const CHARACTER_SCREEN_WIDTH: i32 = 30;
pub const LEVEL_SCREEN_WIDTH: i32 = 40;

//parameters for dungeon generator
pub const ROOM_MAX_SIZE: i32 = 10;
pub const ROOM_MIN_SIZE: i32 = 6;
pub const MAX_ROOMS: usize = 30;

pub const HEAL_AMOUNT: i32 = 40;
pub const SLINGSHOT_DAMAGE: i32 = 40;
pub const SLINGSHOT_RANGE: i32 = 5;
pub const BRICK_RANGE: i32 = 8;
pub const BRICK_NUM_TURNS: i32 = 10;
pub const BLASTING_RADIUS: i32 = 3;
pub const BLASTING_DAMAGE: i32 = 25;

// experience and level-ups
pub const LEVEL_UP_BASE: i32 = 200;
pub const LEVEL_UP_FACTOR: i32 = 150;

pub const FOV_ALGO: tcod::map::FovAlgorithm = tcod::map::FovAlgorithm::Diamond;
// default FOV algorithm
pub const FOV_LIGHT_WALLS: bool = true;
// light walls or not
pub const TORCH_RADIUS: i32 = 10;

pub const GROUND_BITMAP: usize = 0b100010000101000001010001000000001000101000001010000100010000;

pub const LIMIT_FPS: i32 = 20;

// colors:
pub const COLOR_LIGHTEST_GREY: colors::Color = colors::Color::new(192, 209, 204);
pub const COLOR_DARKEST_GREY: colors::Color = colors::Color::new(20, 24, 23);
//pub const COLOR_LIGHT_SEPIA: colors::Color = colors::Color::new(164, 166, 153);
pub const COLOR_SEPIA: colors::Color = colors::Color::new(129, 122, 119);
pub const COLOR_DARK_SEPIA: colors::Color = colors::Color::new(92, 87, 82);
pub const COLOR_DARKER_SEPIA: colors::Color = colors::Color::new(53, 50, 56);
//pub const COLOR_LIGHT_SKY: colors::Color = colors::Color::new(165, 195, 214);
//pub const COLOR_SKY: colors::Color = colors::Color::new(134, 162, 176);
pub const COLOR_DARK_SKY: colors::Color = colors::Color::new(104, 127, 139);
pub const COLOR_GREEN: colors::Color = colors::Color::new(79, 119, 84);
pub const COLOR_DARK_RED: colors::Color = colors::Color::new(127, 78, 77);
pub const COLOR_ORANGE: colors::Color = colors::Color::new(155, 107, 77);

pub const COLOR_DARK_WALL: colors::Color = COLOR_DARK_SEPIA;
pub const COLOR_DARK_WALL_BG: colors::Color = COLOR_DARKER_SEPIA;
pub const COLOR_LIGHT_WALL: colors::Color = COLOR_SEPIA;
pub const COLOR_LIGHT_WALL_BG: colors::Color = COLOR_DARKER_SEPIA;
pub const COLOR_DARK_GROUND: colors::Color = COLOR_DARKER_SEPIA;
pub const COLOR_DARK_GROUND_BG: colors::Color = COLOR_DARKER_SEPIA;
pub const COLOR_LIGHT_GROUND: colors::Color = COLOR_DARK_SEPIA;
pub const COLOR_LIGHT_GROUND_BG: colors::Color = COLOR_DARKER_SEPIA;

// player will always be the first object
pub const PLAYER_ID: u32 = 1;
