//! Texts of the data tables (data.rs, events.rs), keyed by identifier.
//!
//! Descriptions quoting tuning values are templates: `{}` takes the next value of the table entry's
//! `data::Desc`, `{n}` the n-th one (see `Desc::render`).

use super::find;

/// Text table: (identifier, text), in any order.
pub type Table<T> = &'static [(&'static str, T)];

/// Name and description template of a talent, a perk, a synergy…
pub struct Named {
    pub name: &'static str,
    pub desc: &'static str,
}

pub struct UnitText {
    pub name: &'static str,
    pub plural: &'static str,
    /// Name of the talent that unlocks it ("u_<id>").
    pub unlock_name: &'static str,
    /// Name of the talent that boosts it ("m_<id>").
    pub mult_name: &'static str,
}

pub struct ChallengeText {
    pub name: &'static str,
    /// `{0}` is the target level, `{1}` the time limit in minutes for a timed challenge.
    pub desc: &'static str,
    /// Effect of each rank.
    pub reward: &'static str,
}

pub struct EnemyText {
    pub name: &'static str,
    /// Announced effect (`{}`: its current strength, if it has one).
    pub desc: &'static str,
    /// Headline while it is there.
    pub news: &'static str,
}

pub struct DataTexts {
    pub units: Table<UnitText>,
    /// Description of an unlock talent: unit name.
    pub unit_unlock: fn(&str) -> String,
    /// Description of an unlock talent of a challenge unit: unit name, challenge name.
    pub unit_unlock_after: fn(&str, &str) -> String,
    /// Description of a unit boost talent: unit name in the plural, multiplier per rank.
    pub unit_mult: fn(&str, &str) -> String,
    /// Run talents (except the generated unit talents).
    pub talents: Table<Named>,
    /// Talent branches.
    pub branches: Table<&'static str>,
    pub ptalents: Table<Named>,
    pub stalents: Table<Named>,
    pub challenges: Table<ChallengeText>,
    pub skins: Table<&'static str>,
    pub mutations: Table<Named>,
    pub synergies: Table<Named>,
    /// (name, condition).
    pub achievements: Table<Named>,
    pub enemies: Table<EnemyText>,
    /// Chest bonuses (events::CHEST_REWARDS).
    pub chests: Table<&'static str>,
    /// (minimum level, headline): the news banner picks among the unlocked ones.
    pub news: &'static [(u32, &'static str)],
    /// Headlines added after a prestige.
    pub news_prestige: &'static [&'static str],
    /// Headlines added during the Nanarland challenge.
    pub news_nanar: &'static [&'static str],
    /// Headline of the Crocodile Fury challenge.
    pub news_crocodile: &'static str,
    /// Headline while an inspector comes.
    pub news_inspector: &'static str,
}

impl DataTexts {
    pub fn unit(&self, id: &str) -> &UnitText {
        find(self.units, id, "unit")
    }

    pub fn talent(&self, id: &str) -> &Named {
        find(self.talents, id, "talent")
    }

    pub fn branch(&self, id: &str) -> &'static str {
        find(self.branches, id, "branch")
    }

    /// Prestige (p_…) or supernova (s_…) talent.
    pub fn perk(&self, id: &str) -> &Named {
        match self.ptalents.iter().find(|(k, _)| *k == id) {
            Some((_, t)) => t,
            None => find(self.stalents, id, "perk"),
        }
    }

    pub fn challenge(&self, id: &str) -> &ChallengeText {
        find(self.challenges, id, "challenge")
    }

    pub fn skin(&self, id: &str) -> &'static str {
        find(self.skins, id, "skin")
    }

    pub fn mutation(&self, id: &str) -> &Named {
        find(self.mutations, id, "mutation")
    }

    pub fn synergy(&self, id: &str) -> &Named {
        find(self.synergies, id, "synergy")
    }

    pub fn achievement(&self, id: &str) -> &Named {
        find(self.achievements, id, "achievement")
    }

    pub fn chest(&self, id: &str) -> &'static str {
        find(self.chests, id, "chest bonus")
    }

    pub fn enemy(&self, id: &str) -> &EnemyText {
        find(self.enemies, id, "enemy")
    }
}
