use crate::cfg;
use crate::cmtp::{DialogBox, LogMessage, Symbol};
use crate::engine::game;
use tcod::{colors, console, Console as _};

pub fn update(world: &mut game::World, tcod: &mut game::Tcod) {
    if !world.map.is_empty() {
        render_map(world, &mut tcod.con);
        render_map_objects(world, &mut tcod.con);
        // blit the contents of "con" to the root console
        console::blit(
            &tcod.con,
            (0, 0),
            (cfg::MAP_WIDTH, cfg::MAP_HEIGHT),
            &mut tcod.root,
            (0, 0),
            1.0,
            1.0,
        );
    } else {
        render_main_menu_bg(&mut tcod.root);
    }
    if world.get_character(world.player.id).is_some() {
        render_panel(world, &mut tcod.panel);
        // blit the contents of `panel` to the root console
        console::blit(
            &tcod.panel,
            (0, 0),
            (cfg::SCREEN_WIDTH, cfg::PANEL_HEIGHT),
            &mut tcod.root,
            (0, cfg::PANEL_Y),
            1.0,
            1.0,
        );
    }
    render_dialogs(world, &mut tcod.root);
    tcod.root.flush();
}

fn render_map(world: &game::World, con: &mut impl console::Console) {
    let map = &world.map;
    con.set_default_background(cfg::COLOR_DARK_GROUND_BG);
    con.clear();
    for i in 0..map.len() {
        let (x, y) = ((i as i32) % cfg::MAP_WIDTH, (i as i32) / cfg::MAP_WIDTH);
        let visible = map[i].in_fov;
        let wall = map[i].block_sight;
        let wall_bottom = ((y + 1) < cfg::MAP_HEIGHT)
            && wall
            && !map[((y + 1) * cfg::MAP_WIDTH + x) as usize].block_sight;
        let ground_sprite = (cfg::GROUND_BITMAP & 1usize.rotate_left(i as u32)) != 0;
        let ground_bg = if world.player.looking_at == Some((x, y)) {
            cfg::COLOR_LIGHT_GROUND_BG_HIGHLIGHTED
        } else {
            cfg::COLOR_LIGHT_GROUND_BG
        };
        let (fg, bg, glyph) = match (visible, wall, wall_bottom, ground_sprite) {
            // outside of field of view:
            (false, true, false, _) => (cfg::COLOR_DARK_WALL, cfg::COLOR_DARK_WALL_BG, '\u{A0}'),
            (false, true, true, _) => (cfg::COLOR_DARK_WALL, cfg::COLOR_DARK_WALL_BG, '\u{A1}'),
            (false, false, _, false) => (cfg::COLOR_DARK_GROUND, cfg::COLOR_DARK_GROUND_BG, ' '),
            (false, false, _, true) => {
                (cfg::COLOR_DARK_GROUND, cfg::COLOR_DARK_GROUND_BG, '\u{A2}')
            }
            // inside fov:
            (true, true, false, _) => (cfg::COLOR_LIGHT_WALL, cfg::COLOR_LIGHT_WALL_BG, '\u{A0}'),
            (true, true, true, _) => (cfg::COLOR_LIGHT_WALL, cfg::COLOR_LIGHT_WALL_BG, '\u{A1}'),
            (true, false, _, false) => (cfg::COLOR_LIGHT_GROUND, ground_bg, ' '),
            (true, false, _, true) => (cfg::COLOR_LIGHT_GROUND, ground_bg, '\u{A2}'),
        };
        if map[i].explored {
            // show explored tiles only (any visible tile is explored already)
            con.put_char_ex(x, y, glyph, fg, bg);
        }
    }
}

fn render_map_objects(world: &game::World, con: &mut impl console::Console) {
    let mut to_draw: Vec<_> = world
        .map_obj_iter()
        .filter(|(.., map_obj, _, cell)| {
            (cell.in_fov && !map_obj.hidden) || (cell.explored && map_obj.always_visible)
        })
        .collect();
    // sort so that non-blocking objects come first
    to_draw
        .sort_by(|(_, _, map_obj1, ..), (_, _, map_obj2, ..)| map_obj1.block.cmp(&map_obj2.block));
    // draw the objects in the list
    for (_, symbol, _, maybe_char, _) in to_draw {
        let &Symbol { x, y, glyph, color } = symbol;
        con.set_default_foreground(color);
        let glyph = maybe_char
            .filter(|&char| char.looking_right && char.alive)
            .and(Some(std::char::from_u32(symbol.glyph as u32 + 1).unwrap()))
            .unwrap_or(glyph);
        con.put_char(x, y, glyph, console::BackgroundFlag::None);
    }
}

