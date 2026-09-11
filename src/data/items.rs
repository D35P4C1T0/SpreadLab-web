use crate::types::{Item, PokemonType};

pub fn item_boost_type(item: Item) -> Option<PokemonType> {
    use Item::*;
    use PokemonType::*;
    match item {
        SilkScarf => Some(Normal),
        BlackBelt | FistPlate => Some(Fighting),
        BlackGlasses | DreadPlate => Some(Dark),
        Charcoal | FlamePlate => Some(Fire),
        DragonFang | DracoPlate => Some(Dragon),
        HardStone | StonePlate => Some(Rock),
        Magnet | ZapPlate => Some(Electric),
        MetalCoat | IronPlate => Some(Steel),
        MiracleSeed | MeadowPlate => Some(Grass),
        MysticWater | SplashPlate => Some(Water),
        NeverMeltIce | IciclePlate => Some(Ice),
        PoisonBarb | ToxicPlate => Some(Poison),
        SharpBeak | SkyPlate => Some(Flying),
        SilverPowder | InsectPlate => Some(Bug),
        SoftSand | EarthPlate => Some(Ground),
        SpellTag | SpookyPlate => Some(Ghost),
        TwistedSpoon | MindPlate => Some(Psychic),
        FairyFeather => Some(Fairy),
        _ => Option::None,
    }
}

pub fn drive_type(item: Item) -> Option<PokemonType> {
    match item {
        Item::BurnDrive => Some(PokemonType::Fire),
        Item::ChillDrive => Some(PokemonType::Ice),
        Item::DouseDrive => Some(PokemonType::Water),
        Item::ShockDrive => Some(PokemonType::Electric),
        _ => Option::None,
    }
}

pub fn memory_type(item: Item) -> Option<PokemonType> {
    use Item::*;
    use PokemonType::*;
    match item {
        BugMemory => Some(Bug),
        DarkMemory => Some(Dark),
        DragonMemory => Some(Dragon),
        ElectricMemory => Some(Electric),
        FairyMemory => Some(Fairy),
        FightingMemory => Some(Fighting),
        FireMemory => Some(Fire),
        FlyingMemory => Some(Flying),
        GhostMemory => Some(Ghost),
        GrassMemory => Some(Grass),
        GroundMemory => Some(Ground),
        IceMemory => Some(Ice),
        PoisonMemory => Some(Poison),
        PsychicMemory => Some(Psychic),
        RockMemory => Some(Rock),
        SteelMemory => Some(Steel),
        WaterMemory => Some(Water),
        _ => Option::None,
    }
}

pub fn gem_type(item: Item) -> Option<PokemonType> {
    use Item::*;
    use PokemonType::*;
    match item {
        NormalGem => Some(Normal),
        FireGem => Some(Fire),
        WaterGem => Some(Water),
        ElectricGem => Some(Electric),
        GrassGem => Some(Grass),
        IceGem => Some(Ice),
        FightingGem => Some(Fighting),
        PoisonGem => Some(Poison),
        GroundGem => Some(Ground),
        FlyingGem => Some(Flying),
        PsychicGem => Some(Psychic),
        BugGem => Some(Bug),
        RockGem => Some(Rock),
        GhostGem => Some(Ghost),
        DragonGem => Some(Dragon),
        DarkGem => Some(Dark),
        SteelGem => Some(Steel),
        FairyGem => Some(Fairy),
        _ => Option::None,
    }
}

pub fn is_gem(item: Item) -> bool {
    matches!(
        item,
        Item::NormalGem
            | Item::FireGem
            | Item::WaterGem
            | Item::ElectricGem
            | Item::GrassGem
            | Item::IceGem
            | Item::FightingGem
            | Item::PoisonGem
            | Item::GroundGem
            | Item::FlyingGem
            | Item::PsychicGem
            | Item::BugGem
            | Item::RockGem
            | Item::GhostGem
            | Item::DragonGem
            | Item::DarkGem
            | Item::SteelGem
            | Item::FairyGem
    )
}

pub fn berry_resist_type(item: Item) -> Option<PokemonType> {
    use Item::*;
    use PokemonType::*;
    match item {
        ChilanBerry => Some(Normal),
        OccaBerry => Some(Fire),
        PasshoBerry => Some(Water),
        WacanBerry => Some(Electric),
        RindoBerry => Some(Grass),
        YacheBerry => Some(Ice),
        ChopleBerry => Some(Fighting),
        KebiaBerry => Some(Poison),
        ShucaBerry => Some(Ground),
        CobaBerry => Some(Flying),
        PayapaBerry => Some(Psychic),
        TangaBerry => Some(Bug),
        ChartiBerry => Some(Rock),
        KasibBerry => Some(Ghost),
        HabanBerry => Some(Dragon),
        ColburBerry => Some(Dark),
        BabiriBerry => Some(Steel),
        RoseliBerry => Some(Fairy),
        _ => Option::None,
    }
}

