//! Headless simulation to check the game balance.
//!
//! Usage: cargo run --release --bin sim -- [number_of_runs] [presses_per_second] [defis|unites|doubles]
//!
//! With `defis`, the ranks of each challenge are then tried in turn, from the state reached after the runs.
//! With `unites`, the bot plays the challenges that unlock a unit (rank I) during the runs, as soon as they
//! are available, to measure what these units change.
//! With `doubles`, every challenge is set to rank I, then every allowed pair is tried as a double challenge
//! (rank II of both), each from the same state; only the failed and slow pairs are listed.

use champignon::data;
use champignon::events;
use champignon::game::{Game, fmt, wall_time};
use champignon::rng::{Rng, StdRandom};

const DT: f64 = 0.1;
const MAX_RUN_SECONDS: f64 = 3600.0;
const DEFAULT_RUNS: u32 = 8;
const DEFAULT_PRESSES_PER_SECOND: f64 = 6.0;
/// Fixed seeds: the bot's choices and the game's draws, so that two simulations print the same thing.
const BOT_SEED: u64 = 1;
const GAME_SEED: u64 = 2;
/// Longest wait (in seconds of production) to save up for the best unit rather than buy a worse one.
const SAVE_SECONDS: f64 = 30.0;
/// Share of the best isotopes-per-second rate under which the bot stops farming the surplus bonus.
const SURPLUS_STOP: f64 = 0.98;
/// Units bought at most per tick (the bot stops earlier when it can no longer afford any).
const MAX_UNIT_BUYS: usize = 100;
/// Critical mass presses: chance (cumulative) of landing in the green, then in the yellow, else missing;
/// distance from the center of the green zone in each case.
const QTE_GREEN_CHANCE: f64 = 0.4;
const QTE_YELLOW_CHANCE: f64 = 0.8;
const QTE_GREEN_MISS: f64 = 0.0;
const QTE_YELLOW_MISS: f64 = 0.1;
const QTE_MISSED_MISS: f64 = 0.4;
/// Chance per tick of typing the letter of the plane flying by.
const CATCH_CHANCE: f64 = 0.1;
/// Under this core expansion rank, part of the isotopes is kept aside for the next one…
const CAP_RANK_SAVED_FOR: u32 = 4;
/// …this share of its price.
const CAP_RESERVE_SHARE: f64 = 0.6;
/// After a run shorter than this, two core expansion ranks may be bought at once.
const QUICK_RUN_SECONDS: f64 = 120.0;
/// Passes over PRESTIGE_ORDER when spending isotopes (one rank per talent per pass).
const PRESTIGE_BUY_PASSES: usize = 40;
const SECONDS_PER_MINUTE: f64 = 60.0;
/// Mode "doubles": won pairs slower than this are listed.
const SLOW_DOUBLE_SECONDS: f64 = 1800.0;
/// Runs shorter than this are counted as "flash" runs in the summary.
const FLASH_RUN_SECONDS: f64 = 60.0;
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

/// The unit with the best production per radiation spent (the first of them on a tie), among those
/// that pass `keep`.
fn best_unit(g: &Game, keep: impl Fn(&str) -> bool) -> Option<&'static str> {
    let mut best: Option<(&'static str, f64)> = None;
    for u in data::UNITS.iter().filter(|u| keep(u.id)) {
        let value = g.unit_prod(u.id) / g.unit_cost(u.id, None);
        if best.is_none_or(|(_, b)| value > b) {
            best = Some((u.id, value));
        }
    }
    best.map(|(uid, _)| uid)
}

/// Buys the best unit; saves up for it when it can be afforded within SAVE_SECONDS of production, otherwise
/// buys the best affordable one.
fn buy_units(g: &mut Game) {
    for _ in 0..MAX_UNIT_BUYS {
        let Some(target) = best_unit(g, |uid| g.unit_allowed(uid)) else {
            return;
        };
        let missing = g.unit_cost(target, None) - g.radiation;
        let uid = if missing <= 0.0 {
            target
        } else if missing <= g.production() * SAVE_SECONDS {
            return;
        } else {
            let Some(uid) = best_unit(g, |uid| g.can_buy_unit(uid)) else {
                return;
            };
            uid
        };
        g.buy_unit(uid);
    }
}

