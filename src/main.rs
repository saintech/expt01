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
        systems::input::update(&mut world);
        systems::main_menu::update(&mut world);
        systems::dungeon::update(&mut world);
        systems::message_box::update(&mut world);
        systems::map_interaction::update(&mut world);
        systems::player_action::update(&mut world);
        systems::ai::update(&mut world, &mut tcod);
        systems::inventory::update(&mut world, &mut tcod);
        systems::death::update(&mut world);
        systems::character::update(&mut world);
        systems::stats_menu::update(&mut world);
        systems::help_menu::update(&mut world);
        systems::fov::update(&mut world, &mut tcod);
        systems::mouse_look::update(&mut world);
        systems::render::update(&mut world, &mut tcod);
    }
}
