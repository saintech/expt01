use super::game;
use crate::cmtp::{
    AiOption, Ammo, Character, DialogBox, Equipment, Item, LogMessage, MapCell, MapObject, Symbol,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Indexes {
    pub symbol: Option<usize>,
    pub map_cell: Option<usize>,
    pub map_object: Option<usize>,
    pub character: Option<usize>,
    pub ai: Option<usize>,
    pub item: Option<usize>,
    pub equipment: Option<usize>,
    pub ammo: Option<usize>,
    pub log_message: Option<usize>,
    pub dialog: Option<usize>,
}

fn create(
    world: &mut game::World,
    symbol: Option<Symbol>,
    map_cell: Option<MapCell>,
    map_object: Option<MapObject>,
    character: Option<Character>,
    ai: Option<AiOption>,
    item: Option<Item>,
    equipment: Option<Equipment>,
    ammo: Option<Ammo>,
    log_message: Option<LogMessage>,
    dialog: Option<DialogBox>,
) -> u32 {
    let entity_indexes = Indexes {
        symbol: symbol.as_ref().map(|_| world.symbols.len()),
        map_cell: map_cell.as_ref().map(|_| world.map.len()),
        map_object: map_object.as_ref().map(|_| world.map_objects.len()),
        character: character.as_ref().map(|_| world.characters.len()),
        ai: ai.as_ref().map(|_| world.ais.len()),
        item: item.as_ref().map(|_| world.items.len()),
        equipment: equipment.as_ref().map(|_| world.equipments.len()),
        ammo: ammo.as_ref().map(|_| world.ammos.len()),
        log_message: log_message.as_ref().map(|_| world.log.len()),
        dialog: dialog.as_ref().map(|_| world.dialogs.len()),
    };
    symbol.map(|c| world.symbols.push(c));
    map_cell.map(|c| world.map.push(c));
    map_object.map(|c| world.map_objects.push(c));
    character.map(|c| world.characters.push(c));
    ai.map(|c| world.ais.push(c));
    item.map(|c| world.items.push(c));
    equipment.map(|c| world.equipments.push(c));
    ammo.map(|c| world.ammos.push(c));
    log_message.map(|c| world.log.push(c));
    dialog.map(|c| world.dialogs.push(c));
    world.id_count += 1;
    world.entity_indexes.insert(world.id_count, entity_indexes);
    world.id_count
}

pub struct Builder {
    symbol: Option<Symbol>,
    map_cell: Option<MapCell>,
    map_object: Option<MapObject>,
    character: Option<Character>,
    ai: Option<AiOption>,
    item: Option<Item>,
    equipment: Option<Equipment>,
    ammo: Option<Ammo>,
    log_message: Option<LogMessage>,
    dialog: Option<DialogBox>,
}

impl Builder {
    pub(super) fn new() -> Self {
        Builder {
            symbol: None,
            map_cell: None,
            map_object: None,
            character: None,
            ai: None,
            item: None,
            equipment: None,
            ammo: None,
            log_message: None,
            dialog: None,
        }
    }

    pub fn create(self, world: &mut game::World) -> u32 {
        create(
            world,
            self.symbol,
            self.map_cell,
            self.map_object,
            self.character,
            self.ai,
            self.item,
            self.equipment,
            self.ammo,
            self.log_message,
            self.dialog,
        )
    }

    pub fn add_symbol(mut self, symbol: Symbol) -> Self {
        self.symbol = Some(symbol);
        self
    }

    pub fn add_map_cell(mut self, map_cell: MapCell) -> Self {
        self.map_cell = Some(map_cell);
        self
    }

    pub fn add_map_object(mut self, map_object: MapObject) -> Self {
        self.map_object = Some(map_object);
        self
    }

    pub fn add_character(mut self, character: Character) -> Self {
        self.character = Some(character);
        self
    }

    pub fn add_ai(mut self, ai: AiOption) -> Self {
        self.ai = Some(ai);
        self
    }

    pub fn add_item(mut self, item: Item) -> Self {
        self.item = Some(item);
        self
    }

    pub fn add_equipment(mut self, equipment: Option<Equipment>) -> Self {
        self.equipment = equipment;
        self
    }

    pub fn add_ammo(mut self, ammo: Option<Ammo>) -> Self {
        self.ammo = ammo;
        self
    }

    pub fn add_log_message(mut self, message: LogMessage) -> Self {
        self.log_message = Some(message);
        self
    }

    pub fn add_dialog(mut self, dialog: DialogBox) -> Self {
        self.dialog = Some(dialog);
        self
    }
}
