use crate::cmtp;
use crate::cmtp::{Ai, Ammo, Equipment, MapObject, Symbol};
use serde::Deserialize;
use std::collections::HashMap;
use std::{error::Error, fs, io, io::Read as _};

#[derive(Debug, Deserialize)]
pub struct Item {
    pub symbol: Symbol,
    pub map_object: MapObject,
    pub item: cmtp::Item,
    pub equipment: Option<Equipment>,
    pub ammo: Option<Ammo>,
    #[serde(default)]
    spawn_chances: Vec<SpawnChance>,
}

#[derive(Debug, Deserialize)]
pub struct Character {
    pub symbol: Symbol,
    pub map_object: MapObject,
    pub character: cmtp::Character,
    pub ai: Option<Ai>,
    #[serde(default)]
    spawn_chances: Vec<SpawnChance>,
}

#[derive(Debug, Deserialize)]
struct SpawnChance {
    from_level: u32,
    probability_weight: u32,
}

pub struct ItemsLoader {
    item_vals: HashMap<String, serde_json::Value>,
}

impl ItemsLoader {
    pub fn load() -> Result<ItemsLoader, Box<dyn Error>> {
        let mut toml_save_state = String::new();
        let mut file = fs::File::open("assets/items.toml")?;
        file.read_to_string(&mut toml_save_state)?;
        let item_vals: HashMap<String, serde_json::Value> = toml::from_str(&toml_save_state)?;
        for (id, item_val) in &item_vals {
            serde_json::from_value::<Item>(item_val.clone()).map_err(|err| {
                io::Error::new(io::ErrorKind::InvalidData, format!("{}: {}", id, err))
            })?;
        }
        Ok(ItemsLoader { item_vals })
    }

    pub fn weighted_table(&self, for_level: u32) -> (Vec<&str>, Vec<u32>) {
        self.item_vals
            .iter()
            .map(|(id, item_val)| {
                let item: Item = serde_json::from_value(item_val.clone()).unwrap();
                let weight = weight_for_level(&item.spawn_chances, for_level);
                (id.as_str(), weight)
            })
            .unzip()
    }

    pub fn get_clone(&self, id: &str) -> Item {
        serde_json::from_value(self.item_vals[id].clone()).unwrap()
    }
}

pub struct CharactersLoader {
    char_vals: HashMap<String, serde_json::Value>,
}

impl CharactersLoader {
    pub fn load() -> Result<CharactersLoader, Box<dyn Error>> {
        let mut toml_save_state = String::new();
        let mut file = fs::File::open("assets/characters.toml")?;
        file.read_to_string(&mut toml_save_state)?;
        let char_vals: HashMap<String, serde_json::Value> = toml::from_str(&toml_save_state)?;
        for (id, char_val) in &char_vals {
            serde_json::from_value::<Character>(char_val.clone()).map_err(|err| {
                io::Error::new(io::ErrorKind::InvalidData, format!("{}: {}", id, err))
            })?;
        }
        Ok(CharactersLoader { char_vals })
    }

    pub fn weighted_table(&self, for_level: u32) -> (Vec<&str>, Vec<u32>) {
        self.char_vals
            .iter()
            .map(|(id, char_val)| {
                let char: Character = serde_json::from_value(char_val.clone()).unwrap();
                let weight = weight_for_level(&char.spawn_chances, for_level);
                (id.as_str(), weight)
            })
            .unzip()
    }

    pub fn get_clone(&self, id: &str) -> Character {
        serde_json::from_value(self.char_vals[id].clone()).unwrap()
    }
}

fn weight_for_level(spawn_chances: &[SpawnChance], lvl: u32) -> u32 {
    spawn_chances
        .iter()
        .rev()
        .find(|chance| chance.from_level <= lvl)
        .map_or(0, |chance| chance.probability_weight)
}

#[cfg(test)]
mod items_loader_tests {
    use super::*;

    #[test]
    fn load_result_is_ok() {
        let result = ItemsLoader::load();
        assert!(result.is_ok(), result.err().unwrap().to_string());
    }

    #[test]
    fn weighted_table_contains_dummy_item() {
        let loader = ItemsLoader::load().unwrap();
        let weighted_table = loader.weighted_table(1);
        assert!(weighted_table.0.contains(&"dummy"));
    }

    #[test]
    fn weighted_table_has_non_zero() {
        let loader = ItemsLoader::load().unwrap();
        for for_level in 1..15 as u32 {
            let weighted_table = loader.weighted_table(for_level);
            let weighted_table_has_non_zero = weighted_table.1.iter().any(|&weight| weight > 0);
            assert!(
                weighted_table_has_non_zero,
                "The table for level {} has only zeros:\n\t{:?}\nthis is a bug.",
                for_level, weighted_table
            );
        }
    }

    #[test]
    fn getting_clone_of_dummy_item() {
        let loader = ItemsLoader::load().unwrap();
        let item = loader.get_clone("dummy");
        assert_eq!(item.map_object.name, "Dummy");
    }
}

#[cfg(test)]
mod characters_loader_tests {
    use super::*;

    #[test]
    fn load_result_is_ok() {
        let result = CharactersLoader::load();
        assert!(result.is_ok(), result.err().unwrap().to_string());
    }

    #[test]
    fn weighted_table_contains_dummy_char() {
        let loader = CharactersLoader::load().unwrap();
        let weighted_table = loader.weighted_table(1);
        assert!(weighted_table.0.contains(&"dummy"));
    }

    #[test]
    fn weighted_table_has_non_zero() {
        let loader = CharactersLoader::load().unwrap();
        for for_level in 1..15 as u32 {
            let weighted_table = loader.weighted_table(for_level);
            let weighted_table_has_non_zero = weighted_table.1.iter().any(|&weight| weight > 0);
            assert!(
                weighted_table_has_non_zero,
                "The table for level {} has only zeros:\n\t{:?}\nthis is a bug.",
                for_level, weighted_table
            );
        }
    }

    #[test]
    fn getting_clone_of_dummy_char() {
        let loader = CharactersLoader::load().unwrap();
        let char = loader.get_clone("dummy");
        assert_eq!(char.map_object.name, "Dummy");
    }
}
