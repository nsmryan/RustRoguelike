use std::default::Default;
use std::fmt;
use std::str::FromStr;

use serde::{Serialize, Deserialize};

use roguelike_utils::comp::*;

use roguelike_map::*;

use crate::movement::*;
use crate::config::Config;



#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ItemUseResult {
    pub pos: Option<Pos>,
    pub hit_positions: Vec<Pos>,
}

impl ItemUseResult {
    pub fn new() -> ItemUseResult {
        return ItemUseResult {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AttackStyle {
    Stealth,
    Normal,
    Strong,
}

impl fmt::Display for AttackStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AttackStyle::Stealth => write!(f, "stealth"),
            AttackStyle::Normal => write!(f, "normal"),
            AttackStyle::Strong => write!(f, "strong"),
        }
    }
}

impl FromStr for AttackStyle {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "stealth" {
            return Ok(AttackStyle::Stealth);
        } else if s == "normal" {
            return Ok(AttackStyle::Normal);
        } else if s == "strong" {
            return Ok(AttackStyle::Strong);
        }

        return Err(format!("Could not parse '{}' as AttackStyle", s));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WeaponType {
    Blunt,
    Pierce,
    Slash,
}

impl fmt::Display for WeaponType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WeaponType::Blunt => write!(f, "blunt"),
            WeaponType::Pierce => write!(f, "pierce"),
            WeaponType::Slash => write!(f, "slash"),
        }
    }
}

impl FromStr for WeaponType {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "blunt" {
            return Ok(WeaponType::Blunt);
        } else if s == "pierce" {
            return Ok(WeaponType::Pierce);
        } else if s == "slash" {
            return Ok(WeaponType::Slash);
        }

        return Err(format!("Could not parse '{}' as WeaponType", s));
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
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

impl fmt::Display for Skill {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Skill::GrassWall => write!(f, "grass_wall"),
            Skill::GrassThrow => write!(f, "grass_throw"),
            Skill::GrassBlade => write!(f, "grass_blade"),
            Skill::GrassShoes => write!(f, "grass_shoes"),
            Skill::GrassCover => write!(f, "grass_cover"),
            Skill::Blink => write!(f, "blink"),
            Skill::PassWall => write!(f, "pass_wall"),
            Skill::Rubble => write!(f, "rubble"),
            Skill::StoneThrow => write!(f, "stone_throw"),
            Skill::Reform => write!(f, "reform"),
            Skill::Swap => write!(f, "swap"),
            Skill::Push => write!(f, "push"),
            Skill::Illuminate => write!(f, "illuminate"),
            Skill::Heal => write!(f, "heal"),
            Skill::FarSight => write!(f, "farsight"),
            Skill::Sprint => write!(f, "sprint"),
            Skill::Roll => write!(f, "roll"),
            Skill::Ping => write!(f, "ping"),
            Skill::StoneSkin => write!(f, "stone_skin"),
            Skill::PassThrough => write!(f, "pass_through"),
            Skill::WhirlWind => write!(f, "whirlwind"),
            Skill::Swift => write!(f, "swift"),
        }
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
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

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameState::Playing => write!(f, "playing"),
            GameState::Win => write!(f, "win"),
            GameState::Lose => write!(f, "lose"),
            GameState::Inventory => write!(f, "inventory"),
            GameState::SkillMenu => write!(f, "skillmenu"),
            GameState::ClassMenu => write!(f, "classmenu"),
            GameState::HelpMenu => write!(f, "helpmenu"),
            GameState::ConfirmQuit => write!(f, "confirmquit"),
            GameState::Use => write!(f, "use"),
            GameState::Exit => write!(f, "exit"),
        }
    }
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


#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ActionMode {
    Primary,
    Alternate,
}

impl fmt::Display for ActionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionMode::Primary => write!(f, "primary"),
            ActionMode::Alternate => write!(f, "alternate"),
        }
    }
}

impl FromStr for ActionMode {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "primary" {
            return Ok(ActionMode::Primary);
        } else if s == "alternate" {
            return Ok(ActionMode::Alternate);
        }

