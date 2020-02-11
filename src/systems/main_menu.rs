use crate::cmtp::{DialogBox, DialogKind, PlayerAction, PlayerState};
use crate::game;
use std::{error::Error, fs, io::Read as _};

fn is_main_menu(dialog_box: &&DialogBox) -> bool {
    dialog_box.kind == DialogKind::MainMenu
}

pub fn update(world: &mut game::World) {
    let world_is_empty = (world.id_count == 0) && world.entity_indexes.is_empty();
    let menu_is_open = world.dialogs.last().filter(is_main_menu).is_some();
    if world_is_empty {
        let choices: Vec<_> = ["Play a new game", "Continue last game", "Quit"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        game::add_dialog_box(world, DialogKind::MainMenu, String::from(""), choices, 24)
    } else if menu_is_open {
        world.player.state = PlayerState::InDialog;
        match world.player.action {
            // "Play a new game"
            PlayerAction::SelectMenuItem(0) => {
                *world = Default::default();
                world.player.state = PlayerState::MakingTurn;
            }
            // "Continue last game"
            PlayerAction::SelectMenuItem(1) => {
                if load_game(world).is_ok() {
                    world.player.state = PlayerState::MakingTurn;
                } else {
                    let msg = "\nNo saved game to load.\n";
                    game::add_dialog_box(
                        world,
                        DialogKind::MessageBox,
                        String::from(msg),
                        vec![],
                        24,
                    );
                }
            }
            // "Quit"
            PlayerAction::SelectMenuItem(2) | PlayerAction::Cancel => {
                world.must_be_destroyed = true;
            }
            _ => (),
        }
    }
}

fn load_game(world: &mut game::World) -> Result<(), Box<dyn Error>> {
    let mut json_save_state = String::new();
    let mut file = fs::File::open("savegame")?;
    file.read_to_string(&mut json_save_state)?;
    let result = serde_json::from_str::<game::World>(&json_save_state)?;
    *world = result;
    Ok(())
}
