use crate::cfg;
use crate::cmtp::{Character, DialogBox, DialogKind, PlayerAction, PlayerState};
use crate::game;

fn get_lvl_up_player(world: &mut game::World) -> Option<&mut Character> {
    if (world.player.state == PlayerState::MakingTurn) && world.player_is_alive() {
        Some(world.player_char_mut())
            .filter(|char| char.xp >= cfg::LEVEL_UP_BASE + char.level * cfg::LEVEL_UP_FACTOR)
    } else {
        None
    }
}

fn is_lvl_up_menu(dialog_box: &&DialogBox) -> bool {
    dialog_box.kind == DialogKind::LevelUp
}

pub fn update(world: &mut game::World) {
    let lvl_up_is_open = world.dialogs.last().filter(is_lvl_up_menu).is_some();
    let lvl_up_player = get_lvl_up_player(world);
    if let Some(player) = lvl_up_player {
        // it is! level up
        let new_level = player.level + 1;
        let base_max_hp = player.base_max_hp;
        let base_power = player.base_power;
        let base_defense = player.base_defense;
        let level_up_xp = cfg::LEVEL_UP_BASE + player.level * cfg::LEVEL_UP_FACTOR;
        player.level += 1;
        player.xp -= level_up_xp;
        game::add_log(
            world,
            format!(
                "Your battle skills grow stronger! You reached level {}!",
                new_level,
            ),
            cfg::COLOR_ORANGE,
        );
        let header = String::from("Level up! Choose a stat to raise:\n");
        let options = vec![
            format!("Constitution (+20 HP, from {})", base_max_hp),
            format!("Strength (+1 attack, from {})", base_power),
            format!("Agility (+1 defense, from {})", base_defense),
        ];
        game::add_dialog_box(
            world,
            DialogKind::LevelUp,
            header,
            options,
            cfg::LEVEL_SCREEN_WIDTH,
        );
        world.player.state = PlayerState::InDialog;
    } else if lvl_up_is_open {
        let player_action = world.player.action;
        let player = world.player_char_mut();
        let should_close_dialog = match player_action {
            PlayerAction::SelectMenuItem(0) => {
                player.base_max_hp += 20;
                player.hp += 20;
                true
            }
            PlayerAction::SelectMenuItem(1) => {
                player.base_power += 1;
                true
            }
            PlayerAction::SelectMenuItem(2) => {
                player.base_defense += 1;
                true
            }
            _ => false,
        };
        if should_close_dialog {
            world.dialogs.pop();
            if world.dialogs.is_empty() {
                world.player.state = PlayerState::MakingTurn;
            };
        }
    }
}
