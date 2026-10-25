//! Headless simulation to check the game balance.
//!
//! Usage: cargo run --release --bin sim -- [number_of_runs] [presses_per_second]

use champignon::data;
use champignon::events;
use champignon::game::{Game, fmt};
use champignon::rng::{Rng, StdRandom};

const DT: f64 = 0.1;
const MAX_RUN_SECONDS: f64 = 3600.0;
const LEVEL_MARKS: [u32; 8] = [10, 25, 50, 100, 1_000, 10_000, 100_000, 1_000_000];
const PRESTIGE_ORDER: [&str; 10] = [
    "p_cap",
    "p_keep",
    "p_prod",
    "p_click",
    "p_pt",
    "p_vets",
    "p_iso",
    "p_drop",
    "p_offline",
    "p_dna",
];

fn buy_units(g: &mut Game) {
    for _ in 0..100 {
        // The best production per radiation spent (the first of them on a tie).
        let mut best: Option<(&str, f64)> = None;
        for u in data::UNITS.iter().filter(|u| g.can_buy_unit(u.id)) {
            let value = g.unit_prod(u.id) / g.unit_cost(u.id, None);
            if best.is_none_or(|(_, b)| value > b) {
                best = Some((u.id, value));
            }
        }
        let Some((uid, _)) = best else { return };
        g.buy_unit(uid);
    }
}

/// Starts the critical mass and presses at `miss` from the center: 40% in the green, 40% in the yellow, 20% missed.
fn play_qte(g: &mut Game, rng: &mut StdRandom) {
    events::crit_press(g);
    let Some(qte) = g.qte.as_mut() else { return };
    let r = rng.random();
    let miss = if r < 0.4 {
        0.0
    } else if r < 0.8 {
        0.1
    } else {
        0.4
    };
    let pos = if qte.center >= miss {
        qte.center - miss
    } else {
        qte.center + miss
    };
    qte.t = pos / qte.speed;
    events::crit_press(g);
}

struct Bot {
    rng: StdRandom,
    cps: f64,
    presses: f64,
}

impl Bot {
    fn play(&mut self, g: &mut Game) {
        if !g.exploded {
            g.click(true);
        }
        self.presses += DT * self.cps;
        while self.presses >= 1.0 {
            self.presses -= 1.0;
            g.click(true);
        }
        g.auto_talents();
        buy_units(g);
        if g.pending_mutation.is_some() {
            events::choose_mutation(g, 0);
        }
        if g.pending_chest.is_some() {
            events::choose_chest(g, 0);
        }
        if g.crit_gauge >= 1.0 && g.qte.is_none() {
            play_qte(g, &mut self.rng);
        }
        if let Some(plane) = g.plane
            && self.rng.random() < 0.1
        {
            events::catch(g, plane.letter);
        }
        if g.inspector.is_some() || !g.enemies.is_empty() {
            events::chase(g);
        }
    }

    /// Plays a run until the max level or MAX_RUN_SECONDS; returns (level mark, run time) in order.
    fn run_once(&mut self, g: &mut Game) -> Vec<(u32, f64)> {
        let mut marks: Vec<(u32, f64)> = Vec::new();
        while g.run_time < MAX_RUN_SECONDS && g.level < g.cap() {
            self.play(g);
            g.tick(DT);
            for m in LEVEL_MARKS {
                if g.level >= m && !marks.iter().any(|&(k, _)| k == m) {
                    marks.push((m, g.run_time));
                }
            }
        }
        marks
    }
}

/// Buys prestige talents, core expansion first. Returns the ids bought.
fn spend_isotopes(g: &mut Game) -> Vec<&'static str> {
    let mut bought = Vec::new();
    if g.buy_ptalent("p_cap") {
        bought.push("p_cap");
    }
    // Keep enough to aim for the next core expansion.
    let reserve = if g.pr("p_cap") < 4 {
        g.ptalent_cost("p_cap") * 0.6
    } else {
        0.0
    };
    for _ in 0..40 {
        for pid in &PRESTIGE_ORDER[1..] {
            if g.isotopes as f64 - g.ptalent_cost(pid) >= reserve && g.buy_ptalent(pid) {
                bought.push(pid);
            }
        }
    }
    bought
}

/// Supernova talents like a Python dict: {'s_prod': 2, 's_iso': 1}.
fn stalents_text(g: &Game) -> String {
    let items: Vec<String> = data::STALENTS
        .iter()
        .filter(|s| g.sr(s.id) > 0)
        .map(|s| format!("'{}': {}", s.id, g.sr(s.id)))
        .collect();
    format!("{{{}}}", items.join(", "))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let runs: u32 = args
        .get(1)
        .map_or(8, |a| a.parse().expect("number of runs"));
    let cps: f64 = args
        .get(2)
        .map_or(6.0, |a| a.parse().expect("presses per second"));
    let mut bot = Bot {
        rng: StdRandom::seeded(1),
        cps,
        presses: 0.0,
    };
    let mut g = Game::new(Box::new(StdRandom::seeded(2)));
    for r in 0..runs {
        let marks = bot.run_once(&mut g);
        let txt: Vec<String> = marks
            .iter()
            .map(|&(m, t)| format!("L{}:{:.1}m", fmt(m as f64), t / 60.0))
            .collect();
        println!(
            "run {:2} cap {:>5}  level {:>5}  {}",
            r + 1,
            fmt(g.cap() as f64),
            fmt(g.level as f64),
            txt.join("  ")
        );
        println!(
            "        prod {}/s  detonation {}  PT left {}  units {}  mutations {}  achievements {}",
            fmt(g.production()),
            fmt(g.click_value(true)),
            g.pt,
            g.units.total(),
            g.mutations.len(),
            g.achievements.len()
        );
        if !g.can_prestige() {
            println!("  -> stuck, simulation over");
            break;
        }
        if g.can_supernova() {
            let stars = g.supernova_gain();
            g.supernova();
            while data::STALENTS.iter().any(|s| g.buy_stalent(s.id)) {}
            println!(
                "        SUPERNOVA #{} +{} ✦ -> talents {}  left {} ✦",
                g.supernova_count,
                stars,
                stalents_text(&g),
                g.stars
            );
            continue;
        }
        let gain = g.prestige_gain();
        g.prestige(None);
        let mut bought = spend_isotopes(&mut g);
        bought.sort();
        bought.dedup();
        println!(
            "        prestige +{gain} ⚛ -> bought {}  left {}",
            bought.join(", "),
            g.isotopes
        );
    }
}
