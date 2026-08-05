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
    Background,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_player_sprites_are_distinct() {
        assert_ne!(SpriteId::Fighter(1), SpriteId::Fighter(2));
        assert_ne!(SpriteId::Fighter(1), SpriteId::Bomber(1));
    }
}
