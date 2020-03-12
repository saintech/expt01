use serde::{Deserialize, Serialize};
use tcod::colors;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Player {
    pub id: u32,
    pub dungeon_level: u32,
    pub state: PlayerState,
    pub action: PlayerAction,
    pub looking_at: Option<(i32, i32)>,
    pub previous_player_position: (i32, i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlayerState {
    InDialog,
    MakingTurn,
    TargetingTile(u32),
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState::InDialog
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlayerAction {
    None,
    Cancel,
    SelectMenuItem(usize),
    LookAt(i32, i32),
    ClickAt(i32, i32),
    GoToUp,
    GoToDown,
    GoToLeft,
    GoToRight,
    GoToUpLeft,
    GoToUpRight,
    GoToDownLeft,
    GoToDownRight,
    SkipTurn,
    InteractWithMap,
    OpenHelp,
    OpenInventory,
    OpenCharInfo,
    DropItem,
}

impl Default for PlayerAction {
    fn default() -> Self {
        PlayerAction::None
    }
}

#[serde(default)]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Symbol {
    pub x: i32,
    pub y: i32,
    pub glyph: char,
    pub color: colors::Color,
}

/// A tile of the map and its properties
#[derive(Debug, Serialize, Deserialize)]
pub struct MapCell {
    pub block: bool,
    pub explored: bool,
    pub block_sight: bool,
    pub in_fov: bool,
}

#[serde(default)]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MapObject {
    pub name: String,
    pub block: bool,
    pub always_visible: bool,
    pub hidden: bool,
}

#[serde(default)]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Character {
    pub alive: bool,
    pub level: i32,
    pub hp: i32,
    pub base_max_hp: i32,
    pub base_defense: i32,
    pub base_power: i32,
    pub xp: i32,
    pub on_death: DeathCallback,
    pub looking_right: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DeathCallback {
    None,
    Player,
    Monster,
}

impl Default for DeathCallback {
    fn default() -> Self {
        DeathCallback::None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Ai {
    Basic,
    Confused {
        // TODO: fix this unsized stuff
        previous_ai: Box<Ai>,
        num_turns: i32,
    },
}

#[serde(default)]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AiOption {
    pub option: Option<Ai>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ItemKind {
    Medkit,
    SlingshotAmmo,
    Brick,
    BlastingCartridge,
    Melee,
    Clothing,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    pub kind: ItemKind,
    #[serde(default)]
    pub owner: u32,
}

/// An object that can be equipped, yielding bonuses.
#[derive(Debug, Serialize, Deserialize)]
pub struct Equipment {
    pub slot: Slot,
    #[serde(default)]
    pub equipped: bool,
    pub max_hp_bonus: i32,
    pub defense_bonus: i32,
    pub power_bonus: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Slot {
    Body,
    Hands,
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Slot::Body => write!(f, "body"),
            Slot::Hands => write!(f, "hands"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogMessage(pub String, pub colors::Color);

#[derive(Debug, Serialize, Deserialize)]
pub struct DialogBox {
    pub kind: DialogKind,
    pub header: String,
    pub options: Vec<String>,
    pub width: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DialogKind {
    MainMenu,
    MessageBox,
    Inventory,
    DropItem,
    LevelUp,
}
