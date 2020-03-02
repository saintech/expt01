mod cfg;
mod cmtp;
mod engine;
mod systems;

fn main() {
    use engine::game;
    use tcod::console;

    tcod::system::set_fps(cfg::LIMIT_FPS);
    let spritesheet = if tcod::system::get_current_resolution() >= (1920, 1080) {
        "spritesheet-14px-2x.png"
    } else {
        "spritesheet-14px.png"
    };
    let root = console::Root::initializer()
        .font(spritesheet, console::FontLayout::AsciiInRow)
        .font_type(console::FontType::Default)
        .size(cfg::SCREEN_WIDTH, cfg::SCREEN_HEIGHT)
        .title("saintech's experiments: Expt01")
        .init();
    let mut tcod = game::Tcod {
        root: root,
        con: console::Offscreen::new(cfg::MAP_WIDTH, cfg::MAP_HEIGHT),
        panel: console::Offscreen::new(cfg::SCREEN_WIDTH, cfg::PANEL_HEIGHT),
    };
    let mut fov = tcod::map::Map::new(1, 1);
    let mut world: game::World = Default::default();
    while !tcod.root.window_closed() && !world.must_be_destroyed {
        systems::input::update(&mut world);
        systems::main_menu::update(&mut world);
        systems::dungeon::update(&mut world);
        systems::message_box::update(&mut world);
        systems::map_interaction::update(&mut world);
        systems::player_action::update(&mut world);
        systems::ai::update(&mut world);
        systems::inventory::update(&mut world);
        systems::death::update(&mut world);
        systems::character::update(&mut world);
        systems::stats_menu::update(&mut world);
        systems::help_menu::update(&mut world);
        systems::fov::update(&mut world, &mut fov);
        systems::render::update(&mut world, &mut tcod);
    }
}
