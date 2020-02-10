use cfg::*;
use game::*;
use tcod::console;

mod cfg;
mod cmtp;
mod game;
mod systems;

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
        fov: tcod::map::Map::new(1, 1),
        key: Default::default(),
        mouse: Default::default(),
    };
    let mut world = World::default();
    while !tcod.root.window_closed() && !world.must_be_destroyed {
        systems::update_input_state(&mut world, &mut tcod);
        systems::update_main_menu_state(&mut world, &mut tcod);
        systems::update_dungeon_state(&mut world, &mut tcod);
        systems::update_message_box_state(&mut world, &mut tcod);
        systems::update_map_interaction_state(&mut world, &mut tcod);
        systems::player_move_or_attack(&mut world, &mut tcod);
        systems::update_ai_turn_state(&mut world, &mut tcod);
        systems::update_inventory_state(&mut world, &mut tcod);
        systems::update_death_state(&mut world, &mut tcod);
        systems::update_character_state(&mut world, &mut tcod);
        systems::update_stats_menu_state(&mut world, &mut tcod);
        systems::update_help_menu_state(&mut world, &mut tcod);
        systems::update_fov_state(&mut world, &mut tcod);
        systems::update_mouse_look_system(&mut world, &mut tcod);
        systems::render_all(&mut world, &mut tcod);
    }
}
