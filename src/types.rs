use std::fmt;

use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CalcError {
    #[error("unsupported ruleset: {0:?}")]
    UnsupportedRuleset(Ruleset),
    #[error("unsupported move mechanic for {move_}: {reason}")]
    UnsupportedMechanic { move_: String, reason: &'static str },
    #[error("invalid stat point value {value}; Champions stat points must be 0..=32")]
    InvalidStatPoints { value: u16 },
    #[error("invalid stat stage {value}; stages must be -6..=6")]
    InvalidBoost { value: i8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Ruleset {
    Champions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PokemonType {
    Normal,
    Grass,
    Fire,
    Water,
    Electric,
    Ice,
    Flying,
    Bug,
    Poison,
    Ground,
    Rock,
    Fighting,
    Psychic,
    Ghost,
    Dragon,
    Dark,
    Steel,
    Fairy,
    Stellar,
    Typeless,
}

impl fmt::Display for PokemonType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Normal => "Normal",
            Self::Grass => "Grass",
            Self::Fire => "Fire",
            Self::Water => "Water",
            Self::Electric => "Electric",
            Self::Ice => "Ice",
            Self::Flying => "Flying",
            Self::Bug => "Bug",
            Self::Poison => "Poison",
            Self::Ground => "Ground",
            Self::Rock => "Rock",
            Self::Fighting => "Fighting",
            Self::Psychic => "Psychic",
            Self::Ghost => "Ghost",
            Self::Dragon => "Dragon",
            Self::Dark => "Dark",
            Self::Steel => "Steel",
            Self::Fairy => "Fairy",
            Self::Stellar => "Stellar",
            Self::Typeless => "Typeless",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Category {
    Physical,
    Special,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Stat {
    Hp,
    Attack,
    Defense,
    SpecialAttack,
    SpecialDefense,
    Speed,
}

impl Stat {
    pub const NON_HP: [Stat; 5] = [
        Stat::Attack,
        Stat::Defense,
        Stat::SpecialAttack,
        Stat::SpecialDefense,
        Stat::Speed,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Nature {
    Adamant,
    Bashful,
    Bold,
    Brave,
    Calm,
    Careful,
    Docile,
    Gentle,
    Hardy,
    Hasty,
    Impish,
    Jolly,
    Lax,
    Lonely,
    Mild,
    Modest,
    Naive,
    Naughty,
    Quiet,
    Quirky,
    Rash,
    Relaxed,
    Sassy,
    Serious,
    Timid,
}

impl Nature {
    pub fn increased_stat(self) -> Option<Stat> {
        match self {
            Self::Adamant | Self::Brave | Self::Lonely | Self::Naughty => Some(Stat::Attack),
            Self::Bold | Self::Impish | Self::Lax | Self::Relaxed => Some(Stat::Defense),
            Self::Mild | Self::Modest | Self::Quiet | Self::Rash => Some(Stat::SpecialAttack),
            Self::Calm | Self::Careful | Self::Gentle | Self::Sassy => Some(Stat::SpecialDefense),
            Self::Hasty | Self::Jolly | Self::Naive | Self::Timid => Some(Stat::Speed),
            Self::Bashful | Self::Docile | Self::Hardy | Self::Quirky | Self::Serious => None,
        }
    }

    pub fn decreased_stat(self) -> Option<Stat> {
        match self {
            Self::Bold | Self::Calm | Self::Modest | Self::Timid => Some(Stat::Attack),
            Self::Gentle | Self::Hasty | Self::Lonely | Self::Mild => Some(Stat::Defense),
            Self::Adamant | Self::Careful | Self::Impish | Self::Jolly => Some(Stat::SpecialAttack),
            Self::Lax | Self::Naive | Self::Naughty | Self::Rash => Some(Stat::SpecialDefense),
            Self::Brave | Self::Quiet | Self::Relaxed | Self::Sassy => Some(Stat::Speed),
            Self::Bashful | Self::Docile | Self::Hardy | Self::Quirky | Self::Serious => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum StatusCondition {
    #[default]
    Healthy,
    Burned,
    Paralyzed,
    Poisoned,
    BadlyPoisoned,
    Asleep,
    Drowsy,
    Frozen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum RivalryTarget {
    #[default]
    Unspecified,
    SameGender,
    OppositeGender,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Weather {
    #[default]
    None,
    Sun,
    HarshSun,
    Rain,
    HeavyRain,
    Sand,
    Hail,
    Snow,
    StrongWinds,
}

impl Weather {
    pub fn is_sun(self) -> bool {
        matches!(self, Weather::Sun | Weather::HarshSun)
    }

    pub fn is_rain(self) -> bool {
        matches!(self, Weather::Rain | Weather::HeavyRain)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Terrain {
    #[default]
    None,
    Electric,
    Grassy,
    Misty,
    Psychic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Format {
    Singles,
    #[default]
    Doubles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Ability {
    #[default]
    None,
    Adaptability,
    Aerilate,
    AirLock,
    Analytic,
    ArmorTail,
    AromaVeil,
    AsOne,
    Battery,
    BattleArmor,
    BattleBond,
    BeadsOfRuin,
    Blaze,
    Bulletproof,
    Chlorophyll,
    ClearBody,
    CloudNine,
    Comatose,
    Competitive,
    Contrary,
    Dazzling,
    DauntlessShield,
    Damp,
    Defeatist,
    Defiant,
    Disguise,
    Download,
    Drought,
    Drizzle,
    DragonMaw,
    Dragonize,
    DrySkin,
    EarthEater,
    Eelevate,
    EffectSpore,
    ElectricSurge,
    Electromorphosis,
    EmbodyAspect,
    Filter,
    FairyAura,
    FireMane,
    FlareBoost,
    FlashFire,
    FlowerGift,
    FlowerVeil,
    Fluffy,
    Forecast,
    Forewarn,
    FriendGuard,
    FurCoat,
    FullMetalBody,
    Galvanize,
    Gooey,
    GoodAsGold,
    GrassPelt,
    GuardDog,
    Guts,
    HadronEngine,
    HeavyMetal,
    Heatproof,
    HyperCutter,
    HugePower,
    Hustle,
    IceScales,
    Infiltrator,
    InnerFocus,
    Intimidate,
    IntrepidSword,
    IronFist,
    Klutz,
    LeafGuard,
    Levitate,
    LightningRod,
    Libero,
    LightMetal,
    LiquidVoice,
    LongReach,
    MegaLauncher,
    MegaSol,
    MarvelScale,
    MagicGuard,
    Merciless,
    Mimicry,
    MindEye,
    MirrorArmor,
    MoldBreaker,
    MotorDrive,
    Multiscale,
    Neuroforce,
    NeutralizingGas,
    Normalize,
    Oblivious,
    OrichalcumPulse,
    OwnTempo,
    Overgrow,
    ParentalBond,
    Pixilate,
    PiercingDrill,
    Plus,
    PowerSpot,
    PrismArmor,
    Protosynthesis,
    Protean,
    PunkRock,
    PurePower,
    PurifyingSalt,
    QuickFeet,
    QuarkDrive,
    QueenlyMajesty,
    Rattled,
    Reckless,
    Refrigerate,
    Ripen,
    RockyPayload,
    SandForce,
    SandRush,
    SandSpit,
    SandStream,
    SapSipper,
    Scrappy,
    ScreenCleaner,
    Sharpness,
    ShadowShield,
    ShellArmor,
    SheerForce,
    Simple,
    SlushRush,
    Sniper,
    SolidRock,
    SolarPower,
    Soundproof,
    SnowWarning,
    SpicySpray,
    Stamina,
    Stalwart,
    Stakeout,
    StormDrain,
    Sturdy,
    Steelworker,
    SteelySpirit,
    StrongJaw,
    SupersweetSyrup,
    SupremeOverlord,
    SurgeSurfer,
    Swarm,
    SwiftSwim,
    SwordOfRuin,
    TabletsOfRuin,
    Technician,
    ThickFat,
    ThermalExchange,
    TintedLens,
    TanglingHair,
    Torrent,
    ToughClaws,
    CottonDown,
    TeraShell,
    Trace,
    Transistor,
    Unaware,
    Unnerve,
    UnseenFist,
    VesselOfRuin,
    VoltAbsorb,
    WeakArmor,
    WaterBubble,
    WaterAbsorb,
    WaterVeil,
    Minus,
    WindPower,
    WindRider,
    WonderGuard,
    Stench,
    SpeedBoost,
    Limber,
    SandVeil,
    Static,
    CompoundEyes,
    Insomnia,
    Immunity,
    ShieldDust,
    SuctionCups,
    ShadowTag,
    RoughSkin,
    Synchronize,
    NaturalCure,
    Illuminate,
    PoisonPoint,
    MagmaArmor,
    RainDish,
    Pressure,
    EarlyBird,
    FlameBody,
    KeenEye,
    Pickup,
    CuteCharm,
    StickyHold,
    ShedSkin,
    RockHead,
    VitalSpirit,
    TangledFeet,
    Rivalry,
    Steadfast,
    SnowCloak,
    Gluttony,
    AngerPoint,
    Unburden,
    PoisonHeal,
    SkillLink,
    Hydration,
    NoGuard,
    Stall,
    SuperLuck,
    Aftermath,
    Anticipation,
    IceBody,
    Frisk,
    Pickpocket,
    CursedBody,
    Healer,
    Harvest,
    Telepathy,
    Moody,
    Overcoat,
    PoisonTouch,
    Regenerator,
    BigPecks,
    Illusion,
    Imposter,
    Mummy,
    Moxie,
    Justified,
    MagicBounce,
    Prankster,
    CheekPouch,
    Magician,
    SweetVeil,
    StanceChange,
    GaleWings,
    Symbiosis,
    Berserk,
    Corrosion,
    InnardsOut,
    Receiver,
    WanderingSpirit,
    HungerSwitch,
    QuickDraw,
    CuriousMedicine,
    ZeroToHero,
    Opportunist,
    CudChew,
    ToxicDebris,
    Hospitality,
    WhiteSmoke,
}

impl Ability {
    /// Default state of the browser calculator's ability toggle.
    pub const fn is_on_by_default(self) -> bool {
        !matches!(
            self,
            Self::None
                | Self::FlashFire
                | Self::Plus
                | Self::Minus
                | Self::Trace
                | Self::Stakeout
                | Self::SandSpit
                | Self::BattleBond
                | Self::Electromorphosis
                | Self::WindPower
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Item {
    #[default]
    None,
    AbilityShield,
    AdrenalineOrb,
    AssaultVest,
    AirBalloon,
    BigRoot,
    BrightPowder,
    ChoiceBand,
    ChoiceScarf,
    ChoiceSpecs,
    ClearAmulet,
    BoosterEnergy,
    CornerstoneMask,
    ExpertBelt,
    ElectricSeed,
    Eviolite,
    FloatStone,
    FocusBand,
    GrassySeed,
    HearthflameMask,
    IronBall,
    KingsRock,
    KlutzSuppressed,
    Leftovers,
    LightClay,
    LightBall,
    LifeOrb,
    MentalHerb,
    Metronome,
    MistySeed,
    MuscleBand,
    WiseGlasses,
    PunchingGlove,
    ProtectivePads,
    PsychicSeed,
    RingTarget,
    ScopeLens,
    ShedShell,
    ShellBell,
    SmoothRock,
    UtilityUmbrella,
    WhiteHerb,
    WideLens,
    WellspringMask,
    ZoomLens,
    DampRock,
    HeatRock,
    IcyRock,
    QuickClaw,
    FlamePlate,
    SplashPlate,
    ZapPlate,
    MeadowPlate,
    IciclePlate,
    FistPlate,
    ToxicPlate,
    EarthPlate,
    SkyPlate,
    MindPlate,
    InsectPlate,
    StonePlate,
    SpookyPlate,
    DracoPlate,
    DreadPlate,
    IronPlate,
    SilkScarf,
    BlackBelt,
    BlackGlasses,
    Charcoal,
    DragonFang,
    HardStone,
    Magnet,
    MetalCoat,
    MiracleSeed,
    MysticWater,
    NeverMeltIce,
    PoisonBarb,
    SharpBeak,
    SilverPowder,
    SoftSand,
    SpellTag,
    TwistedSpoon,
    FairyFeather,
    FocusSash,
    Venusaurite,
    Abomasite,
    Aggronite,
    Barbaracite,
    Beedrillite,
    Blazikenite,
    CharizarditeX,
    CharizarditeY,
    Blastoisinite,
    Pidgeotite,
    Clefablite,
    Chesnaughtite,
    Delphoxite,
    Dragalgite,
    Eelektrossite,
    Falinksite,
    Floettite,
    Garchompite,
    Greninjite,
    Gyaradosite,
    Heracronite,
    Malamarite,
    Manectite,
    Mawilite,
    Metagrossite,
    Pyroarite,
    RaichuniteX,
    RaichuniteY,
    Sceptilite,
    Scolipite,
    Scraftinite,
    Staraptite,
    Steelixite,
    Swampertite,
    Alakazite,
    Victreebelite,
    Slowbronite,
    Gengarite,
    Kangaskhanite,
    Starminite,
    Pinsirite,
    Aerodactylite,
    Dragoninite,
    Meganiumite,
    Feraligite,
    Ampharosite,
    Scizorite,
    Skarmorite,
    Houndoominite,
    Tyranitarite,
    Gardevoirite,
    Sablenite,
    Medichamite,
    Sharpedonite,
    Cameruptite,
    Altarianite,
    Banettite,
    Chimechite,
    Absolite,
    Glalitite,
    Lopunnite,
    Lucarionite,
    Galladite,
    Froslassite,
    Emboarite,
    Excadrite,
    Audinite,
    Chandelurite,
    Golurkite,
    Meowsticite,
    Hawluchanite,
    Crabominite,
    Drampanite,
    Scovillainite,
    Glimmoranite,
    BurnDrive,
    ChillDrive,
    DouseDrive,
    ShockDrive,
    BugMemory,
    DarkMemory,
    DragonMemory,
    ElectricMemory,
    FairyMemory,
    FightingMemory,
    FireMemory,
    FlyingMemory,
    GhostMemory,
    GrassMemory,
    GroundMemory,
    IceMemory,
    PoisonMemory,
    PsychicMemory,
    RockMemory,
    SteelMemory,
    WaterMemory,
    AguavBerry,
    ApicotBerry,
    AspearBerry,
    BelueBerry,
    BlukBerry,
    ChilanBerry,
    CheriBerry,
    ChestoBerry,
    OccaBerry,
    PasshoBerry,
    WacanBerry,
    RindoBerry,
    YacheBerry,
    ChopleBerry,
    KebiaBerry,
    ShucaBerry,
    CobaBerry,
    PayapaBerry,
    TangaBerry,
    ChartiBerry,
    KasibBerry,
    HabanBerry,
    ColburBerry,
    BabiriBerry,
    RoseliBerry,
    CornnBerry,
    CustapBerry,
    DurinBerry,
    EnigmaBerry,
    FigyBerry,
    GanlonBerry,
    GrepaBerry,
    HondewBerry,
    IapapaBerry,
    JabocaBerry,
    KeeBerry,
    LansatBerry,
    LeppaBerry,
    LiechiBerry,
    LumBerry,
    MagoBerry,
    MagostBerry,
    MarangaBerry,
    MicleBerry,
    NanabBerry,
    NomelBerry,
    OranBerry,
    PamtreBerry,
    PechaBerry,
    PersimBerry,
    PetayaBerry,
    PinapBerry,
    PomegBerry,
    QualotBerry,
    RabutaBerry,
    RawstBerry,
    RazzBerry,
    RowapBerry,
    SalacBerry,
    SitrusBerry,
    SpelonBerry,
    StarfBerry,
    TamatoBerry,
    WatmelBerry,
    WepearBerry,
    WikiBerry,
    NormalGem,
    FireGem,
    WaterGem,
    ElectricGem,
    GrassGem,
    IceGem,
    FightingGem,
    PoisonGem,
    GroundGem,
    FlyingGem,
    PsychicGem,
    BugGem,
    RockGem,
    GhostGem,
    DragonGem,
    DarkGem,
    SteelGem,
    FairyGem,
    CustomFlingPower(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StatTable {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub special_attack: u16,
    pub special_defense: u16,
    pub speed: u16,
}

impl StatTable {
    pub const fn new(
        hp: u16,
        attack: u16,
        defense: u16,
        special_attack: u16,
        special_defense: u16,
        speed: u16,
    ) -> Self {
        Self {
            hp,
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
        }
    }

    pub fn get(self, stat: Stat) -> u16 {
        match stat {
            Stat::Hp => self.hp,
            Stat::Attack => self.attack,
            Stat::Defense => self.defense,
            Stat::SpecialAttack => self.special_attack,
            Stat::SpecialDefense => self.special_defense,
            Stat::Speed => self.speed,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Boosts {
    pub attack: i8,
    pub defense: i8,
    pub special_attack: i8,
    pub special_defense: i8,
    pub speed: i8,
}

impl Boosts {
    pub fn get(self, stat: Stat) -> i8 {
        match stat {
            Stat::Hp => 0,
            Stat::Attack => self.attack,
            Stat::Defense => self.defense,
            Stat::SpecialAttack => self.special_attack,
            Stat::SpecialDefense => self.special_defense,
            Stat::Speed => self.speed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EffectCount {
    #[default]
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Custom(u8),
}

impl EffectCount {
    pub const UI_OPTIONS: [Self; 7] = [
        Self::Zero,
        Self::One,
        Self::Two,
        Self::Three,
        Self::Four,
        Self::Five,
        Self::Six,
    ];

    pub const fn count(self) -> u8 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
            Self::Five => 5,
            Self::Six => 6,
            Self::Custom(count) => count,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Zero => "0x effect",
            Self::One => "1x effect",
            Self::Two => "2x effect",
            Self::Three => "3x effect",
            Self::Four => "4x effect",
            Self::Five => "5x effect",
            Self::Six => "6x effect",
            Self::Custom(_) => "custom effect count",
        }
    }

    pub const fn is_active(self) -> bool {
        self.count() > 0
    }

    pub const fn base_power_multiplier(self) -> u16 {
        self.count() as u16 + 1
    }

    pub const fn supreme_overlord_index(self) -> Option<usize> {
        let count = self.count();
        if count == 0 {
            None
        } else if count >= 5 {
            Some(4)
        } else {
            Some(count as usize - 1)
        }
    }
}

impl From<u8> for EffectCount {
    fn from(count: u8) -> Self {
        match count {
            0 => Self::Zero,
            1 => Self::One,
            2 => Self::Two,
            3 => Self::Three,
            4 => Self::Four,
            5 => Self::Five,
            6 => Self::Six,
            count => Self::Custom(count),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Pokemon {
    pub name: String,
    pub level: u8,
    pub types: [Option<PokemonType>; 2],
    pub base_stats: StatTable,
    pub stat_points: StatTable,
    pub nature: Nature,
    pub boosts: Boosts,
    pub ability: Ability,
    pub item: Item,
    pub status: StatusCondition,
    /// Current badly-poisoned turn counter. Ignored unless status is badly poisoned.
    pub toxic_counter: u8,
    pub current_hp: Option<u16>,
    pub max_hp_override: Option<u16>,
    pub is_terastalized: bool,
    pub tera_type: Option<PokemonType>,
    pub ability_on: bool,
    pub can_evolve: bool,
    pub supreme_overlord_allies: u8,
    pub paradox_ability_boost: bool,
    pub highest_stat_override: Option<Stat>,
    pub rivalry_target: RivalryTarget,
    pub weight_kg: f32,
    pub custom_bp_mods: Vec<i32>,
    pub custom_attack_mods: Vec<i32>,
    pub custom_defense_mods: Vec<i32>,
    pub custom_final_mods: Vec<i32>,
}

impl Pokemon {
    pub fn champions(
        name: impl Into<String>,
        types: [Option<PokemonType>; 2],
        base_stats: StatTable,
        stat_points: StatTable,
        nature: Nature,
    ) -> Self {
        Self {
            name: name.into(),
            level: 50,
            types,
            base_stats,
            stat_points,
            nature,
            boosts: Boosts::default(),
            ability: Ability::None,
            item: Item::None,
            status: StatusCondition::Healthy,
            toxic_counter: 1,
            current_hp: None,
            max_hp_override: None,
            is_terastalized: false,
            tera_type: None,
            ability_on: false,
            can_evolve: false,
            supreme_overlord_allies: 0,
            paradox_ability_boost: false,
            highest_stat_override: None,
            rivalry_target: RivalryTarget::Unspecified,
            weight_kg: 10.0,
            custom_bp_mods: Vec::new(),
            custom_attack_mods: Vec::new(),
            custom_defense_mods: Vec::new(),
            custom_final_mods: Vec::new(),
        }
    }

    pub fn has_type(&self, type_: PokemonType) -> bool {
        self.types.iter().flatten().any(|t| *t == type_)
    }

    pub fn supreme_overlord_effect(&self) -> EffectCount {
        self.supreme_overlord_allies.into()
    }

    pub fn set_supreme_overlord_effect(&mut self, effect: EffectCount) {
        self.supreme_overlord_allies = effect.count();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Move {
    pub name: String,
    pub base_power: u16,
    pub type_: PokemonType,
    pub category: Category,
    pub deals_physical_damage: bool,
    pub is_spread: bool,
    pub targets_single_target: bool,
    pub is_critical: bool,
    pub makes_contact: bool,
    pub has_secondary_effect: bool,
    pub is_punch: bool,
    pub is_bite: bool,
    pub is_pulse: bool,
    pub is_sound: bool,
    pub is_slice: bool,
    pub is_bullet: bool,
    pub is_wind: bool,
    pub is_priority: bool,
    pub is_ohko: bool,
    pub gets_stellar_boost: bool,
    pub is_z: bool,
    pub is_max: bool,
    pub breaks_protect: bool,
    pub has_recoil: bool,
    pub has_crash: bool,
    pub ignores_screens: bool,
    pub ignores_defense_boosts: bool,
    pub ignores_burn: bool,
    pub is_double_power: bool,
    pub countered_damage_rolls: Option<Vec<u16>>,
    pub countered_move_category: Option<Category>,
    pub countered_move_index: u8,
    pub times_affected: u8,
    pub current_triple_hit: Option<u8>,
    /// Reference move metadata marks this move as having a selectable hit range.
    pub is_multi_hit: bool,
    pub is_parental_bond_child: bool,
    pub hits: u8,
}

impl Move {
    pub fn new(
        name: impl Into<String>,
        base_power: u16,
        type_: PokemonType,
        category: Category,
    ) -> Self {
        Self {
            name: name.into(),
            base_power,
            type_,
            category,
            deals_physical_damage: false,
            is_spread: false,
            targets_single_target: false,
            is_critical: false,
            makes_contact: false,
            has_secondary_effect: false,
            is_punch: false,
            is_bite: false,
            is_pulse: false,
            is_sound: false,
            is_slice: false,
            is_bullet: false,
            is_wind: false,
            is_priority: false,
            is_ohko: false,
            gets_stellar_boost: false,
            is_z: false,
            is_max: false,
            breaks_protect: false,
            has_recoil: false,
            has_crash: false,
            ignores_screens: false,
            ignores_defense_boosts: false,
            ignores_burn: false,
            is_double_power: false,
            countered_damage_rolls: None,
            countered_move_category: None,
            countered_move_index: 0,
            times_affected: 0,
            current_triple_hit: None,
            is_multi_hit: false,
            is_parental_bond_child: false,
            hits: 1,
        }
    }

    pub fn effect_count(&self) -> EffectCount {
        self.times_affected.into()
    }

    pub fn set_effect_count(&mut self, effect: EffectCount) {
        self.times_affected = effect.count();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SideConditions {
    pub reflect: bool,
    pub light_screen: bool,
    pub aurora_veil: bool,
    pub friend_guard: bool,
    pub stealth_rock: bool,
    pub spikes: u8,
    pub salt_cure: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Field {
    pub format: Format,
    pub weather: Weather,
    pub terrain: Terrain,
    pub defender_side: SideConditions,
    pub helping_hand: bool,
    pub battery: bool,
    pub power_spot: bool,
    pub steely_spirit: bool,
    pub flower_gift_attack: bool,
    pub flower_gift_special_defense: bool,
    pub gravity: bool,
    pub protect: bool,
    pub charge: bool,
    pub glaive_rush: bool,
    pub attacker_tailwind: bool,
    pub defender_tailwind: bool,
    pub attacker_swamp: bool,
    pub defender_swamp: bool,
    pub foresight: bool,
    pub attacker_evo_boost: bool,
    pub defender_evo_boost: bool,
    pub attacker_clangorous_soul: bool,
    pub defender_clangorous_soul: bool,
    pub attacker_weakness_policy: bool,
    pub defender_weakness_policy: bool,
    pub tablets_of_ruin: bool,
    pub vessel_of_ruin: bool,
    pub sword_of_ruin: bool,
    pub beads_of_ruin: bool,
    pub neutralizing_gas: bool,
    pub defender_leech_seed: bool,
}

impl Default for Field {
    fn default() -> Self {
        Self {
            format: Format::Doubles,
            weather: Weather::None,
            terrain: Terrain::None,
            defender_side: SideConditions::default(),
            helping_hand: false,
            battery: false,
            power_spot: false,
            steely_spirit: false,
            flower_gift_attack: false,
            flower_gift_special_defense: false,
            gravity: false,
            protect: false,
            charge: false,
            glaive_rush: false,
            attacker_tailwind: false,
            defender_tailwind: false,
            attacker_swamp: false,
            defender_swamp: false,
            foresight: false,
            attacker_evo_boost: false,
            defender_evo_boost: false,
            attacker_clangorous_soul: false,
            defender_clangorous_soul: false,
            attacker_weakness_policy: false,
            defender_weakness_policy: false,
            tablets_of_ruin: false,
            vessel_of_ruin: false,
            sword_of_ruin: false,
            beads_of_ruin: false,
            neutralizing_gas: false,
            defender_leech_seed: false,
        }
    }
}
