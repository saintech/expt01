use crate::cfg;
use crate::cmtp::*;
use serde::{Deserialize, Serialize};
use std::collections::btree_map::BTreeMap;
use tcod::{colors, console};

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityIndexes {
    pub symbol: Option<usize>,
    pub map_cell: Option<usize>,
    pub map_object: Option<usize>,
    pub character: Option<usize>,
    pub ai: Option<usize>,
    pub item: Option<usize>,
    pub equipment: Option<usize>,
    pub log_message: Option<usize>,
    pub dialog: Option<usize>,
}

pub struct Tcod {
    pub root: console::Root,
    pub con: console::Offscreen,
    pub panel: console::Offscreen,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct World {
    pub id_count: u32,
    pub entity_indexes: BTreeMap<u32, EntityIndexes>,
    pub player: Player,
    pub must_be_destroyed: bool,
    pub symbols: Vec<Symbol>,
    pub map: Vec<MapCell>,
    pub map_objects: Vec<MapObject>,
    pub characters: Vec<Character>,
    pub ais: Vec<AiOption>,
    pub items: Vec<OwnedItem>,
    pub equipments: Vec<Equipment>,
    pub log: Vec<LogMessage>,
    pub dialogs: Vec<DialogBox>,
}

impl World {
    pub fn create_entity(
        &mut self,
        symbol: Option<Symbol>,
        map_cell: Option<MapCell>,
        map_object: Option<MapObject>,
        character: Option<Character>,
        ai: Option<AiOption>,
        item: Option<OwnedItem>,
        equipment: Option<Equipment>,
        log_message: Option<LogMessage>,
        dialog: Option<DialogBox>,
    ) -> u32 {
        let entity_indexes = EntityIndexes {
            symbol: symbol.as_ref().map(|_| self.symbols.len()),
            map_cell: map_cell.as_ref().map(|_| self.map.len()),
            map_object: map_object.as_ref().map(|_| self.map_objects.len()),
            character: character.as_ref().map(|_| self.characters.len()),
            ai: ai.as_ref().map(|_| self.ais.len()),
            item: item.as_ref().map(|_| self.items.len()),
            equipment: equipment.as_ref().map(|_| self.equipments.len()),
            log_message: log_message.as_ref().map(|_| self.log.len()),
            dialog: dialog.as_ref().map(|_| self.dialogs.len()),
        };
        symbol.map(|c| self.symbols.push(c));
        map_cell.map(|c| self.map.push(c));
        map_object.map(|c| self.map_objects.push(c));
        character.map(|c| self.characters.push(c));
        ai.map(|c| self.ais.push(c));
        item.map(|c| self.items.push(c));
        equipment.map(|c| self.equipments.push(c));
        log_message.map(|c| self.log.push(c));
        dialog.map(|c| self.dialogs.push(c));
        self.id_count += 1;
        self.entity_indexes.insert(self.id_count, entity_indexes);
        self.id_count
    }

    pub fn player_is_alive(&self) -> bool {
        self.get_character(self.player.id)
            .map_or(false, |(.., char, _)| char.alive)
    }

    pub fn get_character(&self, id: u32) -> Option<(&Symbol, &MapObject, &Character, &AiOption)> {
        let indexes = self
            .entity_indexes
            .get(&id)
            .map(|i| (i.symbol, i.map_object, i.character, i.ai));
        if let Some((Some(si), Some(moi), Some(ci), Some(aii))) = indexes {
            Some((
                &self.symbols[si],
                &self.map_objects[moi],
                &self.characters[ci],
                &self.ais[aii],
            ))
        } else {
            None
        }
    }

    pub fn get_character_mut(
        &mut self,
        id: u32,
    ) -> Option<(&mut Symbol, &mut MapObject, &mut Character, &mut AiOption)> {
        let indexes = self
            .entity_indexes
            .get(&id)
            .map(|i| (i.symbol, i.map_object, i.character, i.ai));
        if let Some((Some(si), Some(moi), Some(ci), Some(aii))) = indexes {
            Some((
                &mut self.symbols[si],
                &mut self.map_objects[moi],
                &mut self.characters[ci],
                &mut self.ais[aii],
            ))
        } else {
            None
        }
    }

    pub fn character_iter(
        &self,
    ) -> impl Iterator<Item = (u32, &Symbol, &MapObject, &Character, &AiOption)> {
        self.entity_indexes.keys().filter_map(move |&id| {
            self.get_character(id)
                .map(|char| (id, char.0, char.1, char.2, char.3))
        })
    }

    pub fn check_fov(&self, id: u32) -> bool {
        let &Symbol { x, y, .. } = self.get_map_obj(id).unwrap().0;
        self.map[(y * cfg::MAP_WIDTH + x) as usize].in_fov
    }

    pub fn get_map_obj(
        &self,
        id: u32,
    ) -> Option<(&Symbol, &MapObject, Option<&Character>, &MapCell)> {
        let indexes = self
            .entity_indexes
            .get(&id)
            .map(|i| (i.symbol, i.map_object, i.character));
        if let Some((Some(si), Some(moi), ci)) = indexes {
            let symbol = &self.symbols[si];
            let index_in_map = (symbol.y * cfg::MAP_WIDTH + symbol.x) as usize;
            Some((
                symbol,
                &self.map_objects[moi],
                ci.map(|ci| &self.characters[ci]),
                &self.map[index_in_map],
            ))
        } else {
            None
        }
    }

