use std::default::Default;
use std::fmt;
use std::str::FromStr;

use parse_display::{Display, FromStr};
use serde::{Serialize, Deserialize};

use roguelike_utils::comp::*;
use roguelike_utils::math::*;

use roguelike_map::*;

use crate::movement::*;
use crate::config::Config;



#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UseResult {
    pub pos: Option<Pos>,
    pub hit_positions: Vec<Pos>,
}

impl UseResult {
    pub fn new() -> UseResult {
        return UseResult {
            pos: None,
            hit_positions: Vec::new(),
        };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FovBlock {
    Block,
    Transparent,
    Opaque(usize),
    Magnify(usize),
}

#[derive(Clone, Display, FromStr, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[display(style = "snake_case")]
pub enum AttackStyle {
    Stealth,
    Normal,
    Strong,
}

#[derive(Clone, Display, FromStr, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[display(style = "snake_case")]
pub enum WeaponType {
    Blunt,
    Pierce,
    Slash,
}

impl WeaponType {
    pub fn sharp(&self) -> bool {
        return *self == WeaponType::Slash || *self == WeaponType::Pierce;
    }

    pub fn sound_radius(&self, config: &Config) -> usize {
        match self {
            WeaponType::Blunt => {
                return config.sound_radius_blunt;
            },

            WeaponType::Pierce => {
                return config.sound_radius_pierce;
            },

            WeaponType::Slash => {
                return config.sound_radius_slash;
            },
        }
    }

    pub fn stun_turns(&self, config: &Config) -> usize {
        match self {
            WeaponType::Blunt => {
                return config.stun_turns_blunt;
            },

            WeaponType::Pierce => {
                return config.stun_turns_pierce;
            },

            WeaponType::Slash => {
                return config.stun_turns_slash;
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Trap {
    Spikes,
    Sound,
    Blink,
    Freeze,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Display, FromStr, Serialize, Deserialize)]
#[display(style = "snake_case")]
pub enum Skill {
    GrassWall,
    GrassThrow,
    GrassBlade,
    GrassShoes,
    GrassCover,
    Blink,
    Swap,
    Sprint,
    Roll,
    PassWall,
    Rubble,
    StoneThrow,
    StoneSkin,
    Reform,
    Push,
    Illuminate,
    Heal,
    FarSight,
    Ping,
    PassThrough,
    WhirlWind,
    Swift,
}

impl Skill {
    pub fn class(&self) -> EntityClass {
        match self {
            Skill::GrassWall => EntityClass::Grass,
            Skill::GrassThrow => EntityClass::Grass,
            Skill::GrassBlade => EntityClass::Grass,
            Skill::GrassShoes => EntityClass::Grass,
            Skill::GrassCover => EntityClass::Grass,
            Skill::Blink => EntityClass::General,
            Skill::Swap => EntityClass::General,
            Skill::Sprint => EntityClass::General,
            Skill::Roll => EntityClass::General,
            Skill::PassWall => EntityClass::Monolith,
            Skill::Rubble => EntityClass::Monolith,
            Skill::StoneThrow => EntityClass::Monolith,
            Skill::StoneSkin => EntityClass::Monolith,
            Skill::Reform => EntityClass::Monolith,
            Skill::Push => EntityClass::Clockwork,
            Skill::Illuminate => EntityClass::Hierophant,
            Skill::Heal => EntityClass::Hierophant,
            Skill::FarSight => EntityClass::Hierophant,
            Skill::Ping => EntityClass::Hierophant,
            Skill::PassThrough => EntityClass::Wind,
            Skill::WhirlWind => EntityClass::Wind,
            Skill::Swift => EntityClass::Wind,
        }
    }

    pub fn mode(&self) -> SkillMode {
        match self {
            Skill::GrassWall => SkillMode::Direction,
            Skill::GrassThrow => SkillMode::Direction,
            Skill::GrassBlade => SkillMode::Direction,
            Skill::GrassShoes => SkillMode::Immediate,
            Skill::GrassCover => SkillMode::Direction,
            Skill::Blink => SkillMode::Immediate,
            Skill::Swap => SkillMode::Cursor,
            Skill::Sprint => SkillMode::Direction,
            Skill::Roll => SkillMode::Direction,
            Skill::PassWall => SkillMode::Direction,
            Skill::Rubble => SkillMode::Direction,
            Skill::StoneThrow => SkillMode::Direction,
            Skill::StoneSkin => SkillMode::Immediate,
            Skill::Reform => SkillMode::Direction,
            Skill::Push => SkillMode::Direction,
            Skill::Illuminate => SkillMode::Direction,
            Skill::Heal => SkillMode::Immediate,
            Skill::FarSight => SkillMode::Immediate,
            Skill::Ping => SkillMode::Cursor,
            Skill::PassThrough => SkillMode::Direction,
            Skill::WhirlWind => SkillMode::Cursor,
            Skill::Swift => SkillMode::Direction,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillMode {
    Direction,
    Cursor,
    Immediate,
}


#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn white() -> Color {
        return Color::new(255, 255, 255, 255);
    }

    pub fn black() -> Color {
        return Color::new(0, 0, 0, 255);
    }

    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        return Color {r,
                      g,
                      b,
                      a,
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Display, FromStr, Serialize, Deserialize)]
#[display(style="lowercase")]
pub enum GameState {
    Playing,
    Win,
    Lose,
    Inventory,
    SkillMenu,
    ClassMenu,
    HelpMenu,
    ConfirmQuit,
    Use,
    Exit,
}

impl Default for GameState {
    fn default() -> GameState {
        return GameState::Playing;
    }
}

impl GameState {
    pub fn is_menu(self) -> bool {
        return self == GameState::Inventory   ||
               self == GameState::SkillMenu   ||
               self == GameState::ConfirmQuit ||
               self == GameState::HelpMenu    ||
               self == GameState::ClassMenu;
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Display, FromStr, Serialize, Deserialize)]
#[display(style="snake_case")]
pub enum ActionMode {
    Primary,
    Alternate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ItemModifier {
    pub fragile: bool,
    pub modifier: i32,
}

impl ItemModifier {
    pub fn new() -> ItemModifier {
        return ItemModifier { fragile: false, modifier: 0 };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Display, FromStr, Serialize, Deserialize)]
#[display(style = "lowercase")]
pub enum Item {
    Stone,
    Key,
    Dagger,
    Shield,
    Hammer,
    Spear,
    GreatSword,
    Axe,
    Khopesh,
    Sword,
    Lantern,
    Thumper,
    Sling,
    Teleporter,
    Herb,
    SeedOfStone,
    SeedCache,
    SmokeBomb,
    LookingGlass,
    GlassEye,
    SpikeTrap,
    SoundTrap,
    BlinkTrap,
    FreezeTrap,
}

impl Item {
    pub fn class(&self) -> ItemClass {
        match self {
            Item::Stone => ItemClass::Misc,
            Item::Key => ItemClass::Misc,
            Item::Dagger => ItemClass::Primary,
            Item::Shield => ItemClass::Primary,
            Item::Hammer => ItemClass::Primary,
            Item::Spear => ItemClass::Primary,
            Item::GreatSword => ItemClass::Primary,
            Item::Sword => ItemClass::Primary,
            Item::Axe => ItemClass::Primary,
            Item::Khopesh => ItemClass::Primary,
            Item::Sling => ItemClass::Primary,
            Item::Teleporter => ItemClass::Consumable,
            Item::Herb => ItemClass::Consumable,
            Item::SeedOfStone => ItemClass::Consumable,
            Item::SeedCache => ItemClass::Consumable,
            Item::SmokeBomb => ItemClass::Consumable,
            Item::LookingGlass => ItemClass::Consumable,
            Item::GlassEye => ItemClass::Consumable,
            Item::Lantern => ItemClass::Consumable,
            Item::Thumper => ItemClass::Consumable,
            Item::SpikeTrap => ItemClass::Consumable,
            Item::SoundTrap => ItemClass::Consumable,
            Item::BlinkTrap => ItemClass::Consumable,
            Item::FreezeTrap => ItemClass::Consumable,
        }
    }

    pub fn name(&self) -> EntityName {
        match self {
            Item::Stone => EntityName::Stone,
            Item::Key => EntityName::Key,
            Item::Dagger => EntityName::Dagger,
            Item::Shield => EntityName::Shield,
            Item::Hammer => EntityName::Hammer,
            Item::Spear => EntityName::Spear,
            Item::GreatSword => EntityName::GreatSword,
            Item::Sword => EntityName::Sword,
            Item::Axe => EntityName::Axe,
            Item::Khopesh => EntityName::Khopesh,
            Item::Teleporter => EntityName::Teleporter,
            Item::Herb => EntityName::Herb,
            Item::SeedOfStone => EntityName::SeedOfStone,
            Item::SeedCache => EntityName::SeedCache,
            Item::SmokeBomb => EntityName::SmokeBomb,
            Item::LookingGlass => EntityName::LookingGlass,
            Item::GlassEye => EntityName::GlassEye,
            Item::Lantern => EntityName::Lantern,
            Item::Thumper => EntityName::Thumper,
            Item::Sling => EntityName::Sling,
            Item::SpikeTrap => EntityName::SpikeTrap,
            Item::SoundTrap => EntityName::SoundTrap,
            Item::BlinkTrap => EntityName::BlinkTrap,
            Item::FreezeTrap => EntityName::FreezeTrap,
        }
    }

    pub fn weapon_type(&self) -> Option<WeaponType> {
        match self {
            Item::Dagger => Some(WeaponType::Slash),
            Item::Shield => Some(WeaponType::Blunt),
            Item::Hammer => Some(WeaponType::Blunt),
            Item::Spear => Some(WeaponType::Pierce),
            Item::GreatSword => Some(WeaponType::Slash),
            Item::Sword => Some(WeaponType::Slash),
            
            Item::Sling => Some(WeaponType::Blunt),
            Item::Axe => Some(WeaponType::Slash),
            Item::Khopesh => Some(WeaponType::Slash),

            Item::Teleporter => None,
            Item::SeedOfStone => None,
            Item::SeedCache => None,
            Item::SmokeBomb => None,
            Item::LookingGlass => None,
            Item::GlassEye => None,
            Item::Herb => None,
            Item::Stone => None,
            Item::Key => None,
            Item::Lantern => None,
            Item::Thumper => None,
            Item::SpikeTrap => None,
            Item::SoundTrap => None,
            Item::BlinkTrap => None,
            Item::FreezeTrap => None,
        }
    }

    pub fn throw_stun_turns(&self, config: &Config) -> usize {
        match self {
            Item::Stone => config.stun_turns_throw_stone,
            Item::Spear => config.stun_turns_throw_spear,
            _ => config.stun_turns_throw_default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Display, FromStr, Serialize, Deserialize)]
#[display(style="snake_case")]
pub enum ItemClass {
    Primary,
    Consumable,
    Misc,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Display, FromStr)]
#[display(style="snake_case")]
pub enum UseAction {
    #[display("item {0}")]
    Item(ItemClass),
    #[display("skill {0} {1}")]
    Skill(Skill, ActionMode),
    Interact,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hp {
    pub max_hp: i32,
    pub hp: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum InventoryAction {
    Use,
    Drop,
}

impl Default for InventoryAction {
    fn default() -> InventoryAction {
        return InventoryAction::Drop;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Display, FromStr, Serialize, Deserialize)]
#[display(style="snake_case")]
pub enum Stance {
    Crouching,
    Standing,
    Running,
}

impl Stance {
    pub fn waited(&self, move_mode: MoveMode) -> Stance {
        dbg!();
        match (self, move_mode) {
            (Stance::Crouching, MoveMode::Run) => Stance::Standing,
            (Stance::Standing, MoveMode::Sneak) => Stance::Crouching,
            (Stance::Running, _) => Stance::Standing,
            _ => *self,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Display, FromStr, Serialize, Deserialize)]
#[display(style="lowercase")]
pub enum EntityName {
    Player,
    Gol,
    Pawn,
    Rook,
    Column,
    Key,
    Exit,
    Dagger,
    Hammer,
    Spear,
    GreatSword,
    Sword,
    Shield,
    Lantern,
    Thumper,
    Axe,
    Khopesh,
    Sling,
    SeedOfStone,
    SeedCache,
    SmokeBomb,
    LookingGlass,
    GlassEye,
    Teleporter,
    Spire,
    Armil,
    SpikeTrap,
    BlinkTrap,
    FreezeTrap,
    SoundTrap,
    GateTrigger,
    Stone,
    Mouse,
    Cursor,
    Energy,
    Herb,
    Grass,
    Statue,
    Smoke,
    Magnifier,
    Other,
}

impl Default for EntityName {
    fn default() -> EntityName {
        return EntityName::Other;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Display, FromStr, Serialize, Deserialize)]
#[display(style="snake_case")]
pub enum EntityType {
    Player,
    Enemy,
    Item,
    Column,
    Energy,
    Trigger,
    Environment,
    Other,
}

impl Default for EntityType {
    fn default() -> EntityType {
        return EntityType::Other;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Display, FromStr, Serialize, Deserialize)]
#[display(style="snake_case")]
pub enum EntityClass {
    General,
    Grass,
    Monolith,
    Clockwork,
    Hierophant,
    Wind,
}

impl Default for EntityClass {
    fn default() -> EntityClass {
        return EntityClass::General;
    }
}

impl EntityClass {
    pub fn classes() -> Vec<EntityClass> {
        use EntityClass::*;
        return vec!(General, Grass, Monolith, Clockwork, Hierophant, Wind);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Passive {
    pub stone_thrower: bool,
    pub whet_stone: bool,
    pub soft_shoes: bool,
    pub light_touch: bool,
    pub sure_footed: bool,
    pub quick_reflexes: bool,
}

impl Passive {
    pub fn new() -> Passive {
        return Passive::default();
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct StatusEffect {
    pub frozen: usize, // turns
    pub soft_steps: usize, // turns
    pub extra_fov: usize, // amount
    pub blinked: bool,
    pub active: bool,
    pub alive: bool,
    pub stone: usize,
    pub land_roll: bool,
    pub hammer_raised: Option<(EntityId, Direction, usize)>, // item id, direction to hit, turns to wait
    pub test_mode: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Message {
    Sound(Pos),
    Attack(EntityId),
    Hit(Pos),
    Disappeared(EntityId),
}