/// Starts the critical mass and presses at `miss` from the center: in the green, in the yellow or missed
/// (QTE_* chances).
fn play_qte(g: &mut Game, rng: &mut StdRandom) {
    events::crit_press(g);
    let Some(qte) = g.qte.as_mut() else { return };
    let r = rng.random();
    let miss = if r < QTE_GREEN_CHANCE {
        QTE_GREEN_MISS
    } else if r < QTE_YELLOW_CHANCE {
        QTE_YELLOW_MISS
    } else {
        QTE_MISSED_MISS
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
    fn new(cps: f64) -> Bot {
        Bot {
            rng: StdRandom::seeded(BOT_SEED),
            cps,
            presses: 0.0,
        }
    }

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
            && self.rng.random() < CATCH_CHANCE
        {
            events::catch(g, plane.letter);
        }
        if g.inspector.is_some() || !g.enemies.is_empty() {
            events::chase(g);
        }
    }

    /// Plays a double challenge until it ends or MAX_RUN_SECONDS; None if it cannot start, else whether it
    /// was won.
    fn run_double(&mut self, g: &mut Game, a: &'static str, b: &'static str) -> Option<bool> {
        let before = g.stats.doubles;
        if !g.prestige_double(a, b) {
            return None;
        }
        while g.run_time < MAX_RUN_SECONDS && g.in_challenge(a) {
            self.play(g);
            g.tick(DT);
        }
        Some(g.stats.doubles > before)
    }

    /// Plays the next rank of a challenge until it ends (success or failure) or MAX_RUN_SECONDS; None if it
    /// cannot start, else whether the rank was completed.
    fn run_challenge(&mut self, g: &mut Game, id: &'static str) -> Option<bool> {
        let before = g.challenge_rank(id);
        if !g.prestige(Some(id)) {
            return None;
        }
        while g.run_time < MAX_RUN_SECONDS && g.in_challenge(id) {
            self.play(g);
            g.tick(DT);
        }
        Some(g.challenge_rank(id) > before)
    }

    /// Plays a run until the max level or MAX_RUN_SECONDS, then farms the surplus bonus; returns (level mark, run time) in order.
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
        self.farm_surplus(g);
        marks
    }

    /// At the max level, keeps playing for the surplus bonus as long as the isotopes per second do not drop
    /// below SURPLUS_STOP of the best rate seen.
    fn farm_surplus(&mut self, g: &mut Game) {
        let rate = |g: &Game| g.prestige_gain() as f64 / g.run_time.max(DT);
        let mut best = rate(g);
        while g.can_prestige() && g.run_time < MAX_RUN_SECONDS && rate(g) >= best * SURPLUS_STOP {
            self.play(g);
            g.tick(DT);
            best = best.max(rate(g));
        }
    }
}

/// Buys prestige talents, core expansion first (two ranks after a quick run). Returns the ids bought.
fn spend_isotopes(g: &mut Game, last_run_time: f64) -> Vec<&'static str> {
    let mut bought = Vec::new();
    if g.buy_ptalent("p_cap") {
        bought.push("p_cap");
        // A second rank right away only when the max level was reached quickly.
        if last_run_time < QUICK_RUN_SECONDS && g.buy_ptalent("p_cap") {
            bought.push("p_cap");
        }
    }
    // Keep enough to aim for the next core expansion.
    let reserve = if g.pr("p_cap") < CAP_RANK_SAVED_FOR {
        g.ptalent_cost("p_cap") * CAP_RESERVE_SHARE
    } else {
        0.0
    };
    for _ in 0..PRESTIGE_BUY_PASSES {
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
        .map_or(DEFAULT_RUNS, |a| a.parse().expect("number of runs"));
    let cps: f64 = args.get(2).map_or(DEFAULT_PRESSES_PER_SECOND, |a| {
        a.parse().expect("presses per second")
    });
    let mode = args.get(3).map(String::as_str);
    let mut bot = Bot::new(cps);
    let mut g = Game::new(Box::new(StdRandom::seeded(GAME_SEED)));
    // Unit challenges already tried in this supernova cycle (mode "unites").
    let mut tried: Vec<&'static str> = Vec::new();
    let mut total = 0.0;
    let mut durations: Vec<f64> = Vec::new();
    let mut supernova_times: Vec<f64> = Vec::new();
    for r in 0..runs {
        let marks = bot.run_once(&mut g);
        total += g.run_time;
        durations.push(g.run_time);
        let txt: Vec<String> = marks
            .iter()
            .map(|&(m, t)| format!("L{}:{:.1}m", fmt(m as f64), t / SECONDS_PER_MINUTE))
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
            supernova_times.push(total);
            tried.clear();
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
        let run_time = g.run_time;
        g.prestige(None);
        let mut bought = spend_isotopes(&mut g, run_time);
        bought.sort();
        bought.dedup();
        println!(
            "        prestige +{gain} ⚛ -> bought {}  left {}",
            bought.join(", "),
            g.isotopes
        );
        if mode == Some("unites") {
            total += play_unit_challenge(&mut bot, &mut g, &mut tried);
        }
    }
    print_summary(total, &durations, &supernova_times);
    if mode == Some("defis") {
        try_challenges(cps, &g);
    }
    if mode == Some("doubles") {
        try_doubles(cps, &g);
    }
}

