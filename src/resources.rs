//! Names for the game's art, from `components/resources.lua`.
//!
//! Identifiers only. Loading the PNGs, padding them to powers of two and
//! choosing filtering modes is renderer work for phase 4; gameplay just needs to
//! say which image an actor wears, and the Lua does that by name too.

/// Which of the four players a per-player sprite belongs to. 1-based, matching
/// the Lua's `sprites_table[player]` indexing.
pub type Player = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpriteId {
    Loading,
    Title,
    Credits,
    AButton,

    Fighter(Player),
    Bomber(Player),
    Frigate(Player),
    Factory(Player),

    /// Neutral factory art, used by the player-select ships and the CPU.
    FactorySprite,
    FactoryLightDamage(usize),
    FactoryHeavyDamage(usize),

    Number(usize),

    ProductionLayer(usize),
    Needle,
    FighterPreview,
    BomberPreview,
    FrigatePreview,
    UpgradePreview,

    HealthFull,
    HealthSome,
    HealthNone,

    Laser,
    Bomb,
    Missile,

    Bubble,
    BigBubble,
    Explosion,
    Spark,

    Debris(usize),
    Fish(usize),
}

/// Where the sprite's own origin sits, in pixels from its bottom-left corner.
/// `graphics.sprite_from_image`'s third argument.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Origin {
    Center,
    At(f32, f32),
}

/// Everything the renderer needs to know about an image that is not in the file
/// itself: where it is, where its origin is, and whether it is one of the
/// sprites `components/resources.lua` smooths after loading.
#[derive(Clone, Copy, Debug)]
pub struct SpriteMeta {
    pub path: &'static str,
    pub origin: Origin,
    /// `smoothen()` in the Lua: linear filtering instead of nearest. Applied to
    /// the things that move or scale continuously.
    pub smooth: bool,
}

const fn centered(path: &'static str) -> SpriteMeta {
    SpriteMeta {
        path,
        origin: Origin::Center,
        smooth: false,
    }
}

const fn smooth(path: &'static str) -> SpriteMeta {
    SpriteMeta {
        path,
        origin: Origin::Center,
        smooth: true,
    }
}