        return Err(format!("Could not parse '{}' as ActionMode", s));
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
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

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::Stone => write!(f, "stone"),
            Item::Key => write!(f, "key"),
            Item::Dagger => write!(f, "dagger"),
            Item::Shield => write!(f, "shield"),
            Item::Hammer => write!(f, "hammer"),
            Item::Spear => write!(f, "spear"),
            Item::GreatSword => write!(f, "greatsword"),
            Item::Sword => write!(f, "sword"),
            Item::Axe => write!(f, "axe"),
            Item::Khopesh => write!(f, "khopesh"),
            Item::Lantern => write!(f, "lantern"),
            Item::Thumper => write!(f, "thumper"),
            Item::Sling => write!(f, "sling"),
            Item::Teleporter => write!(f, "teleporter"),
            Item::Herb => write!(f, "herb"),
            Item::SeedOfStone => write!(f, "seedofstone"),
            Item::SeedCache => write!(f, "seedcache"),
            Item::SmokeBomb => write!(f, "smokebomb"),
            Item::LookingGlass => write!(f, "lookinglass"),
            Item::GlassEye => write!(f, "glasseye"),
            Item::SpikeTrap => write!(f, "spiketrap"),
            Item::SoundTrap => write!(f, "soundtrap"),
            Item::BlinkTrap => write!(f, "blinktrap"),
            Item::FreezeTrap => write!(f, "freezetrap"),
        }
    }
}

impl FromStr for Item {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "stone" {
            return Ok(Item::Stone);
        } else if s == "key" {
            return Ok(Item::Key);
        } else if s == "dagger" {
            return Ok(Item::Dagger);
        } else if s == "shield" {
            return Ok(Item::Shield);
        } else if s == "hammer" {
            return Ok(Item::Hammer);
        } else if s == "spear" {
            return Ok(Item::Spear);
        } else if s == "lantern" {
            return Ok(Item::Lantern);
        } else if s == "thumper" {
            return Ok(Item::Thumper);
        } else if s == "axe" {
            return Ok(Item::Axe);
        } else if s == "khopesh" {
            return Ok(Item::Khopesh);
        } else if s == "sling" {
            return Ok(Item::Sling);
        } else if s == "greatsword" {
            return Ok(Item::GreatSword);
        } else if s == "sword" {
            return Ok(Item::Sword);
        } else if s == "teleporter" {
            return Ok(Item::Teleporter);
        } else if s == "herb" {
            return Ok(Item::Herb);
        } else if s == "seedofstone" {
            return Ok(Item::SeedOfStone);
        } else if s == "seedcache" {
            return Ok(Item::SeedCache);
        } else if s == "smokebomb" {
            return Ok(Item::SmokeBomb);
        } else if s == "lookingglass" {
            return Ok(Item::LookingGlass);
        } else if s == "glasseye" {
            return Ok(Item::GlassEye);
        } else if s == "spiketrap" {
            return Ok(Item::SpikeTrap);
        } else if s == "soundtrap" {
            return Ok(Item::SoundTrap);
        } else if s == "blinktrap" {
            return Ok(Item::BlinkTrap);
        } else if s == "freezetrap" {
            return Ok(Item::FreezeTrap);
        }

        return Err(format!("Could not parse '{}' as Item", s));
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ItemClass {
    Primary,
    Consumable,
    Misc,
}

impl fmt::Display for ItemClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ItemClass::Primary => write!(f, "primary"),
            ItemClass::Consumable => write!(f, "consumable"),
            ItemClass::Misc => write!(f, "misc"),
        }
    }
}

impl FromStr for ItemClass {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "primary" {
            return Ok(ItemClass::Primary);
        } else if s == "consumable" {
            return Ok(ItemClass::Consumable);
        } else if s == "misc" {
            return Ok(ItemClass::Misc);
        }

        return Err(format!("Could not parse '{}' as ItemClass", s));
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum UseAction {
    Item(ItemClass),
    Interact,
}

impl fmt::Display for UseAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UseAction::Item(item_class) => item_class.fmt(f),
            UseAction::Interact => write!(f, "interact"),
        }
    }
}

