//! The ammo bookkeeping shared by `scripts/fighter_shooting.lua` and
//! `scripts/frigate_shooting.lua`.
//!
//! The two Lua files are near-duplicates that differ in four constants and in
//! one predicate: a fighter shoots whenever it has a round left, a frigate only
//! when its magazine is completely full, which is why frigates fire in volleys.
//! That difference lives in the callers; the counting lives here.

/// A magazine that reloads continuously and a per-shot cooldown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weapon {
    /// Fractional: it reloads by a fraction of a round each frame.
    pub shots: f64,
    pub cooldown: f64,
    capacity: f64,
    reload_rate: f64,
    cooldown_time: f64,
}

impl Weapon {
    pub fn new(shots: f64, capacity: f64, reload_rate: f64, cooldown_time: f64) -> Self {
        Self {
            shots,
            cooldown: 0.0,
            capacity,
            reload_rate,
            cooldown_time,
        }
    }

    /// A fighter's: five rounds, starting full, reloading in 30 seconds.
    pub fn fighter() -> Self {
        Self::new(5.0, 5.0, 2.0 / 60.0, 6.0)
    }

    /// A frigate's: eight rounds, starting **empty**, reloading more slowly.
    pub fn frigate() -> Self {
        Self::new(0.0, 8.0, 1.2 / 60.0, 6.0)
    }

    pub fn is_empty(&self) -> bool {
        self.shots < 1.0
    }

    pub fn is_reloaded(&self) -> bool {
        self.shots == self.capacity
    }

    pub fn is_cooled_down(&self) -> bool {
        self.cooldown == 0.0
    }

    pub fn update(&mut self) {
        self.cooldown = (self.cooldown - 1.0).max(0.0);
        self.shots = (self.shots + self.reload_rate).min(self.capacity);
    }

    /// Spends a round if there is one. Returns whether the shot happened.
    pub fn try_fire(&mut self) -> bool {
        if self.cooldown == 0.0 && self.shots >= 1.0 {
            self.shots -= 1.0;
            self.cooldown = self.cooldown_time;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fighter_starts_loaded_and_a_frigate_starts_empty() {
        assert!(!Weapon::fighter().is_empty());
        assert!(Weapon::frigate().is_empty());
    }

    #[test]
    fn firing_spends_a_round_and_starts_the_cooldown() {
        let mut weapon = Weapon::fighter();
        assert!(weapon.try_fire());
        assert_eq!(weapon.shots, 4.0);
        assert_eq!(weapon.cooldown, 6.0);
    }

    #[test]
    fn a_weapon_on_cooldown_refuses_to_fire() {
        let mut weapon = Weapon::fighter();
        assert!(weapon.try_fire());
        assert!(!weapon.try_fire());

        for _ in 0..6 {
            weapon.update();
        }
        assert!(weapon.is_cooled_down());
        assert!(weapon.try_fire());
    }

    #[test]
    fn an_empty_weapon_refuses_to_fire() {
        let mut weapon = Weapon::frigate();
        assert!(!weapon.try_fire());
    }

    #[test]
    fn reloading_stops_at_capacity() {
        let mut weapon = Weapon::frigate();
        for _ in 0..1000 {
            weapon.update();
        }
        assert_eq!(weapon.shots, 8.0);
        assert!(weapon.is_reloaded());
    }

    #[test]
    fn a_partly_reloaded_magazine_is_not_reloaded() {
        // the frigate's volley predicate hangs on this exact distinction
        let mut weapon = Weapon::frigate();
        for _ in 0..100 {
            weapon.update();
        }
        assert!(!weapon.is_empty());
        assert!(!weapon.is_reloaded());
    }
}