fn render_main_menu_bg(con: &mut impl console::Console) {
    let img = tcod::image::Image::from_file("menu_background.png")
        .ok()
        .expect("Background image not found");
    tcod::image::blit_2x(&img, (0, 0), (-1, -1), con, (0, 0));
    con.set_default_foreground(cfg::COLOR_DARK_RED);
    con.print_ex(
        cfg::SCREEN_WIDTH / 2,
        cfg::SCREEN_HEIGHT / 2 - 4,
        console::BackgroundFlag::None,
        console::TextAlignment::Center,
        "EXPERIMENT 01: ABANDONED MINES",
    );
    con.print_ex(
        cfg::SCREEN_WIDTH / 2,
        cfg::SCREEN_HEIGHT - 2,
        console::BackgroundFlag::None,
        console::TextAlignment::Center,
        "by saintech",
    );
}

fn render_panel(world: &game::World, con: &mut impl console::Console) {
    // prepare to render the GUI panel
    con.set_default_background(cfg::COLOR_DARKEST_GREY);
    con.clear();
    // print the game messages, one line at a time
    let mut y = cfg::MSG_HEIGHT;
    for &LogMessage(ref msg, color) in world.log.iter().rev() {
        let msg_height =
            con.get_height_rect(cfg::MSG_X, cfg::MSG_HEIGHT - y, cfg::MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        con.set_default_foreground(color);
        con.print_rect(cfg::MSG_X, y, cfg::MSG_WIDTH, 0, msg);
    }
    // show the player's stats
    let hp = world.player_char().hp;
    let max_hp = world.max_hp(world.player.id);
    render_bar(
        con,
        1,
        2,
        cfg::BAR_WIDTH,
        "HP",
        hp,
        max_hp,
        cfg::COLOR_DARK_RED,
        cfg::COLOR_DARKEST_SEPIA,
    );
    con.print_ex(
        1,
        1,
        console::BackgroundFlag::None,
        console::TextAlignment::Left,
        format!("Mine level: {}", world.player.dungeon_level),
    );
    // display names of objects under the mouse
    con.set_default_foreground(cfg::COLOR_LIGHTEST_GREY);
    con.print_rect(
        1,
        3,
        cfg::BAR_WIDTH,
        0,
        String::from("You see: ") + &get_names_under_mouse(world),
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
    panel.set_default_foreground(cfg::COLOR_LIGHTEST_GREY);
    panel.print_ex(
        x + total_width / 2,
        y,
        console::BackgroundFlag::None,
        console::TextAlignment::Center,
        &format!("{}: {}/{}", name, value, maximum),
    );
}

/// return a string with the names of all objects under the mouse
fn get_names_under_mouse(world: &game::World) -> String {
    let mut names = vec![];
    if let Some((x, y)) = world.player.looking_at {
        names = world
            .map_obj_iter()
            .filter(|(_, sym, ..)| (sym.x, sym.y) == (x, y))
            .map(|(_, _, map_obj, ..)| map_obj.name.clone())
            .collect();
    }
    let max_len = 4;
    match names.len() {
        0 => String::from("nothing out of the ordinary"),
        l if l >= max_len => {
            names.truncate(max_len - 1);
            names.join(", ") + " and more..."
        }
        _ => names.join(", "),
    }
}

fn render_dialogs(world: &game::World, destination_console: &mut impl console::Console) {
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
            destination_console.get_height_rect(0, 0, width - 2, cfg::SCREEN_HEIGHT - 2, header)
        };
        let height = if options.len() > 0 {
            header_height + options.len() as i32 + 3
        } else {
            header_height + 2
        };
        // create an off-screen console that represents the menu's window
        let mut window = console::Offscreen::new(*width, height);
        window.set_default_background(cfg::COLOR_DARK_SKY);
        window.set_default_foreground(cfg::COLOR_DARKEST_SEPIA);
        window.clear();
        // print the header, with auto-wrap
        window.print_rect(1, 1, width - 1, height, header);
        // print all the options
        for (index, option_text) in options.iter().enumerate() {
            let menu_letter = keys[index] as char;
            let text = format!("[{}] {}", menu_letter, option_text);
            window.print(1, header_height + 2 + index as i32, text);
        }
        let x = cfg::SCREEN_WIDTH / 2 - width / 2;
        let y = cfg::SCREEN_HEIGHT / 2 - height / 2;
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
