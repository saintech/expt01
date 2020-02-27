use crate::cmtp::{DialogBox, DialogKind, PlayerAction, PlayerState};
use crate::engine::game;

fn is_message_box(dialog_box: &&DialogBox) -> bool {
    dialog_box.kind == DialogKind::MessageBox
}

pub fn update(world: &mut game::World) {
    let message_box_is_open = world.dialogs.last().filter(is_message_box).is_some();
    if message_box_is_open {
        world.player.state = PlayerState::InDialog;
        if world.player.action == PlayerAction::Cancel {
            world.dialogs.pop();
            if world.dialogs.is_empty() {
                world.player.state = PlayerState::MakingTurn;
            }
        }
    }
}