impl FromStr for UseAction {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "interact" {
            return Ok(UseAction::Interact);
        } else {
            return ItemClass::from_str(string).map(|item_class| UseAction::Item(item_class));
        }
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
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

impl fmt::Display for Stance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Stance::Crouching => write!(f, "crouching"),
            Stance::Standing => write!(f, "standing"),
            Stance::Running => write!(f, "running"),
        }
    }
}

impl FromStr for Stance {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();
        if s == "crouching" {
            return Ok(Stance::Crouching);
        } else if s == "standing" {
            return Ok(Stance::Standing);
        } else if s == "running" {
            return Ok(Stance::Running);
        }

        return Err(format!("Could not parse '{}' as Stance", s));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
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

impl fmt::Display for EntityName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityName::Player => write!(f, "player"),
            EntityName::Gol => write!(f, "gol"),
            EntityName::Pawn => write!(f, "pawn"),
            EntityName::Rook => write!(f, "rook"),
            EntityName::Column => write!(f, "column"),
            EntityName::Key => write!(f, "key"),
            EntityName::Exit => write!(f, "exit"),
            EntityName::Dagger => write!(f, "dagger"),
            EntityName::Hammer => write!(f, "hammer"),
            EntityName::Spear => write!(f, "spear"),
            EntityName::GreatSword => write!(f, "greatsword"),
            EntityName::Axe => write!(f, "axe"),
            EntityName::Khopesh => write!(f, "khopesh"),
            EntityName::Sword => write!(f, "sword"),
            EntityName::Teleporter => write!(f, "teleporter"),
            EntityName::Lantern => write!(f, "lantern"),
            EntityName::Thumper => write!(f, "thumper"),
            EntityName::Sling => write!(f, "sling"),
            EntityName::SeedOfStone => write!(f, "seedofstone"),
            EntityName::SeedCache => write!(f, "seedcache"),
            EntityName::SmokeBomb => write!(f, "smokebomb"),
            EntityName::LookingGlass => write!(f, "lookingglass"),
            EntityName::GlassEye => write!(f, "glasseye"),
            EntityName::Shield => write!(f, "shield"),
            EntityName::Spire => write!(f, "spire"),
            EntityName::Armil => write!(f, "armil"),
            EntityName::SpikeTrap => write!(f, "spiketrap"),
            EntityName::BlinkTrap => write!(f, "blinktrap"),
            EntityName::FreezeTrap => write!(f, "freezetrap"),
            EntityName::SoundTrap => write!(f, "soundtrap"),
            EntityName::GateTrigger => write!(f, "gatetrigger"),
            EntityName::Stone => write!(f, "stone"),
            EntityName::Mouse => write!(f, "mouse"),
            EntityName::Cursor => write!(f, "cursor"),
            EntityName::Energy => write!(f, "energy"),
            EntityName::Herb => write!(f, "herb"),
            EntityName::Grass => write!(f, "grass"),
            EntityName::Statue => write!(f, "statue"),
            EntityName::Smoke => write!(f, "smoke"),
            EntityName::Magnifier => write!(f, "magnifier"),
            EntityName::Other => write!(f, "other"),
        }
    }
}

impl FromStr for EntityName {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();