/// Per-player art: `sprites/fighter_p1.png` and friends, 1-based like the Lua.
fn per_player(stem: &'static str, player: Player) -> &'static str {
    // the table is tiny and fixed, so the paths are literals rather than
    // formatted at load time
    match (stem, player) {
        ("fighter", 1) => "sprites/fighter_p1.png",
        ("fighter", 2) => "sprites/fighter_p2.png",
        ("fighter", 3) => "sprites/fighter_p3.png",
        ("fighter", _) => "sprites/fighter_p4.png",
        ("bomber", 1) => "sprites/bomber_p1.png",
        ("bomber", 2) => "sprites/bomber_p2.png",
        ("bomber", 3) => "sprites/bomber_p3.png",
        ("bomber", _) => "sprites/bomber_p4.png",
        ("frigate", 1) => "sprites/frigate_p1.png",
        ("frigate", 2) => "sprites/frigate_p2.png",
        ("frigate", 3) => "sprites/frigate_p3.png",
        ("frigate", _) => "sprites/frigate_p4.png",
        (_, 1) => "sprites/factory_p1.png",
        (_, 2) => "sprites/factory_p2.png",
        (_, 3) => "sprites/factory_p3.png",
        (_, _) => "sprites/factory_p4.png",
    }
}

impl SpriteId {
    /// The file, origin and filtering `components/resources.lua` gives this
    /// image.
    pub fn meta(self) -> SpriteMeta {
        match self {
            SpriteId::Loading => SpriteMeta {
                path: "sprites/loading.png",
                origin: Origin::At(113.0, -10.0),
                smooth: false,
            },
            SpriteId::Title => centered("sprites/title.png"),
            SpriteId::Credits => centered("sprites/credits.png"),
            SpriteId::AButton => centered("sprites/a_button.png"),

            SpriteId::Fighter(p) => centered(per_player("fighter", p)),
            SpriteId::Bomber(p) => centered(per_player("bomber", p)),
            SpriteId::Frigate(p) => centered(per_player("frigate", p)),
            SpriteId::Factory(p) => smooth(per_player("factory", p)),
            SpriteId::FactorySprite => smooth(per_player("factory", 1)),

            SpriteId::FactoryLightDamage(1) => centered("sprites/factory_light_damage_1.png"),
            SpriteId::FactoryLightDamage(2) => centered("sprites/factory_light_damage_2.png"),
            SpriteId::FactoryLightDamage(_) => centered("sprites/factory_light_damage_3.png"),
            SpriteId::FactoryHeavyDamage(1) => centered("sprites/factory_heavy_damage_1.png"),
            SpriteId::FactoryHeavyDamage(2) => centered("sprites/factory_heavy_damage_2.png"),
            SpriteId::FactoryHeavyDamage(_) => centered("sprites/factory_heavy_damage_3.png"),

            SpriteId::Number(1) => centered("sprites/1.png"),
            SpriteId::Number(2) => centered("sprites/2.png"),
            SpriteId::Number(3) => centered("sprites/3.png"),
            SpriteId::Number(4) => centered("sprites/4.png"),
            SpriteId::Number(_) => centered("sprites/5.png"),

            SpriteId::ProductionLayer(1) => smooth("sprites/production1.png"),
            SpriteId::ProductionLayer(2) => smooth("sprites/production2.png"),
            SpriteId::ProductionLayer(_) => smooth("sprites/production3.png"),
            SpriteId::Needle => smooth("sprites/needle.png"),
            SpriteId::FighterPreview => smooth("sprites/fighter_outline.png"),
            SpriteId::BomberPreview => smooth("sprites/bomber_outline.png"),
            SpriteId::FrigatePreview => smooth("sprites/frigate_outline.png"),
            SpriteId::UpgradePreview => smooth("sprites/upgrade_outline.png"),

            SpriteId::HealthFull => smooth("sprites/health_full.png"),
            SpriteId::HealthSome => smooth("sprites/health_some.png"),
            SpriteId::HealthNone => smooth("sprites/health_none.png"),

            SpriteId::Laser => centered("sprites/laser.png"),
            SpriteId::Bomb => centered("sprites/bomb.png"),
            SpriteId::Missile => centered("sprites/missile.png"),

            SpriteId::Bubble => centered("sprites/bubble.png"),
            SpriteId::BigBubble => centered("sprites/big_bubble.png"),
            SpriteId::Explosion => centered("sprites/explosion.png"),
            SpriteId::Spark => centered("sprites/spark.png"),

            SpriteId::Debris(1) => centered("sprites/debris_large.png"),
            SpriteId::Debris(2) => centered("sprites/debris_med.png"),
            SpriteId::Debris(_) => centered("sprites/debris_small.png"),
            SpriteId::Fish(n) => smooth(match n {
                1 => "sprites/fish1.png",
                2 => "sprites/fish2.png",
                3 => "sprites/fish3.png",
                4 => "sprites/fish4.png",
                5 => "sprites/fish5.png",
                6 => "sprites/fish6.png",
                7 => "sprites/fish7.png",
                _ => "sprites/fish8.png",
            }),
        }
    }
}

/// Every image the game can ask for, so the renderer can load them all up front.
pub fn all() -> Vec<SpriteId> {
    let mut ids = vec![
        SpriteId::Loading,
        SpriteId::Title,
        SpriteId::Credits,
        SpriteId::AButton,
        SpriteId::FactorySprite,
        SpriteId::Needle,
        SpriteId::FighterPreview,
        SpriteId::BomberPreview,
        SpriteId::FrigatePreview,
        SpriteId::UpgradePreview,
        SpriteId::HealthFull,
        SpriteId::HealthSome,
        SpriteId::HealthNone,
        SpriteId::Laser,
        SpriteId::Bomb,
        SpriteId::Missile,
        SpriteId::Bubble,
        SpriteId::BigBubble,
        SpriteId::Explosion,
        SpriteId::Spark,
    ];

    for player in 1..=4 {
        ids.push(SpriteId::Fighter(player));
        ids.push(SpriteId::Bomber(player));
        ids.push(SpriteId::Frigate(player));
        ids.push(SpriteId::Factory(player));
    }
    for n in 1..=3 {
        ids.push(SpriteId::FactoryLightDamage(n));
        ids.push(SpriteId::FactoryHeavyDamage(n));
        ids.push(SpriteId::ProductionLayer(n));
        ids.push(SpriteId::Debris(n));
    }
    for n in 1..=5 {
        ids.push(SpriteId::Number(n));
    }
    for n in 1..=8 {
        ids.push(SpriteId::Fish(n));
    }

    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_player_sprites_are_distinct() {
        assert_ne!(SpriteId::Fighter(1), SpriteId::Fighter(2));
        assert_ne!(SpriteId::Fighter(1), SpriteId::Bomber(1));
    }

    #[test]
    fn every_sprite_names_a_file_that_exists() {
        for id in all() {
            let path = id.meta().path;
            assert!(
                std::path::Path::new(path).exists(),
                "{id:?} points at {path}, which is not there"
            );
        }
    }

    #[test]
    fn distinct_sprites_map_to_distinct_files() {
        // a typo in the path table would silently draw the wrong art, and
        // per-player colours are how you tell whose ship is whose
        let mut seen = std::collections::HashMap::new();
        for id in all() {
            if id == SpriteId::FactorySprite {
                continue; // deliberately shares player 1's factory art
            }
            if let Some(other) = seen.insert(id.meta().path, id) {
                panic!("{id:?} and {other:?} both use {}", id.meta().path);
            }
        }
    }
}
