//! `components/log.lua`: the end-of-match statistics.
//!
//! Not gameplay -- nothing reads these back -- but `print_stats` runs on every
//! restart and on exit, and the headless runs print it, so the numbers are part
//! of the observable output.
//!
//! The Lua iterates its tables with `pairs` when printing, and Lua 5.4
//! randomises its string hash seed per process, so the original's output line
//! order varies run to run. This prints in a fixed order instead. That is a
//! deliberate difference: matching an unordered output is not possible, and the
//! numbers are what matter.

/// The three ship classes that shoot, which is what the log counts by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShipClass {
    Fighter,
    Bomber,
    Frigate,
}

impl ShipClass {
    pub const ALL: [ShipClass; 3] = [ShipClass::Fighter, ShipClass::Bomber, ShipClass::Frigate];

    /// `identify_bullet`: which class fired this projectile.
    pub fn of_bullet(blueprint: &str) -> Option<ShipClass> {
        match blueprint {
            "laser" => Some(ShipClass::Fighter),
            "bomb" => Some(ShipClass::Bomber),
            "missile" => Some(ShipClass::Frigate),
            _ => None,
        }
    }

    pub fn of_ship(blueprint: &str) -> Option<ShipClass> {
        match blueprint {
            "fighter" => Some(ShipClass::Fighter),
            "bomber" => Some(ShipClass::Bomber),
            "frigate" => Some(ShipClass::Frigate),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ShipClass::Fighter => "FIGHTER",
            ShipClass::Bomber => "BOMBER",
            ShipClass::Frigate => "FRIGATE",
        }
    }
}

/// A count per ship class.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ByClass {
    pub fighter: f64,
    pub bomber: f64,
    pub frigate: f64,
}

impl ByClass {
    pub fn get(&self, class: ShipClass) -> f64 {
        match class {
            ShipClass::Fighter => self.fighter,
            ShipClass::Bomber => self.bomber,
            ShipClass::Frigate => self.frigate,
        }
    }

    fn add(&mut self, class: ShipClass, amount: f64) {
        match class {
            ShipClass::Fighter => self.fighter += amount,
            ShipClass::Bomber => self.bomber += amount,
            ShipClass::Frigate => self.frigate += amount,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Log {
    pub spawn: ByClass,
    pub death: ByClass,
    pub damage_given: ByClass,
    /// Damage taken by each class, broken down by the class that dealt it.
    pub fighter_damaged_by: ByClass,
    pub bomber_damaged_by: ByClass,
    pub frigate_damaged_by: ByClass,
    pub factory_damaged_by: ByClass,
    pub accuracy_hits: ByClass,
    pub accuracy_misses: ByClass,
    pub frame_count: u64,
}

impl Log {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_spawn(&mut self, blueprint: &str) {
        if let Some(class) = ShipClass::of_ship(blueprint) {
            self.spawn.add(class, 1.0);
        }
    }

    pub fn record_death(&mut self, blueprint: &str) {
        if let Some(class) = ShipClass::of_ship(blueprint) {
            self.death.add(class, 1.0);
        }
    }

    /// `record_hit`. `real_damage` is capped at the target's remaining health,
    /// so overkill is not counted, which means this must be called *before* the
    /// damage lands.
    pub fn record_hit(
        &mut self,
        receiver_blueprint: &str,
        bullet_blueprint: &str,
        damage: f64,
        receiver_hit_points: f64,
    ) {
        let Some(giver) = ShipClass::of_bullet(bullet_blueprint) else {
            return;
        };
        let real_damage = damage.min(receiver_hit_points);

        self.damage_given.add(giver, real_damage);
        self.accuracy_hits.add(giver, 1.0);

        let received = match receiver_blueprint {
            "fighter" => Some(&mut self.fighter_damaged_by),
            "bomber" => Some(&mut self.bomber_damaged_by),
            "frigate" => Some(&mut self.frigate_damaged_by),
            "factory" => Some(&mut self.factory_damaged_by),
            _ => None,
        };
        if let Some(received) = received {
            received.add(giver, real_damage);
        }
    }

    pub fn record_miss(&mut self, bullet_blueprint: &str) {
        if let Some(shooter) = ShipClass::of_bullet(bullet_blueprint) {
            self.accuracy_misses.add(shooter, 1.0);
        }
    }

    pub fn record_time(&mut self) {
        self.frame_count += 1;
    }

    pub fn print_stats(&self) {
        println!("=====================================");
        println!("==         GAME STATISTICS         ==");
        println!("=====================================");
        println!("Time Elapsed:");
        println!("\t{:.3} seconds", self.frame_count as f64 / 60.0);

        let section = |label: &str, counts: &ByClass| {
            println!("{label}");
            for class in ShipClass::ALL {
                println!("\t{}\t{}", class.name(), counts.get(class));
            }
        };
        section("Constructed:", &self.spawn);
        section("Destroyed:", &self.death);
        section("Damage Given:", &self.damage_given);

        println!("Damage Received:");
        for (label, counts) in [
            ("Fighter", &self.fighter_damaged_by),
            ("Bomber", &self.bomber_damaged_by),
            ("Frigate", &self.frigate_damaged_by),
            ("Factory", &self.factory_damaged_by),
        ] {
            println!("\t{label} damaged by");
            for class in ShipClass::ALL {
                println!("\t\t{}\t{}", class.name(), counts.get(class));
            }
        }

        println!("Accuracy:");
        for class in ShipClass::ALL {
            let hits = self.accuracy_hits.get(class);
            let accuracy = hits / (hits + self.accuracy_misses.get(class)) * 100.0;
            println!("    \t{}\t{accuracy:.2}%", class.name());
        }
        println!("=====================================");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bullets_map_to_the_class_that_fired_them() {
        assert_eq!(ShipClass::of_bullet("laser"), Some(ShipClass::Fighter));
        assert_eq!(ShipClass::of_bullet("bomb"), Some(ShipClass::Bomber));
        assert_eq!(ShipClass::of_bullet("missile"), Some(ShipClass::Frigate));
        assert_eq!(ShipClass::of_bullet("bubble"), None);
    }

    #[test]
    fn overkill_is_not_counted_as_damage_given() {
        let mut log = Log::new();
        log.record_hit("fighter", "bomb", 200.0, 40.0);
        assert_eq!(log.damage_given.bomber, 40.0);
        assert_eq!(log.fighter_damaged_by.bomber, 40.0);
    }

    #[test]
    fn hits_and_misses_accumulate_per_class() {
        let mut log = Log::new();
        log.record_hit("frigate", "laser", 10.0, 1000.0);
        log.record_miss("laser");
        log.record_miss("laser");
        assert_eq!(log.accuracy_hits.fighter, 1.0);
        assert_eq!(log.accuracy_misses.fighter, 2.0);
    }

    #[test]
    fn factories_are_not_counted_as_constructed_ships() {
        // the Lua's spawn table only has fighter/bomber/frigate keys
        let mut log = Log::new();
        log.record_spawn("factory");
        assert_eq!(log.spawn, ByClass::default());
    }
}