        if s == "player" {
            return Ok(EntityName::Player);
        } else if s == "gol" {
            return Ok(EntityName::Gol);
        } else if s == "pawn" {
            return Ok(EntityName::Pawn);
        } else if s == "rook" {
            return Ok(EntityName::Rook);
        } else if s == "column" {
            return Ok(EntityName::Column);
        } else if s == "key" {
            return Ok(EntityName::Key);
        } else if s == "exit" {
            return Ok(EntityName::Exit);
        } else if s == "dagger" {
            return Ok(EntityName::Dagger);
        } else if s == "hammer" {
            return Ok(EntityName::Hammer);
        } else if s == "teleporter" {
            return Ok(EntityName::Teleporter);
        } else if s == "spear" {
            return Ok(EntityName::Spear);
        } else if s == "greatsword" {
            return Ok(EntityName::GreatSword);
        } else if s == "sword" {
            return Ok(EntityName::Sword);
        } else if s == "axe" {
            return Ok(EntityName::Axe);
        } else if s == "khopesh" {
            return Ok(EntityName::Khopesh);
        } else if s == "lantern" {
            return Ok(EntityName::Lantern);
        } else if s == "thumper" {
            return Ok(EntityName::Thumper);
        } else if s == "sling" {
            return Ok(EntityName::Sling);
        } else if s == "seedofstone" {
            return Ok(EntityName::SeedOfStone);
        } else if s == "seedcache" {
            return Ok(EntityName::SeedCache);
        } else if s == "smokebomb" {
            return Ok(EntityName::SmokeBomb);
        } else if s == "lookingglass" {
            return Ok(EntityName::LookingGlass);
        } else if s == "glasseye" {
            return Ok(EntityName::GlassEye);
        } else if s == "shield" {
            return Ok(EntityName::Shield);
        } else if s == "spire" {
            return Ok(EntityName::Spire);
        } else if s == "armil" {
            return Ok(EntityName::Armil);
        } else if s == "spiketrap" {
            return Ok(EntityName::SpikeTrap);
        } else if s == "blinktrap" {
            return Ok(EntityName::BlinkTrap);
        } else if s == "freezetrap" {
            return Ok(EntityName::FreezeTrap);
        } else if s == "soundtrap" {
            return Ok(EntityName::SoundTrap);
        } else if s == "gatetrigger" {
            return Ok(EntityName::GateTrigger);
        } else if s == "stone" {
            return Ok(EntityName::Stone);
        } else if s == "mouse" {
            return Ok(EntityName::Mouse);
        } else if s == "cursor" {
            return Ok(EntityName::Cursor);
        } else if s == "energy" {
            return Ok(EntityName::Energy);
        } else if s == "herb" {
            return Ok(EntityName::Herb);
        } else if s == "grass" {
            return Ok(EntityName::Grass);
        } else if s == "statue" {
            return Ok(EntityName::Statue);
        } else if s == "smoke" {
            return Ok(EntityName::Smoke);
        } else if s == "magnifier" {
            return Ok(EntityName::Magnifier);
        } else if s == "other" {
            return Ok(EntityName::Other);
        }

        dbg!(s);
        panic!("EntityName not expected!");
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
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

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityType::Player => write!(f, "player"),
            EntityType::Enemy => write!(f, "enemy"),
            EntityType::Item => write!(f, "item"),
            EntityType::Column => write!(f, "column"),
            EntityType::Energy => write!(f, "energy"),
            EntityType::Trigger => write!(f, "trigger"),
            EntityType::Environment => write!(f, "environment"),
            EntityType::Other => write!(f, "other"),
        }
    }
}

impl FromStr for EntityType {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();
        s.make_ascii_lowercase();

        if s == "player" {
            return Ok(EntityType::Player);
        } else if s == "enemy" {
            return Ok(EntityType::Enemy);
        } else if s == "item" {
            return Ok(EntityType::Item);
        } else if s == "column" {
            return Ok(EntityType::Column);
        } else if s == "energy" {
            return Ok(EntityType::Energy);
        } else if s == "trigger" {
            return Ok(EntityType::Trigger);
        } else if s == "environment" {
            return Ok(EntityType::Environment);
        } else if s == "other" {
            return Ok(EntityType::Other);
        }

        dbg!(s);
        panic!("EntityType not expected!");
    }
}

impl Default for EntityType {
    fn default() -> EntityType {
        return EntityType::Other;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum EntityClass {
    General,
    Grass,
    Monolith,
    Clockwork,
    Hierophant,
    Wind,
}

impl fmt::Display for EntityClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityClass::General => write!(f, "general"),
            EntityClass::Grass => write!(f, "grass"),
            EntityClass::Monolith => write!(f, "monolith"),
            EntityClass::Clockwork => write!(f, "clockword"),
            EntityClass::Hierophant => write!(f, "hierophant"),
            EntityClass::Wind => write!(f, "wind"),
        }
    }
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