    pub fn map_obj_iter(
        &self,
    ) -> impl Iterator<Item = (u32, &Symbol, &MapObject, Option<&Character>, &MapCell)> {
        self.entity_indexes.keys().filter_map(move |&id| {
            self.get_map_obj(id)
                .map(|char| (id, char.0, char.1, char.2, char.3))
        })
    }

    pub fn get_item(
        &self,
        id: u32,
    ) -> Option<(&Symbol, &MapObject, &OwnedItem, Option<&Equipment>)> {
        let indexes = self
            .entity_indexes
            .get(&id)
            .map(|i| (i.symbol, i.map_object, i.item, i.equipment));
        if let Some((Some(si), Some(moi), Some(ii), ei)) = indexes {
            Some((
                &self.symbols[si],
                &self.map_objects[moi],
                &self.items[ii],
                ei.map(|ei| &self.equipments[ei]),
            ))
        } else {
            None
        }
    }

    pub fn get_item_mut(
        &mut self,
        id: u32,
    ) -> Option<(
        &mut Symbol,
        &mut MapObject,
        &mut OwnedItem,
        Option<&mut Equipment>,
    )> {
        let indexes = self
            .entity_indexes
            .get(&id)
            .map(|i| (i.symbol, i.map_object, i.item, i.equipment));
        if let Some((Some(si), Some(moi), Some(ii), ei)) = indexes {
            let equipments = &mut self.equipments;
            Some((
                &mut self.symbols[si],
                &mut self.map_objects[moi],
                &mut self.items[ii],
                ei.map(move |ei| &mut equipments[ei]),
            ))
        } else {
            None
        }
    }

    pub fn item_iter(
        &self,
    ) -> impl Iterator<Item = (u32, &Symbol, &MapObject, &OwnedItem, Option<&Equipment>)> {
        self.entity_indexes.keys().filter_map(move |&id| {
            self.get_item(id)
                .map(|char| (id, char.0, char.1, char.2, char.3))
        })
    }

    pub fn player_char(&self) -> &Character {
        self.get_character(self.player.id)
            .expect("the player has not been created yet")
            .2
    }

    pub fn player_char_mut(&mut self) -> &mut Character {
        self.get_character_mut(self.player.id)
            .expect("the player has not been created yet")
            .2
    }

    pub fn player_sym(&self) -> &Symbol {
        self.get_character(self.player.id)
            .expect("the player has not been created yet")
            .0
    }

    pub fn add_log(&mut self, color: colors::Color, message: impl Into<String>) {
        self.create_entity(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(LogMessage(message.into(), color)),
            None,
        );
    }

    pub fn add_dialog_box(
        &mut self,
        kind: DialogKind,
        header: String,
        options: Vec<String>,
        width: i32,
    ) {
        self.create_entity(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(DialogBox {
                kind,
                header,
                options,
                width,
            }),
        );
    }

    /// returns a list of equipped items
    fn get_all_equipped(&self, owner: u32) -> impl Iterator<Item = &Equipment> {
        self.item_iter().filter_map(move |(.., item, eqp)| {
            eqp.filter(|eqp| (item.owner == owner) && eqp.equipped)
        })
    }

    pub fn power(&self, id: u32) -> i32 {
        let base_power = self
            .get_character(id)
            .map_or(0, |(.., ch, _)| ch.base_power);
        let bonus: i32 = self.get_all_equipped(id).map(|eq| eq.power_bonus).sum();
        base_power + bonus
    }

    pub fn defense(&self, id: u32) -> i32 {
        let base_defense = self
            .get_character(id)
            .map_or(0, |(.., ch, _)| ch.base_defense);
        let bonus: i32 = self.get_all_equipped(id).map(|eq| eq.defense_bonus).sum();
        base_defense + bonus
    }

    pub fn max_hp(&self, id: u32) -> i32 {
        let base_max_hp = self
            .get_character(id)
            .map_or(0, |(.., ch, _)| ch.base_max_hp);
        let bonus: i32 = self.get_all_equipped(id).map(|eq| eq.max_hp_bonus).sum();
        base_max_hp + bonus
    }

    pub fn get_equipped_in_slot(&self, slot: Slot) -> Option<u32> {
        self.item_iter().find_map(|(id, .., eqp)| {
            eqp.filter(|eqp| eqp.equipped && (eqp.slot == slot))
                .and(Some(id))
        })
    }

    fn is_blocked(&self, x: i32, y: i32) -> bool {
        let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
        // first test the map tile
        if self.map[index_in_map].block {
            return true;
        }
        // now check for any blocking objects
        self.map_obj_iter()
            .any(|(_, sym, map_obj, ..)| map_obj.block && ((sym.x, sym.y) == (x, y)))
    }
}

/// return the distance to another object
pub fn distance_to(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> f32 {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
}

pub fn take_damage(target: &mut Character, damage: i32) -> Option<i32> {
    // apply damage if possible
    if damage > 0 {
        target.hp -= damage;
    }
    // check for death, call the death function
    if target.hp <= 0 {
        target.alive = false;
        //fighter.on_death.callback(self, game);
        Some(target.xp)
    } else {
        None
    }
}

pub fn attack_by(attacker_id: u32, target_id: u32, world: &mut World) {
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
pub fn equip(id: u32, world: &mut World) {
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
pub fn move_by(id: u32, dx: i32, dy: i32, world: &mut World) {
    let symbol = world.get_character(id).unwrap().0;
    let (x, y) = (symbol.x, symbol.y);
    if !world.is_blocked(x + dx, y + dy) {
        world.get_character_mut(id).unwrap().0.x = x + dx;
        world.get_character_mut(id).unwrap().0.y = y + dy;
    }
}
