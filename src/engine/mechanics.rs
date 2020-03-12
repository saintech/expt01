use super::game;
use crate::cfg;
use crate::cmtp::Character;

pub fn take_damage(target: &mut Character, damage: i32) -> Option<i32> {
    // apply damage if possible
    if damage > 0 {
        target.hp -= damage;
    }
    // check for death
    if target.hp <= 0 {
        target.alive = false;
        Some(target.xp)
    } else {
        None
    }
}

pub fn attack_by(attacker_id: u32, target_id: u32, world: &mut game::World) {
    let attacker_name = world.get_character(attacker_id).unwrap().1.name.clone();
    let target_name = world.get_character(target_id).unwrap().1.name.clone();
    // a simple formula for attack damage
    let damage = world.power(attacker_id) - world.defense(target_id);
    if damage > 0 {
        world.add_log(
            cfg::COLOR_LIGHTEST_GREY,
            format!(
                "{} attacks {} for {} hit points.",
                attacker_name, target_name, damage
            ),
        );
        let target_char = world.get_character_mut(target_id).unwrap().2;
        if let Some(xp) = take_damage(target_char, damage) {
            // yield experience to the player
            world.get_character_mut(attacker_id).unwrap().2.xp += xp;
        }
    } else {
        world.add_log(
            cfg::COLOR_LIGHTEST_GREY,
            format!(
                "{} attacks {} but it has no effect!",
                attacker_name, target_name
            ),
        );
    }
}

/// Equip object and show a message about it
pub fn equip(id: u32, world: &mut game::World) {
    let name = world.get_item(id).unwrap().1.name.clone();
    let maybe_eqp = world.get_item_mut(id).unwrap().3;
    if let Some(equipment) = maybe_eqp {
        if !equipment.equipped {
            equipment.equipped = true;
            let slot = equipment.slot;
            world.add_log(cfg::COLOR_GREEN, format!("Equipped {} on {}.", name, slot));
        }
    } else {
        world.add_log(
            cfg::COLOR_ORANGE,
            format!("Can't equip {} because it's not an Equipment.", name),
        );
    }
}

/// move by the given amount, if the destination is not blocked
pub fn move_by(id: u32, dx: i32, dy: i32, world: &mut game::World) {
    let symbol = world.get_character(id).unwrap().0;
    let (x, y) = (symbol.x, symbol.y);
    if !world.is_blocked(x + dx, y + dy) {
        world.get_character_mut(id).unwrap().0.x = x + dx;
        world.get_character_mut(id).unwrap().0.y = y + dy;
    }
}