pub fn natural_gift(item: Item) -> Option<(PokemonType, u16)> {
    use Item::*;
    use PokemonType::*;
    let gift = match item {
        AguavBerry => (Dragon, 80),
        ApicotBerry => (Ground, 100),
        AspearBerry => (Ice, 80),
        BabiriBerry => (Steel, 80),
        BelueBerry => (Electric, 100),
        BlukBerry => (Fire, 90),
        ChartiBerry => (Rock, 80),
        CheriBerry => (Fire, 80),
        ChestoBerry => (Water, 80),
        ChilanBerry => (Normal, 80),
        ChopleBerry => (Fighting, 80),
        CobaBerry => (Flying, 80),
        ColburBerry => (Dark, 80),
        CornnBerry => (Bug, 90),
        CustapBerry => (Ghost, 100),
        DurinBerry => (Water, 100),
        EnigmaBerry => (Bug, 100),
        FigyBerry => (Bug, 80),
        GanlonBerry => (Ice, 100),
        GrepaBerry => (Flying, 90),
        HabanBerry => (Dragon, 80),
        HondewBerry => (Ground, 90),
        IapapaBerry => (Dark, 80),
        JabocaBerry => (Dragon, 100),
        KasibBerry => (Ghost, 80),
        KebiaBerry => (Poison, 80),
        KeeBerry => (Fairy, 100),
        LansatBerry => (Flying, 100),
        LeppaBerry => (Fighting, 80),
        LiechiBerry => (Grass, 100),
        LumBerry => (Flying, 80),
        MagoBerry => (Ghost, 80),
        MagostBerry => (Rock, 90),
        MarangaBerry => (Dark, 100),
        MicleBerry => (Rock, 100),
        NanabBerry => (Water, 90),
        NomelBerry => (Dragon, 90),
        OccaBerry => (Fire, 80),
        OranBerry => (Poison, 80),
        PamtreBerry => (Steel, 90),
        PasshoBerry => (Water, 80),
        PayapaBerry => (Psychic, 80),
        PechaBerry => (Electric, 80),
        PersimBerry => (Ground, 80),
        PetayaBerry => (Poison, 100),
        PinapBerry => (Grass, 90),
        PomegBerry => (Ice, 90),
        QualotBerry => (Poison, 90),
        RabutaBerry => (Ghost, 90),
        RawstBerry => (Grass, 80),
        RazzBerry => (Steel, 80),
        RindoBerry => (Grass, 80),
        RoseliBerry => (Fairy, 80),
        RowapBerry => (Dark, 100),
        SalacBerry => (Fighting, 100),
        ShucaBerry => (Ground, 80),
        SitrusBerry => (Psychic, 80),
        SpelonBerry => (Dark, 90),
        StarfBerry => (Psychic, 100),
        TamatoBerry => (Psychic, 90),
        TangaBerry => (Bug, 80),
        WacanBerry => (Electric, 80),
        WatmelBerry => (Fire, 100),
        WepearBerry => (Electric, 90),
        WikiBerry => (Rock, 80),
        YacheBerry => (Ice, 80),
        _ => return Option::None,
    };
    Some(gift)
}

pub fn is_berry(item: Item) -> bool {
    natural_gift(item).is_some()
}

pub fn fling_power(item: Item) -> Option<u16> {
    use Item::*;
    let power = match item {
        None => return Option::None,
        IronBall => 130,
        HardStone => 100,
        RockyHelmet | Leek => 60,
        BindingBand | TerrainExtender => 30,
        FlamePlate | SplashPlate | ZapPlate | MeadowPlate | IciclePlate | FistPlate
        | ToxicPlate | EarthPlate | SkyPlate | MindPlate | InsectPlate | StonePlate
        | SpookyPlate | DracoPlate | DreadPlate | IronPlate => 90,
        AssaultVest => 80,
        PoisonBarb | DragonFang => 70,
        UtilityUmbrella => 60,
        SharpBeak => 50,
        BlackBelt | BlackGlasses | Charcoal | LifeOrb | LightBall | Magnet | MetalCoat
        | MiracleSeed | MysticWater | NeverMeltIce | SpellTag | TwistedSpoon | FloatStone
        | ProtectivePads | PunchingGlove | ShellBell | AbilityShield | BoosterEnergy
        | ClearAmulet | AdrenalineOrb => 30,
        CustomFlingPower(power) => power,
        _ => 10,
    };
    Some(power)
}

pub fn can_fling(item: Item, attacker_name: &str, defender_ability: crate::types::Ability) -> bool {
    use crate::types::Ability;
    if matches!(item, Item::None | Item::KlutzSuppressed) || is_gem(item) {
        return false;
    }
    if matches!(defender_ability, Ability::AsOne | Ability::Unnerve) && is_berry(item) {
        return false;
    }
    if attacker_name == "Arceus" && item_boost_type(item).is_some() {
        return false;
    }
    if can_mega(item, attacker_name) {
        return false;
    }
    true
}

pub fn can_mega(item: Item, species: &str) -> bool {
    mega_stone_user(item).is_some_and(|user| user == species)
}

