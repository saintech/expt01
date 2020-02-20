use crate::cfg;
use crate::cmtp::{DialogKind, PlayerAction, PlayerState};
use crate::game;

pub fn update(world: &mut game::World) {
    let should_open_stats = (world.player.state == PlayerState::MakingTurn)
        && (world.player.action == PlayerAction::OpenCharInfo);
    if !should_open_stats {
        return;
    }
    if world.get_character(world.player.id).is_some() {
        // show character information
        let player = world.player_char();
        let level_up_xp = cfg::LEVEL_UP_BASE + player.level * cfg::LEVEL_UP_FACTOR;
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
            player.level,
            player.xp,
            level_up_xp,
            world.max_hp(world.player.id),
            world.power(world.player.id),
            world.defense(world.player.id),
        );
        world.add_dialog_box(
            DialogKind::MessageBox,
            msg,
            vec![],
            cfg::CHARACTER_SCREEN_WIDTH,
        );
    }
}
