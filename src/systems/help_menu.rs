use crate::cmtp::{DialogKind, PlayerAction, PlayerState};
use crate::engine::game;

pub fn update(world: &mut game::World) {
    let should_open_help = (world.player.state == PlayerState::MakingTurn)
        && (world.player.action == PlayerAction::OpenHelp);
    if !should_open_help {
        return;
    }
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
    world.add_dialog_box(DialogKind::MessageBox, String::from(msg), vec![], 36);
}