pub fn locked_item_for_species(species: &str) -> Option<Item> {
    use Item::*;
    let item = match species {
        "Mega Absol Z" => AbsoliteZ,
        "Mega Salamence" => Salamencite,
        "Mega Garchomp Z" => GarchompiteZ,
        "Mega Lucario Z" => LucarioniteZ,
        "Mega Golisopod" => Golisopite,
        "Mega Baxcalibur" => Baxcalibrite,
        "Mega Venusaur" => Venusaurite,
        "Mega Charizard X" => CharizarditeX,
        "Mega Charizard Y" => CharizarditeY,
        "Mega Blastoise" => Blastoisinite,
        "Mega Pidgeot" => Pidgeotite,
        "Mega Clefable" => Clefablite,
        "Mega Alakazam" => Alakazite,
        "Mega Victreebel" => Victreebelite,
        "Mega Slowbro" => Slowbronite,
        "Mega Gengar" => Gengarite,
        "Mega Kangaskhan" => Kangaskhanite,
        "Mega Starmie" => Starminite,
        "Mega Pinsir" => Pinsirite,
        "Mega Aerodactyl" => Aerodactylite,
        "Mega Dragonite" => Dragoninite,
        "Mega Meganium" => Meganiumite,
        "Mega Feraligatr" => Feraligite,
        "Mega Ampharos" => Ampharosite,
        "Mega Scizor" => Scizorite,
        "Mega Skarmory" => Skarmorite,
        "Mega Houndoom" => Houndoominite,
        "Mega Tyranitar" => Tyranitarite,
        "Mega Gardevoir" => Gardevoirite,
        "Mega Sableye" => Sablenite,
        "Mega Medicham" => Medichamite,
        "Mega Sharpedo" => Sharpedonite,
        "Mega Camerupt" => Cameruptite,
        "Mega Altaria" => Altarianite,
        "Mega Banette" => Banettite,
        "Mega Chimecho" => Chimechite,
        "Mega Absol" => Absolite,
        "Mega Glalie" => Glalitite,
        "Mega Lopunny" => Lopunnite,
        "Mega Lucario" => Lucarionite,
        "Mega Gallade" => Galladite,
        "Mega Froslass" => Froslassite,
        "Mega Emboar" => Emboarite,
        "Mega Excadrill" => Excadrite,
        "Mega Audino" => Audinite,
        "Mega Chandelure" => Chandelurite,
        "Mega Golurk" => Golurkite,
        "Mega Meowstic" => Meowsticite,
        "Mega Hawlucha" => Hawluchanite,
        "Mega Crabominable" => Crabominite,
        "Mega Drampa" => Drampanite,
        "Mega Scovillain" => Scovillainite,
        "Mega Glimmora" => Glimmoranite,
        _ => return Option::None,
    };
    Some(item)
}

fn mega_stone_user(item: Item) -> Option<&'static str> {
    use Item::*;
    let user = match item {
        AbsoliteZ => "Absol",
        Salamencite => "Salamence",
        GarchompiteZ => "Garchomp",
        LucarioniteZ => "Lucario",
        Golisopite => "Golisopod",
        Baxcalibrite => "Baxcalibur",
        Venusaurite => "Venusaur",
        CharizarditeX | CharizarditeY => "Charizard",
        Blastoisinite => "Blastoise",
        Pidgeotite => "Pidgeot",
        Clefablite => "Clefable",
        Alakazite => "Alakazam",
        Victreebelite => "Victreebel",
        Slowbronite => "Slowbro",
        Gengarite => "Gengar",
        Kangaskhanite => "Kangaskhan",
        Starminite => "Starmie",
        Pinsirite => "Pinsir",
        Aerodactylite => "Aerodactyl",
        Dragoninite => "Dragonite",
        Meganiumite => "Meganium",
        Feraligite => "Feraligatr",
        Ampharosite => "Ampharos",
        Scizorite => "Scizor",
        Skarmorite => "Skarmory",
        Houndoominite => "Houndoom",
        Tyranitarite => "Tyranitar",
        Gardevoirite => "Gardevoir",
        Sablenite => "Sableye",
        Medichamite => "Medicham",
        Sharpedonite => "Sharpedo",
        Cameruptite => "Camerupt",
        Altarianite => "Altaria",
        Banettite => "Banette",
        Chimechite => "Chimecho",
        Absolite => "Absol",
        Glalitite => "Glalie",
        Lopunnite => "Lopunny",
        Lucarionite => "Lucario",
        Galladite => "Gallade",
        Froslassite => "Froslass",
        Emboarite => "Emboar",
        Excadrite => "Excadrill",
        Audinite => "Audino",
        Chandelurite => "Chandelure",
        Golurkite => "Golurk",
        Meowsticite => "Meowstic",
        Hawluchanite => "Hawlucha",
        Crabominite => "Crabominable",
        Drampanite => "Drampa",
        Scovillainite => "Scovillain",
        Glimmoranite => "Glimmora",
        _ => return Option::None,
    };
    Some(user)
}
