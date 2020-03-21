use super::entity;
use crate::cfg;
use crate::cmtp::{
    AiOption, Ammo, Character, DialogBox, DialogKind, Equipment, Item, LogMessage, MapCell,
    MapObject, Player, Slot, Symbol,
};
use serde::{Deserialize, Serialize};
use std::collections::btree_map::BTreeMap;
use tcod::{colors, console};

pub struct Tcod {
    pub root: console::Root,
    pub con: console::Offscreen,
    pub panel: console::Offscreen,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct World {
    pub id_count: u32,
    pub entity_indexes: BTreeMap<u32, entity::Indexes>,
    pub player: Player,
    pub must_be_destroyed: bool,
    pub symbols: Vec<Symbol>,
    pub map: Vec<MapCell>,
    pub map_objects: Vec<MapObject>,
    pub characters: Vec<Character>,
    pub ais: Vec<AiOption>,
    pub items: Vec<Item>,
    pub equipments: Vec<Equipment>,
    pub ammos: Vec<Ammo>,
    pub log: Vec<LogMessage>,
    pub dialogs: Vec<DialogBox>,
}

impl World {
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
    ) -> Option<(
        &Symbol,
        &MapObject,
        &Item,
        Option<&Equipment>,
        Option<&Ammo>,
    )> {
        let indexes = self
            .entity_indexes
            .get(&id)
            .map(|i| (i.symbol, i.map_object, i.item, i.equipment, i.ammo));
        if let Some((Some(si), Some(moi), Some(ii), ei, ai)) = indexes {
            Some((
                &self.symbols[si],
                &self.map_objects[moi],
                &self.items[ii],
                ei.map(|ei| &self.equipments[ei]),
                ai.map(|ai| &self.ammos[ai]),
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
        &mut Item,
        Option<&mut Equipment>,
        Option<&mut Ammo>,
    )> {
        let indexes = self
            .entity_indexes
            .get(&id)
            .map(|i| (i.symbol, i.map_object, i.item, i.equipment, i.ammo));
        if let Some((Some(si), Some(moi), Some(ii), ei, ai)) = indexes {
            let equipments = &mut self.equipments;
            let ammos = &mut self.ammos;
            Some((
                &mut self.symbols[si],
                &mut self.map_objects[moi],
                &mut self.items[ii],
                ei.map(move |ei| &mut equipments[ei]),
                ai.map(move |ai| &mut ammos[ai]),
            ))
        } else {
            None
        }
    }

    pub fn item_iter(
        &self,
    ) -> impl Iterator<
        Item = (
            u32,
            &Symbol,
            &MapObject,
            &Item,
            Option<&Equipment>,
            Option<&Ammo>,
        ),
    > {
        self.entity_indexes.keys().filter_map(move |&id| {
            self.get_item(id)
                .map(|item| (id, item.0, item.1, item.2, item.3, item.4))
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
        let msg = message.into();
        println!("game log: \"{}\"", &msg);
        new_entity()
            .add_log_message(LogMessage(msg, color))
            .create(self);
    }

    pub fn add_dialog_box(
        &mut self,
        kind: DialogKind,
        header: String,
        options: Vec<String>,
        width: i32,
    ) {
        new_entity()
            .add_dialog(DialogBox {
                kind,
                header,
                options,
                width,
            })
            .create(self);
    }

    /// returns a list of equipped items
    fn get_all_equipped(&self, owner: u32) -> impl Iterator<Item = &Equipment> {
        self.item_iter().filter_map(move |(.., item, eqp, _)| {
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
        self.item_iter().find_map(|(id, .., itm, eqp, _)| {
            eqp.filter(|_| itm.owner == self.player.id)
                .filter(|eqp| eqp.equipped && (eqp.slot == slot))
                .and(Some(id))
        })
    }

    pub fn is_blocked(&self, x: i32, y: i32) -> bool {
        let index_in_map = (y * cfg::MAP_WIDTH + x) as usize;
        // first test the map tile
        if self.map[index_in_map].block {
            return true;
        }
        // now check for any blocking objects
        self.map_obj_iter()
            .any(|(_, sym, map_obj, ..)| map_obj.block && ((sym.x, sym.y) == (x, y)))
    }

    /// return the distance to another object
    pub fn distance_to(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> f32 {
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }
}

pub fn new_entity() -> entity::Builder {
    entity::Builder::new()
}