/// Mode "doubles": every allowed pair at rank II, from a copy of `start` where every challenge is at rank I.
/// Lists the pairs that fail, cannot start or take more than SLOW_DOUBLE_SECONDS, then a summary.
fn try_doubles(cps: f64, start: &Game) {
    let mut base = start.save_data();
    base.challenge_ranks = Some(
        data::CHALLENGES
            .iter()
            .map(|c| (c.id.to_string(), 1))
            .collect(),
    );
    let (mut won, mut slowest) = (0, (0.0, String::new()));
    let mut tried = 0;
    for (i, a) in data::CHALLENGES.iter().enumerate() {
        for b in &data::CHALLENGES[i + 1..] {
            if data::double_forbidden(a.id, b.id) {
                continue;
            }
            tried += 1;
            let mut g = Game::from_save(base.clone(), wall_time());
            g.rng = Box::new(StdRandom::seeded(GAME_SEED));
            let pair = format!("{} + {}", a.name, b.name);
            match Bot::new(cps).run_double(&mut g, a.id, b.id) {
                None => println!("double {pair}: cannot start"),
                Some(false) => println!(
                    "double {pair}: failed at level {} after {:.1}m",
                    fmt(g.level as f64),
                    g.run_time / SECONDS_PER_MINUTE
                ),
                Some(true) => {
                    won += 1;
                    if g.run_time > SLOW_DOUBLE_SECONDS {
                        println!(
                            "double {pair}: slow, {:.1}m",
                            g.run_time / SECONDS_PER_MINUTE
                        );
                    }
                    if g.run_time > slowest.0 {
                        slowest = (g.run_time, pair);
                    }
                }
            }
        }
    }
    println!(
        "doubles: {won}/{tried} won, slowest {:.1}m ({})",
        slowest.0 / SECONDS_PER_MINUTE,
        slowest.1
    );
}

/// Mode "unites": starts the next run with the first challenge that unlocks a unit, not done yet, available
/// and not tried in this supernova cycle; the run then goes on normally once it ends. A failed attempt (which
/// lasts MAX_RUN_SECONDS and would leave the run stuck below the max level) is dropped: the game goes back to its
/// state before it. Returns the time lost to a failed attempt.
fn play_unit_challenge(bot: &mut Bot, g: &mut Game, tried: &mut Vec<&'static str>) -> f64 {
    let next = data::UNITS
        .iter()
        .filter_map(|u| u.challenge)
        .map(|(c, _)| c)
        .find(|&c| g.challenge_rank(c) == 0 && g.challenge_available(c) && !tried.contains(&c));
    let Some(c) = next else { return 0.0 };
    tried.push(c);
    let mut before = Game::from_save(g.save_data(), wall_time());
    let done = bot.run_challenge(g, c) == Some(true);
    let name = data::challenge(c).name;
    let result = if done { "done" } else { "failed" };
    println!(
        "        challenge {name} I {result} after {:.1}m",
        g.run_time / SECONDS_PER_MINUTE
    );
    if done {
        return 0.0;
    }
    let lost = g.run_time;
    std::mem::swap(&mut before.rng, &mut g.rng);
    *g = before;
    lost
}

/// Tries each challenge once, each from a copy of the same state with a fresh bot (the order does not matter).
fn try_challenges(cps: f64, start: &Game) {
    for c in &data::CHALLENGES {
        let mut g = Game::from_save(start.save_data(), wall_time());
        g.rng = Box::new(StdRandom::seeded(GAME_SEED));
        let mut bot = Bot::new(cps);
        let mut results = Vec::new();
        // Its ranks one after the other, as long as they succeed.
        while g.challenge_rank(c.id) < data::CHALLENGE_RANKS {
            let rank = data::RANK_NAMES[g.challenge_rank(c.id) as usize];
            let target = g.challenge_target(c.id);
            match bot.run_challenge(&mut g, c.id) {
                None => {
                    results.push(format!("{rank}: needs max level {}", fmt(target as f64)));
                    break;
                }
                Some(true) => {
                    results.push(format!("{rank}: {:.1}m", g.run_time / SECONDS_PER_MINUTE));
                }
                Some(false) => {
                    results.push(format!(
                        "{rank}: failed at level {} after {:.1}m",
                        fmt(g.level as f64),
                        g.run_time / SECONDS_PER_MINUTE
                    ));
                    break;
                }
            }
        }
        println!("challenge {:<22} {}", c.name, results.join("  "));
    }
}

fn minutes(seconds: f64) -> String {
    format!("{:.0} min", seconds / SECONDS_PER_MINUTE)
}

/// Total time, flash runs (under FLASH_RUN_SECONDS), shortest run and time of each supernova.
fn print_summary(total: f64, durations: &[f64], supernova_times: &[f64]) {
    let flash = durations.iter().filter(|&&d| d < FLASH_RUN_SECONDS).count();
    let shortest = durations.iter().copied().fold(f64::INFINITY, f64::min);
    let supernovas: Vec<String> = supernova_times.iter().map(|&t| minutes(t)).collect();
    println!(
        "total {}  runs {}  under {:.0} s: {}  shortest {:.0} s  supernovas at [{}]",
        minutes(total),
        durations.len(),
        FLASH_RUN_SECONDS,
        flash,
        shortest,
        supernovas.join(", ")
    );
}
