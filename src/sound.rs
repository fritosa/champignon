//! Sound effects synthesized on the fly, played through PulseAudio (libpulse-simple loaded at runtime).
//!
//! Sounds are generated once at startup, then mixed in a thread that writes small blocks to a
//! PulseAudio stream. Without PulseAudio, everything is silently disabled.

use std::collections::HashMap;
use std::f64::consts::TAU;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread;

use libloading::Library;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

pub const RATE: usize = 22050;
const RATE_F: f64 = RATE as f64;
pub const CHUNK: usize = 441; // 20 ms
pub const MAX_VOICES: usize = 10;
pub const MAX_MINOR: usize = 4; // max short sound effects (presses, ticks...) playing at once
pub const DUCK: f64 = 0.5; // sound effect volume while a jingle plays
/// Sound priorities. From JINGLE up, they are exclusive jingles: one at a time, a more important jingle
/// cuts off the current one, a less (or equally) important one is dropped.
/// The others (1 by default) are sound effects, capped at MAX_MINOR voices.
pub const JINGLE: u8 = 2;
pub const PRIORITY: &[(&str, u8)] = &[
    ("supernova", 6),
    ("prestige", 6),
    ("boom", 5),
    ("perfect", 5),
    ("fanfare", 4),
    ("synergy", 4),
    ("achievement", 4),
    ("mutation", 3),
    ("alarm", 3),
    ("freeze", 3),
    ("chased", 3),
    ("milestone", 3),
    ("chest", 3),
    ("enemy", 3),
    ("talent", 2),
    ("drop", 2),
    ("partial", 2),
    ("fizzle", 2),
    ("level", 2),
    ("buy", 2),
];
pub const MUSIC_VOL: f64 = 0.25;
/// Target peak per effect after normalization (DEFAULT_LEVEL otherwise): frequent effects stay quieter.
pub const LEVEL: &[(&str, f64)] = &[
    ("tick", 0.3),
    ("click", 0.45),
    ("jam", 0.45),
    ("plane", 0.5),
    ("charge", 0.5),
    ("alarm", 0.6),
];
pub const DEFAULT_LEVEL: f64 = 0.8;
pub const MUSIC_PEAK: f64 = 0.9;
pub const MUSIC_FADE_SECONDS: f64 = 1.0;
pub const FADE_PER_CHUNK: f64 = CHUNK as f64 / RATE_F / MUSIC_FADE_SECONDS;

/// Priority of a sound (1 for plain sound effects).
pub fn priority(name: &str) -> u8 {
    PRIORITY
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(1, |&(_, p)| p)
}

/// Target peak of a sound, numbered variants (click0..click7) sharing their family's level.
pub fn level(name: &str) -> f64 {
    let family = name.trim_end_matches(|c: char| c.is_ascii_digit());
    LEVEL
        .iter()
        .find(|(n, _)| *n == family)
        .map_or(DEFAULT_LEVEL, |&(_, l)| l)
}

/// Stable 64-bit seed from a string (FNV-1a), for the RNGs seeded by name.
fn seed_of(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn pick<T: Copy>(rng: &mut StdRng, items: &[T]) -> T {
    items[rng.random_range(0..items.len())]
}

// --- Synthesis -----------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wave {
    Sine,
    Square,
    Tri,
    Saw,
}
use Wave::{Saw, Sine, Square, Tri};

pub fn osc(kind: Wave, phase: f64) -> f64 {
    let p = phase.rem_euclid(1.0);
    match kind {
        Sine => (TAU * p).sin(),
        Square => {
            if p < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Tri => 4.0 * (p - 0.5).abs() - 1.0,
        Saw => 2.0 * p - 1.0,
    }
}

/// Envelope of a tone: attack (s), exponential decay time constant (linear fade to the end if None),
/// vibrato (frequency in Hz, relative depth).
#[derive(Clone, Copy, Debug)]
pub struct Env {
    pub attack: f64,
    pub decay: Option<f64>,
    pub vibrato: (f64, f64),
}

pub const PLAIN: Env = Env {
    attack: 0.005,
    decay: None,
    vibrato: (0.0, 0.0),
};

const fn decay(d: f64) -> Env {
    Env {
        decay: Some(d),
        ..PLAIN
    }
}

const fn attack(a: f64) -> Env {
    Env { attack: a, ..PLAIN }
}

/// Note with an exponential glide f0 → f1, attack + decay envelope.
pub fn tone(f0: f64, f1: f64, dur: f64, kind: Wave, vol: f64, env: Env) -> Vec<f64> {
    let length = (dur * RATE_F) as usize;
    let (vibrato_freq, vibrato_depth) = env.vibrato;
    let mut phase = 0.0;
    (0..length)
        .map(|i| {
            let t = i as f64 / RATE_F;
            let mut freq = f0 * (f1 / f0).powf(i as f64 / length as f64);
            if vibrato_freq != 0.0 {
                freq *= 1.0 + vibrato_depth * (TAU * vibrato_freq * t).sin();
            }
            phase += freq / RATE_F;
            let fade_in = if env.attack > 0.0 {
                (t / env.attack).min(1.0)
            } else {
                1.0
            };
            let fade_out = match env.decay {
                Some(d) => (-t / d).exp(),
                None => 1.0 - i as f64 / length as f64,
            };
            osc(kind, phase) * (fade_in * fade_out) * vol
        })
        .collect()
}

/// Low-pass filtered (one-pole) white noise with exponential decay.
pub fn noise(dur: f64, vol: f64, decay: f64, cutoff: f64, seed: u64) -> Vec<f64> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut filtered = 0.0;
    (0..(dur * RATE_F) as usize)
        .map(|i| {
            filtered += cutoff * (rng.random_range(-1.0..1.0) - filtered);
            filtered * vol * (-(i as f64) / RATE_F / decay).exp()
        })
        .collect()
}

/// Overlays (offset in s, samples) pairs.
pub fn mix(parts: Vec<(f64, Vec<f64>)>) -> Vec<f64> {
    let start = |offset: f64| (offset * RATE_F) as usize;
    let length = parts
        .iter()
        .map(|(offset, s)| start(*offset) + s.len())
        .max()
        .unwrap_or(0);
    let mut out = vec![0.0; length];
    for (offset, samples) in parts {
        for (o, x) in out[start(offset)..].iter_mut().zip(samples) {
            *o += x;
        }
    }
    out
}

/// Plays sounds one after another.
pub fn seq(notes: Vec<Vec<f64>>, gap: f64) -> Vec<f64> {
    let mut t = 0.0;
    let parts = notes
        .into_iter()
        .map(|samples| {
            let part = (t, samples);
            t += part.1.len() as f64 / RATE_F + gap;
            part
        })
        .collect();
    mix(parts)
}

/// Steady notes played one after another, all with the same shape.
fn arpeggio(freqs: &[f64], dur: f64, kind: Wave, vol: f64, env: Env) -> Vec<f64> {
    seq(
        freqs
            .iter()
            .map(|&f| tone(f, f, dur, kind, vol, env))
            .collect(),
        0.0,
    )
}

fn normalize(samples: &[f64], peak: f64) -> Arc<[f32]> {
    let gain = peak / samples.iter().fold(0.0, |m: f64, x| m.max(x.abs()));
    samples.iter().map(|x| (x * gain) as f32).collect()
}

/// Effect bank: name → samples normalized to their LEVEL.
pub type Bank = HashMap<String, Arc<[f32]>>;

pub fn build() -> Bank {
    let mut s: Vec<(String, Vec<f64>)> = Vec::new();
    let mut add = |name: &str, samples: Vec<f64>| s.push((name.to_string(), samples));
    for i in 0..8 {
        let pitch = 220.0 * 2f64.powf(f64::from(i) / 6.0);
        add(
            &format!("click{i}"),
            tone(pitch * 1.5, pitch, 0.035, Square, 0.12, PLAIN),
        );
    }
    add(
        "crit",
        mix(vec![
            (0.0, tone(1400.0, 300.0, 0.15, Saw, 0.25, PLAIN)),
            (0.0, noise(0.12, 0.25, 0.05, 0.6, 1)),
        ]),
    );
    add(
        "level",
        arpeggio(&[523.0, 659.0, 784.0], 0.05, Square, 0.15, PLAIN),
    );
    let boom = mix(vec![
        (0.0, noise(1.6, 0.8, 0.45, 0.06, 2)),
        (0.0, tone(90.0, 30.0, 1.2, Sine, 0.55, decay(0.4))),
        (0.0, noise(0.08, 0.5, 0.03, 0.7, 3)),
    ]);
    add("boom", boom.clone());
    add(
        "overheat",
        mix(vec![
            (0.0, tone(700.0, 120.0, 0.5, Saw, 0.22, PLAIN)),
            (0.0, noise(0.6, 0.15, 0.3, 0.8, 4)),
        ]),
    );
    add("nope", tone(110.0, 100.0, 0.12, Square, 0.15, PLAIN));
    add("jam", tone(140.0, 90.0, 0.08, Square, 0.15, PLAIN)); // press refused during overheat
    add(
        "buy",
        seq(
            vec![
                tone(988.0, 988.0, 0.06, Square, 0.15, PLAIN),
                tone(1319.0, 1319.0, 0.22, Square, 0.15, decay(0.08)),
            ],
            0.0,
        ),
    );
    add(
        "milestone",
        mix(vec![
            (
                0.0,
                arpeggio(&[784.0, 988.0, 1175.0, 1568.0], 0.06, Square, 0.12, PLAIN),
            ),
            (0.24, tone(2093.0, 2093.0, 0.3, Tri, 0.2, decay(0.1))),
        ]),
    );
    add(
        "chest",
        mix(vec![
            (0.0, tone(300.0, 300.0, 0.05, Square, 0.12, PLAIN)),
            (0.06, noise(0.08, 0.2, 0.03, 0.5, 13)),
            (
                0.12,
                arpeggio(&[1047.0, 1319.0, 1568.0, 2093.0], 0.05, Tri, 0.3, PLAIN),
            ),
        ]),
    );
    add(
        "talent",
        arpeggio(&[392.0, 523.0, 784.0], 0.07, Tri, 0.3, PLAIN),
    );
    add("tick", tone(1800.0, 1800.0, 0.008, Sine, 0.06, PLAIN));
    let achievement = seq(
        [(523.0, 0.09), (659.0, 0.09), (784.0, 0.09), (1047.0, 0.45)]
            .iter()
            .map(|&(f, d)| tone(f, f, d, Tri, 0.35, decay(d)))
            .collect(),
        0.0,
    );
    add("achievement", achievement.clone());
    add(
        "plane",
        mix(vec![
            (
                0.0,
                tone(
                    110.0,
                    95.0,
                    1.6,
                    Saw,
                    0.12,
                    Env {
                        vibrato: (7.0, 0.02),
                        ..attack(0.4)
                    },
                ),
            ),
            (0.0, noise(1.6, 0.12, 1.0, 0.05, 5)),
        ]),
    );
    add("drop", tone(400.0, 1600.0, 0.16, Square, 0.15, PLAIN));
    add(
        "alarm",
        arpeggio(
            &[660.0, 880.0, 660.0, 880.0],
            0.22,
            Square,
            0.12,
            attack(0.02),
        ),
    );
    add(
        "hit",
        mix(vec![
            (0.0, tone(160.0, 60.0, 0.1, Sine, 0.6, decay(0.04))),
            (0.0, noise(0.06, 0.4, 0.02, 0.5, 6)),
        ]),
    );
    add(
        "enemy",
        arpeggio(&[392.0, 294.0, 392.0, 294.0], 0.14, Saw, 0.15, attack(0.01)),
    );
    add(
        "chased",
        tone(
            1300.0,
            250.0,
            0.5,
            Sine,
            0.3,
            Env {
                vibrato: (9.0, 0.03),
                ..PLAIN
            },
        ),
    );
    add(
        "freeze",
        arpeggio(
            &[2093.0, 1568.0, 1047.0, 784.0],
            0.12,
            Tri,
            0.25,
            decay(0.1),
        ),
    );
    add("charge", tone(200.0, 900.0, 0.45, Saw, 0.12, attack(0.05)));
    add(
        "perfect",
        mix(vec![(0.0, boom.clone()), (0.15, achievement)]),
    );
    add(
        "partial",
        seq(
            vec![
                tone(659.0, 659.0, 0.08, Tri, 0.3, PLAIN),
                tone(880.0, 880.0, 0.2, Tri, 0.3, decay(0.1)),
            ],
            0.0,
        ),
    );
    add(
        "fizzle",
        mix(vec![
            (0.0, noise(0.5, 0.35, 0.2, 0.3, 7)),
            (0.0, tone(300.0, 80.0, 0.5, Sine, 0.2, PLAIN)),
        ]),
    );
    let mutation = tone(
        300.0,
        600.0,
        0.7,
        Sine,
        0.35,
        Env {
            vibrato: (6.0, 0.15),
            ..attack(0.05)
        },
    );
    add("mutation", mutation.clone());
    add(
        "prestige",
        mix(vec![
            (0.0, tone(100.0, 1600.0, 1.4, Saw, 0.15, attack(0.3))),
            (1.2, boom.clone()),
        ]),
    );
    add(
        "supernova",
        mix(vec![
            (0.0, tone(80.0, 2400.0, 2.2, Saw, 0.15, attack(0.5))),
            (2.0, boom.clone()),
            (2.6, boom),
        ]),
    );
    add(
        "synergy",
        mix(vec![
            (0.0, mutation),
            (
                0.35,
                arpeggio(&[659.0, 831.0, 988.0, 1319.0], 0.08, Tri, 0.3, PLAIN),
            ),
        ]),
    );
    add(
        "fanfare",
        seq(
            [(392.0, 0.1), (392.0, 0.1), (392.0, 0.1), (523.0, 0.5)]
                .iter()
                .map(|&(f, d)| tone(f, f, d, Square, 0.15, PLAIN))
                .collect(),
            0.03,
        ),
    );
    // Bring each sound to its target peak: stacked layers don't clip, faint sounds stay audible.
    s.into_iter()
        .map(|(name, samples)| {
            let normalized = normalize(&samples, level(&name));
            (name, normalized)
        })
        .collect()
}

// --- Music -----------------------------------------------------------------------------

pub const BEAT: f64 = 0.5; // 120 bpm
pub const BEATS_PER_BAR: usize = 4;
pub const BARS: usize = 8;
pub const SWING: f64 = 2.0 / 3.0; // the off-beat eighth lands two thirds into the beat (triplet swing)
const C3: f64 = 130.8;
const D3: f64 = 146.8;
const E2: f64 = 82.4;
const F2: f64 = 87.3;
const G2: f64 = 98.0;
const A2: f64 = 110.0;
const B2: f64 = 123.5;
/// Chord in semitones above the root: sixths and sevenths of 1940s jazz.
pub type Chord = [i32; 4];
const MAJ6: Chord = [0, 4, 7, 9];
const MIN6: Chord = [0, 3, 7, 9];
const MIN7: Chord = [0, 3, 7, 10];
const DOM7: Chord = [0, 4, 7, 10];
/// One (root, chord) per bar, fallout-shelter radio style: crooner, film noir, big band.
pub const PROGRESSIONS: [(&str, [(f64, Chord); BARS]); 3] = [
    (
        "calm",
        [
            (C3, MAJ6),
            (A2, DOM7),
            (D3, MIN7),
            (G2, DOM7),
            (C3, MAJ6),
            (E2, DOM7),
            (F2, MAJ6),
            (G2, DOM7),
        ],
    ),
    (
        "tension",
        [
            (A2, MIN6),
            (A2, MIN6),
            (D3, MIN6),
            (E2, DOM7),
            (A2, MIN6),
            (F2, DOM7),
            (B2, MIN7),
            (E2, DOM7),
        ],
    ),
    (
        "epic",
        [
            (C3, MAJ6),
            (C3, MAJ6),
            (F2, MAJ6),
            (F2, MAJ6),
            (C3, MAJ6),
            (A2, DOM7),
            (D3, MIN7),
            (G2, DOM7),
        ],
    ),
];
/// Theme rhythms (note start in beats, length in beats), picked at random bar by bar.
const PHRASES: &[&[(f64, f64)]] = &[
    &[(0.0, 1.5), (1.5, 0.5), (2.0, 2.0)],
    &[(0.0, SWING), (SWING, 1.0 - SWING), (1.0, 1.0), (3.0, 1.0)],
    &[(1.0, 1.0), (2.0, 1.0), (3.0, 1.0)],
    &[(0.0, 3.0)],
    &[(0.5 + SWING / 2.0, 1.0), (2.0, 0.5), (2.0 + SWING, 1.3)],
];

/// Arrangement of a track, in beats of the bar. Bass: beats played among root, third, fifth, approach;
/// theme: played one bar out of theme_every.
#[derive(Debug)]
pub struct Style {
    pub bass: &'static [usize],
    pub piano: &'static [f64],
    pub kick: &'static [f64],
    pub theme_every: usize,
}

/// The tracks, in the order of `build_music`'s result.
pub const STYLES: [(&str, Style); 3] = [
    // crooner ballad
    (
        "calm",
        Style {
            bass: &[0, 2],
            piano: &[1.0, 3.0],
            kick: &[],
            theme_every: 1,
        },
    ),
    // suspended theme
    (
        "tension",
        Style {
            bass: &[0, 1, 2, 3],
            piano: &[1.0, 3.0],
            kick: &[],
            theme_every: 2,
        },
    ),
    // big band
    (
        "epic",
        Style {
            bass: &[0, 1, 2, 3],
            piano: &[1.0, 3.0, 2.0 + SWING, 3.0 + SWING],
            kick: &[0.0, 2.0],
            theme_every: 1,
        },
    ),
];
const RIDE: &[f64] = &[0.0, 1.0, 1.0 + SWING, 2.0, 3.0, 3.0 + SWING]; // "ding, ding-a ding" on the ride cymbal
const BRUSH: &[f64] = &[1.0, 3.0]; // swept brushes
const RADIO_LOW_CUT: f64 = 0.93; // one-pole high-pass (~250 Hz): no bass, like a small speaker
const RADIO_HIGH_CUT: f64 = 0.45; // one-pole low-pass, applied twice (~3 kHz)
const CRACKLES_PER_SECOND: f64 = 7.0;
const CRACKLE_LENGTH: usize = 12; // samples
const CRACKLE_DECAY: f64 = 0.6; // per sample
const NOTE_RELEASE: f64 = 0.01; // note fade-out (s): a note cut dead clicks

/// Index of a track in STYLES (and in `build_music`'s result).
pub fn track_index(name: &str) -> Option<usize> {
    STYLES.iter().position(|(n, _)| *n == name)
}

/// tone() ending with a short fade to zero, for music notes.
fn note(f0: f64, f1: f64, dur: f64, kind: Wave, vol: f64, env: Env) -> Vec<f64> {
    let mut samples = tone(f0, f1, dur, kind, vol, env);
    let n = samples.len().min((NOTE_RELEASE * RATE_F) as usize);
    for (i, x) in samples.iter_mut().rev().take(n).enumerate() {
        *x *= i as f64 / n as f64;
    }
    samples
}

/// Adds samples starting at t seconds, wrapping to the start of out (the track loops).
fn add_at(out: &mut [f64], t: f64, samples: &[f64]) {
    let (start, n) = ((t * RATE_F) as usize, out.len());
    for (i, x) in samples.iter().enumerate() {
        out[(start + i) % n] += x;
    }
}

fn semitones(n: i32) -> f64 {
    2f64.powf(f64::from(n) / 12.0)
}

/// Old radio set: narrow bandwidth, hiss and vinyl crackle. The filters run twice over the loop so
/// their state at the end matches the start (no click at the loop point).
fn radio(mut track: Vec<f64>, seed: &str) -> Vec<f64> {
    let mut rng = StdRng::seed_from_u64(seed_of(&format!("{seed}-crackles")));
    let n = track.len();
    for _ in 0..(n as f64 / RATE_F * CRACKLES_PER_SECOND) as usize {
        let pos = rng.random_range(0..n);
        let amp = rng.random_range(0.05..0.3) * pick(&mut rng, &[-1.0, 1.0]);
        for i in 0..CRACKLE_LENGTH {
            track[(pos + i) % n] += amp * CRACKLE_DECAY.powi(i as i32);
        }
    }
    // +1: never shorter than the track
    let hiss = noise(
        (n + 1) as f64 / RATE_F,
        0.012,
        1e9,
        0.9,
        seed_of(&format!("{seed}-hiss")),
    );
    for (x, h) in track.iter_mut().zip(hiss) {
        *x += h;
    }
    let mut out = vec![0.0; n];
    let (mut prev_x, mut high, mut low1, mut low2) = (0.0, 0.0, 0.0, 0.0);
    for lap in 0..2 {
        for (i, &x) in track.iter().enumerate() {
            high = RADIO_LOW_CUT * (high + x - prev_x);
            prev_x = x;
            low1 += RADIO_HIGH_CUT * (high - low1);
            low2 += RADIO_HIGH_CUT * (low1 - low2);
            if lap == 1 {
                out[i] = low2;
            }
        }
    }
    out
}

fn play_beats(out: &mut [f64], bar_start: f64, beats: &[f64], samples: &[f64]) {
    for beat in beats {
        add_at(out, bar_start + beat * BEAT, samples);
    }
}

/// Walking bass: root, third, fifth, then a chromatic approach note to the next chord.
fn bass(out: &mut [f64], bar_start: f64, root: f64, chord: Chord, next_root: f64, beats: &[usize]) {
    let approach = next_root * semitones(if next_root > root { -1 } else { 1 });
    let walk = [
        root,
        root * semitones(chord[1]),
        root * semitones(chord[2]),
        approach,
    ];
    for &beat in beats {
        let f = walk[beat];
        let env = Env {
            attack: 0.004,
            ..decay(0.2)
        };
        add_at(
            out,
            bar_start + beat as f64 * BEAT,
            &note(f, f, BEAT * 0.95, Tri, 0.35, env),
        );
    }
}

/// Chord notes two octaves up, with slight vibrato like a vibraphone.
fn theme(out: &mut [f64], bar_start: f64, root: f64, chord: Chord, rng: &mut StdRng) {
    for &(start, length) in pick(rng, PHRASES) {
        let f = root * 4.0 * semitones(pick(rng, &chord));
        let env = Env {
            attack: 0.01,
            decay: Some(length * BEAT * 0.6),
            vibrato: (5.0, 0.006),
        };
        add_at(
            out,
            bar_start + start * BEAT,
            &note(f, f, length * BEAT, Sine, 0.16, env),
        );
    }
}

/// Swing loop of BARS bars (bass, piano, brushes, vibraphone theme) run through the radio.
fn track(index: usize) -> Arc<[f32]> {
    let (name, style) = &STYLES[index];
    let progression = &PROGRESSIONS
        .iter()
        .find(|(n, _)| n == name)
        .expect("progression of every style")
        .1;
    let mut out = vec![0.0; ((BARS * BEATS_PER_BAR) as f64 * BEAT * RATE_F) as usize];
    let mut rng = StdRng::seed_from_u64(seed_of(name));
    let ride = noise(0.18, 0.1, 0.06, 0.95, 21);
    let brush = noise(0.25, 0.12, 0.1, 0.4, 22);
    // Under the radio's high-pass only the kick's click gets through, so give it one.
    let kick = mix(vec![
        (0.0, note(180.0, 70.0, 0.15, Sine, 0.5, decay(0.06))),
        (0.0, noise(0.03, 0.4, 0.01, 0.5, 23)),
    ]);
    for (bar, &(root, chord)) in progression.iter().enumerate() {
        let bar_start = (bar * BEATS_PER_BAR) as f64 * BEAT;
        bass(
            &mut out,
            bar_start,
            root,
            chord,
            progression[(bar + 1) % BARS].0,
            style.bass,
        );
        let voicing = chord.iter().map(|&i| {
            (
                0.0,
                note(
                    root * 2.0 * semitones(i),
                    root * 2.0 * semitones(i),
                    0.3,
                    Tri,
                    0.06,
                    decay(0.1),
                ),
            )
        });
        play_beats(&mut out, bar_start, style.piano, &mix(voicing.collect()));
        play_beats(&mut out, bar_start, RIDE, &ride);
        play_beats(&mut out, bar_start, BRUSH, &brush);
        play_beats(&mut out, bar_start, style.kick, &kick);
        if bar % style.theme_every == 0 {
            theme(&mut out, bar_start, root, chord, &mut rng);
        }
    }
    normalize(&radio(out, name), MUSIC_PEAK)
}

/// The music tracks, in STYLES order.
pub fn build_music() -> Vec<Arc<[f32]>> {
    (0..STYLES.len()).map(track).collect()
}

pub const CEIL: f64 = 0.95; // max output peak
pub const RELEASE: f64 = 0.02; // limiter gain recovery per block of CHUNK samples

/// Gain limiter: if the block peak (× vol) exceeds CEIL, the gain ramps down until the first sample
/// that is too loud, then recovers by RELEASE per block. The waveform is never distorted (per-sample
/// clipping makes the music crackle as soon as a loud effect is added on top).
/// Returns (samples, final gain); the gain is continuous across samples and blocks.
pub fn limiter(mut buf: Vec<f64>, gain: f64, vol: f64) -> (Vec<f64>, f64) {
    let peak = buf.iter().fold(0.0, |m: f64, x| m.max(x.abs())) * vol;
    let safe_gain = if peak > CEIL { CEIL / peak } else { 1.0 };
    if safe_gain < gain {
        // Samples before the first loud one stay under CEIL with any gain ≤ gain: we have until it
        // to ramp down.
        let first_loud = buf
            .iter()
            .position(|x| x.abs() * vol * gain > CEIL)
            .unwrap_or(buf.len() - 1);
        for (i, x) in buf.iter_mut().enumerate() {
            let ramp = ((i + 1) as f64 / (first_loud + 1) as f64).min(1.0);
            *x = *x * vol * (gain + (safe_gain - gain) * ramp);
        }
        return (buf, safe_gain);
    }
    let released = safe_gain.min(gain + RELEASE);
    if released == 1.0 && gain == 1.0 {
        // common case: nothing to limit, only apply the volume
        if vol != 1.0 {
            buf.iter_mut().for_each(|x| *x *= vol);
        }
        return (buf, released);
    }
    let n = buf.len() as f64;
    for (i, x) in buf.iter_mut().enumerate() {
        *x = *x * vol * (gain + (released - gain) * (i + 1) as f64 / n);
    }
    (buf, released)
}

// --- PulseAudio output -------------------------------------------------------------

const PA_SAMPLE_S16LE: c_int = 3;
const MONO: u8 = 1;
const PA_STREAM_PLAYBACK: c_int = 1;
const PA_DEFAULT: u32 = u32::MAX; // (uint32_t) -1: value chosen by the server
const BYTES_PER_SAMPLE: usize = 2;
const LATENCY_CHUNKS: usize = 4; // ~80 ms
pub const PCM_SCALE: f64 = 32000.0; // slightly under 32767, the 16-bit integer peak
/// Inaudible background hiss (±2 steps out of 32,000, ~-84 dB) added to the output: WSLg's audio output
/// (Windows Remote Desktop) cuts a stream of pure silence and is slow to restart it, so effects come late
/// and choppy whenever the music is off.
pub const KEEPALIVE_LSB: i16 = 2;
/// One-second loop of the background hiss.
pub static KEEPALIVE: LazyLock<Vec<i16>> = LazyLock::new(|| {
    let mut rng = StdRng::seed_from_u64(0);
    (0..RATE)
        .map(|_| rng.random_range(-KEEPALIVE_LSB..=KEEPALIVE_LSB))
        .collect()
});

/// Adds the background hiss to the PCM block, starting at sample pos of its one-second loop.
pub fn keep_alive(pcm: &[i16], pos: usize) -> Vec<i16> {
    let n = KEEPALIVE.len();
    pcm.iter()
        .enumerate()
        .map(|(i, x)| x.saturating_add(KEEPALIVE[(pos + i) % n]))
        .collect()
}

#[repr(C)]
struct SampleSpec {
    format: c_int,
    rate: u32,
    channels: u8,
}

#[repr(C)]
struct BufferAttr {
    maxlength: u32,
    tlength: u32,
    prebuf: u32,
    minreq: u32,
    fragsize: u32,
}

type PaSimpleNew = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    c_int,
    *const c_char,
    *const c_char,
    *const SampleSpec,
    *const c_void,
    *const BufferAttr,
    *mut c_int,
) -> *mut c_void;
type PaSimpleWrite = unsafe extern "C" fn(*mut c_void, *const c_void, usize, *mut c_int) -> c_int;

/// Playback stream opened with libpulse-simple, loaded at runtime (no build-time dependency).
struct Pulse {
    _lib: Library, // keeps the write function loaded
    stream: *mut c_void,
    write: PaSimpleWrite,
}

impl Pulse {
    fn open() -> Option<Pulse> {
        // SAFETY: libpulse-simple has no unusual load-time side effects, and the symbols are declared
        // with their C signatures from <pulse/simple.h>.
        let lib = unsafe { Library::new("libpulse-simple.so.0") }.ok()?;
        let new: PaSimpleNew = *unsafe { lib.get(b"pa_simple_new\0") }.ok()?;
        let write: PaSimpleWrite = *unsafe { lib.get(b"pa_simple_write\0") }.ok()?;
        let spec = SampleSpec {
            format: PA_SAMPLE_S16LE,
            rate: RATE as u32,
            channels: MONO,
        };
        let target_length = (CHUNK * BYTES_PER_SAMPLE * LATENCY_CHUNKS) as u32;
        let attr = BufferAttr {
            maxlength: PA_DEFAULT,
            tlength: target_length,
            prebuf: PA_DEFAULT,
            minreq: PA_DEFAULT,
            fragsize: PA_DEFAULT,
        };
        let mut err: c_int = 0;
        // SAFETY: every pointer is either null (allowed by the API) or valid for the call.
        let stream = unsafe {
            new(
                ptr::null(),
                c"Champignon atomique".as_ptr(),
                PA_STREAM_PLAYBACK,
                ptr::null(),
                c"sfx".as_ptr(),
                &spec,
                ptr::null(),
                &attr,
                &mut err,
            )
        };
        (!stream.is_null()).then_some(Pulse {
            _lib: lib,
            stream,
            write,
        })
    }

    /// Blocks until the samples are queued; false when the stream failed.
    fn write(&self, pcm: &[i16]) -> bool {
        let bytes: Vec<u8> = pcm.iter().flat_map(|x| x.to_le_bytes()).collect();
        let mut err: c_int = 0;
        // SAFETY: the stream is open and bytes is valid for its length.
        unsafe { (self.write)(self.stream, bytes.as_ptr().cast(), bytes.len(), &mut err) >= 0 }
    }
}

// --- Mixing -------------------------------------------------------------------------

/// A sound being played.
struct Voice {
    samples: Arc<[f32]>,
    pos: usize,
    vol: f64,
    prio: u8,
    fading: bool, // cut off by a more important jingle: fades out over the next block
}

const NO_TRACK: u8 = u8::MAX;

/// State shared between the UI thread (settings, play) and the mixer thread.
struct Shared {
    ok: AtomicBool,
    muted: AtomicBool,
    music_on: AtomicBool,
    volume: AtomicU32,      // f32 bits, master volume (effects and music), 0..1
    wanted_music: AtomicU8, // index in STYLES, NO_TRACK for none
    voices: Mutex<Vec<Voice>>,
    sounds: OnceLock<Bank>,
    tracks: OnceLock<Vec<Arc<[f32]>>>,
}

impl Shared {
    fn voices(&self) -> MutexGuard<'_, Vec<Voice>> {
        self.voices.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What the UI needs from the sound system, so that it can be tested with a fake.
pub trait SoundSink {
    /// The output works (false when PulseAudio is missing or failed).
    fn ok(&self) -> bool;
    fn play(&self, name: &str);
    /// Wanted music track (STYLES name), crossfaded when it changes; an unknown name means none.
    fn set_music(&self, track: &str);
    fn set_muted(&self, muted: bool);
    fn set_music_on(&self, on: bool);
    /// Master volume (effects and music), 0..1.
    fn set_volume(&self, volume: f32);
}

pub struct Sound {
    shared: Arc<Shared>,
}

impl Sound {
    pub fn new(enabled: bool) -> Sound {
        let shared = Arc::new(Shared {
            ok: AtomicBool::new(false),
            muted: AtomicBool::new(false),
            music_on: AtomicBool::new(true),
            volume: AtomicU32::new(1f32.to_bits()),
            wanted_music: AtomicU8::new(NO_TRACK),
            voices: Mutex::new(Vec::new()),
            sounds: OnceLock::new(),
            tracks: OnceLock::new(),
        });
        if enabled {
            let shared = Arc::clone(&shared);
            // No thread, no sound: play() stays a no-op.
            let _ = thread::Builder::new()
                .name("sound".into())
                .spawn(move || run(&shared));
        }
        Sound { shared }
    }

    /// Plays a sound at a given volume (0..1), following the priority rules.
    pub fn play_at(&self, name: &str, vol: f64) {
        let shared = &self.shared;
        if shared.muted.load(Ordering::Relaxed) || !shared.ok.load(Ordering::Acquire) {
            return;
        }
        let Some(samples) = shared.sounds.get().and_then(|bank| bank.get(name)) else {
            return;
        };
        let prio = priority(name);
        let mut voices = shared.voices();
        if prio >= JINGLE {
            if voices.iter().any(|v| v.prio >= prio && !v.fading) {
                return;
            }
            for v in voices.iter_mut().filter(|v| v.prio >= JINGLE) {
                v.fading = true;
            }
        } else {
            // drop the oldest sound effects beyond the limit
            let minor = voices.iter().filter(|v| v.prio < JINGLE).count();
            let mut over_limit = (minor + 1).saturating_sub(MAX_MINOR);
            voices.retain(|v| {
                let drop = over_limit > 0 && v.prio < JINGLE;
                over_limit -= usize::from(drop);
                !drop
            });
        }
        voices.push(Voice {
            samples: Arc::clone(samples),
            pos: 0,
            vol,
            prio,
            fading: false,
        });
        let excess = voices.len().saturating_sub(MAX_VOICES);
        voices.drain(..excess);
    }
}

impl SoundSink for Sound {
    fn ok(&self) -> bool {
        self.shared.ok.load(Ordering::Acquire)
    }

    fn play(&self, name: &str) {
        self.play_at(name, 1.0);
    }

    fn set_music(&self, track: &str) {
        let index = track_index(track).map_or(NO_TRACK, |i| i as u8);
        self.shared.wanted_music.store(index, Ordering::Relaxed);
    }

    fn set_muted(&self, muted: bool) {
        self.shared.muted.store(muted, Ordering::Relaxed);
    }

    fn set_music_on(&self, on: bool) {
        self.shared.music_on.store(on, Ordering::Relaxed);
    }

    fn set_volume(&self, volume: f32) {
        self.shared
            .volume
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
}

/// Mixer thread: opens the output, builds the sounds, then writes blocks until the stream fails.
fn run(shared: &Arc<Shared>) {
    let Some(pulse) = Pulse::open() else {
        return;
    };
    let _ = shared.sounds.set(build());
    shared.ok.store(true, Ordering::Release);
    // Music (slower to compute) is generated alongside so it doesn't delay the effects.
    let music_shared = Arc::clone(shared);
    let _ = thread::Builder::new().name("music".into()).spawn(move || {
        let _ = music_shared.tracks.set(build_music());
    });
    let mut mixer = Mixer::new(Arc::clone(shared));
    let mut floor_pos = 0;
    loop {
        let pcm = keep_alive(&mixer.mix(), floor_pos);
        floor_pos = (floor_pos + CHUNK) % KEEPALIVE.len();
        if !pulse.write(&pcm) {
            shared.ok.store(false, Ordering::Release);
            return;
        }
    }
}

/// State owned by the mixer thread.
struct Mixer {
    shared: Arc<Shared>,
    music: Option<usize>, // track playing (or fading out), index in STYLES
    music_gain: f64,
    music_pos: usize,
    limit_gain: f64,
}

impl Mixer {
    fn new(shared: Arc<Shared>) -> Mixer {
        Mixer {
            shared,
            music: None,
            music_gain: 0.0,
            music_pos: 0,
            limit_gain: 1.0,
        }
    }

    /// Mixes the active voices over a block of CHUNK samples, returns 16-bit PCM.
    fn mix(&mut self) -> Vec<i16> {
        let mut buf = vec![0.0; CHUNK];
        {
            let mut voices = self.shared.voices();
            if self.shared.muted.load(Ordering::Relaxed) {
                voices.clear();
            }
            let jingle_playing = voices.iter().any(|v| v.prio >= JINGLE && !v.fading);
            for v in voices.iter_mut() {
                let vol = if v.prio < JINGLE && jingle_playing {
                    v.vol * DUCK
                } else {
                    v.vol
                };
                let end = v.samples.len().min(v.pos + CHUNK);
                for (i, &x) in v.samples.get(v.pos..end).unwrap_or(&[]).iter().enumerate() {
                    let fade = if v.fading {
                        1.0 - i as f64 / CHUNK as f64
                    } else {
                        1.0
                    };
                    buf[i] += f64::from(x) * vol * fade;
                }
                v.pos += CHUNK;
            }
            voices.retain(|v| v.pos < v.samples.len() && !v.fading);
        }
        self.mix_music(&mut buf);
        let volume = f64::from(f32::from_bits(self.shared.volume.load(Ordering::Relaxed)));
        let (out, gain) = limiter(buf, self.limit_gain, volume);
        self.limit_gain = gain;
        out.iter().map(|x| (x * PCM_SCALE) as i16).collect()
    }

    /// Adds the looping track, fading out then in when the wanted track changes.
    fn mix_music(&mut self, buf: &mut [f64]) {
        let muted = self.shared.muted.load(Ordering::Relaxed);
        if muted {
            (self.music, self.music_gain) = (None, 0.0);
        }
        let wanted = match self.shared.wanted_music.load(Ordering::Relaxed) {
            index
                if index != NO_TRACK && self.shared.music_on.load(Ordering::Relaxed) && !muted =>
            {
                Some(usize::from(index))
            }
            _ => None,
        };
        let tracks = self.shared.tracks.get();
        if self.music != wanted {
            self.music_gain -= FADE_PER_CHUNK;
            if self.music_gain <= 0.0 || self.music.is_none() {
                (self.music, self.music_gain) = (wanted, 0.0);
            }
        } else if tracks.is_some() && self.music.is_some() {
            self.music_gain = (self.music_gain + FADE_PER_CHUNK).min(1.0);
        }
        let Some(track) = self
            .music
            .and_then(|i| tracks?.get(i))
            .filter(|t| !t.is_empty())
        else {
            return;
        };
        let (length, pos, gain) = (track.len(), self.music_pos, self.music_gain * MUSIC_VOL);
        if self.music_gain > 0.0 {
            for (i, b) in buf.iter_mut().enumerate() {
                *b += f64::from(track[(pos + i) % length]) * gain;
            }
        }
        self.music_pos = (pos + CHUNK) % length;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    static BANK: LazyLock<Bank> = LazyLock::new(build);
    static TRACKS: LazyLock<Vec<Arc<[f32]>>> = LazyLock::new(build_music);

    fn peak(samples: &[f32]) -> f64 {
        samples
            .iter()
            .fold(0.0, |m: f64, &x| m.max(f64::from(x).abs()))
    }

    fn peak64(samples: &[f64]) -> f64 {
        samples.iter().fold(0.0, |m: f64, x| m.max(x.abs()))
    }

    /// Enabled sound (as if PulseAudio were there) with the given sounds, without any thread.
    fn sound_with(sounds: &[(&str, Vec<f32>)]) -> (Sound, Mixer) {
        let snd = Sound::new(false);
        snd.shared.ok.store(true, Ordering::Release);
        let bank = sounds
            .iter()
            .map(|(n, s)| (n.to_string(), Arc::from(s.as_slice())))
            .collect();
        let _ = snd.shared.sounds.set(bank);
        let mixer = Mixer::new(Arc::clone(&snd.shared));
        (snd, mixer)
    }

    fn music_sound() -> (Sound, Mixer) {
        let (snd, mixer) = sound_with(&[]);
        let _ = snd.shared.tracks.set(TRACKS.clone());
        (snd, mixer)
    }

    fn run_chunks(mixer: &mut Mixer, n: usize) -> Vec<Vec<i16>> {
        (0..n).map(|_| mixer.mix()).collect()
    }

    fn voice_count(snd: &Sound) -> usize {
        snd.shared.voices().len()
    }

    /// Rust source files of the crate, recursively.
    fn rust_sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("readable src dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                rust_sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// Sound names emitted in the code (g.sfx.push_back("x") / .play("x")), outside this file whose
    /// tests play fake sounds.
    fn used_sound_names() -> Vec<String> {
        let mut files = Vec::new();
        rust_sources(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut files,
        );
        let mut names = Vec::new();
        for file in files.iter().filter(|f| !f.ends_with("src/sound.rs")) {
            let text = std::fs::read_to_string(file).expect("readable source");
            for pattern in ["sfx.push_back(\"", ".play(\""] {
                for (at, _) in text.match_indices(pattern) {
                    let rest = &text[at + pattern.len()..];
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if rest[name.len()..].starts_with("\")") {
                        names.push(name);
                    }
                }
            }
        }
        names
    }

    // --- Synthesis

    #[test]
    fn tone_length_and_envelope() {
        let s = tone(440.0, 440.0, 0.1, Sine, 0.5, PLAIN);
        assert_eq!(s.len(), (0.1 * RATE_F) as usize);
        assert!(s[0].abs() < 1e-3);
        assert!(peak64(&s) <= 0.5);
    }

    #[test]
    fn oscillators_bounded() {
        for kind in [Sine, Square, Tri, Saw] {
            let values: Vec<f64> = (0..100).map(|p| osc(kind, f64::from(p) / 100.0)).collect();
            assert!(values.iter().all(|v| (-1.0..=1.0).contains(v)), "{kind:?}");
        }
    }

    #[test]
    fn noise_deterministic() {
        assert_eq!(noise(0.05, 0.3, 0.2, 0.2, 3), noise(0.05, 0.3, 0.2, 0.2, 3));
        assert_ne!(noise(0.05, 0.3, 0.2, 0.2, 3), noise(0.05, 0.3, 0.2, 0.2, 4));
    }

    #[test]
    fn mix_and_seq() {
        let (a, b) = (vec![1.0; 10], vec![1.0; 10]);
        let m = mix(vec![(0.0, a.clone()), (5.0 / RATE_F, b.clone())]);
        assert_eq!(m.len(), 15);
        assert_eq!(m[7], 2.0);
        assert_eq!(seq(vec![a, b], 0.0).len(), 20);
    }

    // --- Effect bank and limiter

    #[test]
    fn levels() {
        for (name, s) in BANK.iter() {
            let target = level(name);
            assert!((peak(s) - target).abs() < 1e-4, "{name}");
        }
        assert_eq!(level("click3"), 0.45);
        assert_eq!(level("boom"), DEFAULT_LEVEL);
    }

    #[test]
    fn level_keys_exist() {
        let families: Vec<&str> = BANK
            .keys()
            .map(|n| n.trim_end_matches(|c: char| c.is_ascii_digit()))
            .collect();
        for (name, _) in LEVEL {
            assert!(families.contains(name), "{name}");
        }
    }

    #[test]
    fn priority_keys_exist() {
        for (name, prio) in PRIORITY {
            assert!(BANK.contains_key(*name), "{name}");
            assert!(*prio >= JINGLE, "{name}");
        }
        assert_eq!(priority("click0"), 1);
    }

    #[test]
    fn limiter_transparent_below_ceiling() {
        let buf = vec![0.5, -0.9, 0.2];
        assert_eq!(limiter(buf.clone(), 1.0, 1.0), (buf, 1.0));
        assert_eq!(limiter(vec![], 1.0, 1.0), (vec![], 1.0));
    }

    #[test]
    fn limiter_attack_ramps_before_peak() {
        let buf: Vec<f64> = [vec![0.5; 10], vec![1.9], vec![0.5; 9]].concat();
        let (out, gain) = limiter(buf.clone(), 1.0, 1.0);
        assert!((gain - 0.5).abs() < 1e-7);
        assert!((peak64(&out) - CEIL).abs() < 1e-7);
        let gains: Vec<f64> = out.iter().zip(&buf).map(|(o, x)| o / x).collect();
        assert!(gains[0] > 0.9); // no sudden drop at the start of the block
        assert!(gains.windows(2).all(|w| w[1] <= w[0] + 1e-12));
        assert!(gains[10..].iter().all(|g| (g - 0.5).abs() < 1e-9));
    }

    #[test]
    fn limiter_never_exceeds_ceiling() {
        let mut rng = StdRng::seed_from_u64(0);
        let mut gain = 1.0;
        for _ in 0..200 {
            let buf: Vec<f64> = (0..50)
                .map(|_| rng.random_range(-3.0..3.0) * rng.random::<f64>().powi(3))
                .collect();
            let vol = pick(&mut rng, &[1.0, 0.5, 0.2]);
            let (out, g) = limiter(buf, gain, vol);
            gain = g;
            assert!(peak64(&out) <= CEIL + 1e-9);
        }
    }

    #[test]
    fn limiter_releases_gradually() {
        let (mut gain, mut gains, mut last) = (0.5, Vec::new(), 0.0);
        for _ in 0..30 {
            let (out, g) = limiter(vec![0.5; 10], gain, 1.0);
            gain = g;
            gains.push(gain);
            // ramp without jumps, even across blocks
            assert!(
                std::iter::once(&last)
                    .chain(&out)
                    .collect::<Vec<_>>()
                    .windows(2)
                    .all(|w| w[1] >= w[0])
            );
            assert!((out[9] - 0.5 * gain).abs() < 1e-7); // the block ends at the returned gain
            last = out[9];
        }
        assert!((gains[0] - (0.5 + RELEASE)).abs() < 1e-7);
        assert_eq!(gains[29], 1.0);
    }

    #[test]
    fn every_emitted_sound_exists() {
        let missing: Vec<String> = used_sound_names()
            .into_iter()
            .filter(|n| !BANK.contains_key(n))
            .collect();
        assert!(
            missing.is_empty(),
            "sounds emitted but never synthesized: {missing:?}"
        );
        for i in 0..8 {
            assert!(BANK.contains_key(&format!("click{i}")));
        }
    }

    // --- Music

    #[test]
    fn tracks_same_length_no_clipping() {
        assert_eq!(TRACKS.len(), STYLES.len());
        for (track, (name, _)) in TRACKS.iter().zip(&STYLES) {
            // same length: switching tracks keeps the beat
            assert_eq!(
                track.len(),
                (BARS as f64 * 4.0 * BEAT * RATE_F) as usize,
                "{name}"
            );
            assert!(peak(track) <= 0.9 + 1e-6, "{name}");
        }
    }

    #[test]
    fn music_deterministic() {
        assert_eq!(*TRACKS[0], *track(0));
        assert_ne!(*TRACKS[0], *TRACKS[2]);
    }

    #[test]
    fn every_track_has_chords_and_style() {
        let style_names: Vec<&str> = STYLES.iter().map(|(n, _)| *n).collect();
        let progression_names: Vec<&str> = PROGRESSIONS.iter().map(|(n, _)| *n).collect();
        assert_eq!(style_names, progression_names);
        for (name, style) in &STYLES {
            assert!(style.bass.iter().all(|b| *b <= 3), "{name}"); // indices into the bass walk
            assert!(style.theme_every >= 1, "{name}");
        }
    }

    #[test]
    fn fade_in() {
        let (snd, mut mixer) = music_sound();
        snd.set_music("calm");
        mixer.mix();
        assert!(mixer.music_gain < 0.1);
        run_chunks(&mut mixer, 60);
        assert_eq!(mixer.music_gain, 1.0);
    }

    #[test]
    fn fade_waits_for_tracks() {
        let (snd, mut mixer) = sound_with(&[]);
        snd.set_music("calm");
        run_chunks(&mut mixer, 80);
        assert_eq!(mixer.music_gain, 0.0);
        let _ = snd.shared.tracks.set(TRACKS.clone());
        mixer.mix();
        assert!(mixer.music_gain > 0.0 && mixer.music_gain < 0.1);
    }

    #[test]
    fn mute_cuts_music_at_once() {
        let (snd, mut mixer) = music_sound();
        snd.set_music("calm");
        run_chunks(&mut mixer, 60);
        snd.set_muted(true);
        assert!(mixer.mix().iter().all(|&x| x == 0));
    }

    #[test]
    fn crossfade() {
        let (snd, mut mixer) = music_sound();
        snd.set_music("calm");
        run_chunks(&mut mixer, 60);
        snd.set_music("epic");
        run_chunks(&mut mixer, 5);
        assert_eq!(mixer.music, track_index("calm")); // fade out first
        run_chunks(&mut mixer, 60);
        assert_eq!(mixer.music, track_index("epic"));
    }

    #[test]
    fn loops() {
        let (snd, mut mixer) = music_sound();
        snd.set_music("calm");
        let length = TRACKS[0].len();
        run_chunks(&mut mixer, length / CHUNK + 2);
        assert!(mixer.music_pos < length);
        assert!(mixer.mix().iter().any(|&x| x != 0));
    }

    #[test]
    fn music_off_and_mute() {
        let (snd, mut mixer) = music_sound();
        snd.set_music("calm");
        run_chunks(&mut mixer, 60);
        snd.set_music_on(false);
        run_chunks(&mut mixer, 60);
        assert_eq!(mixer.music, None);
        assert!(mixer.mix().iter().all(|&x| x == 0));
        snd.set_music_on(true);
        snd.set_muted(true);
        run_chunks(&mut mixer, 60);
        assert_eq!(mixer.music, None);
    }

    // --- Keep-alive hiss

    #[test]
    fn silence_never_pure_and_signal_kept() {
        let out = keep_alive(&[0; CHUNK], 0);
        assert!(out.iter().any(|&x| x != 0)); // never pure silence: WSLg would cut the stream
        let mut distinct = KEEPALIVE.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert!(distinct.len() > 2); // a hiss, not a constant
        assert!(out.iter().all(|x| x.abs() <= KEEPALIVE_LSB));
        let scale = PCM_SCALE as i16;
        let loud: Vec<i16> = (0..CHUNK)
            .map(|i| if i % 2 == 0 { scale } else { -scale })
            .collect();
        let mixed = keep_alive(&loud, KEEPALIVE.len() - 3); // hiss wraps around
        assert!(
            mixed
                .iter()
                .zip(&loud)
                .all(|(a, b)| (a - b).abs() <= KEEPALIVE_LSB)
        );
    }

    // --- Mixer

    fn mixer_sounds() -> (Sound, Mixer) {
        sound_with(&[("half", vec![0.5; CHUNK + 10]), ("loud", vec![0.8; 5])])
    }

    #[test]
    fn disabled_is_silent_noop() {
        let snd = Sound::new(false);
        assert!(!snd.ok());
        snd.play("boom");
        assert_eq!(voice_count(&snd), 0);
    }

    #[test]
    fn voice_plays_then_ends() {
        let (snd, mut mixer) = mixer_sounds();
        snd.play("half");
        let first = mixer.mix();
        assert_eq!(first.len(), CHUNK);
        assert!(first.iter().all(|&x| x == 16000));
        let second = mixer.mix();
        assert_eq!(second[..10], [16000; 10]);
        assert_eq!(second[10], 0);
        assert_eq!(voice_count(&snd), 0);
    }

    #[test]
    fn volume_and_clamp() {
        let (snd, mut mixer) = mixer_sounds();
        snd.play_at("loud", 0.5);
        assert_eq!(mixer.mix()[0], 12800);
        snd.play("loud");
        snd.play("loud");
        let expected = CEIL * PCM_SCALE; // limiter: capped at CEIL
        assert!((f64::from(mixer.mix()[0]) - expected).abs() <= 2.0);
    }

    #[test]
    fn master_volume() {
        let (snd, mut mixer) = mixer_sounds();
        snd.set_volume(0.5);
        snd.play("half");
        assert_eq!(mixer.mix()[0], 8000);
    }

    #[test]
    fn unknown_sound_ignored() {
        let (snd, _) = mixer_sounds();
        snd.play("unknown");
        assert_eq!(voice_count(&snd), 0);
    }

    #[test]
    fn mute() {
        let (snd, mut mixer) = mixer_sounds();
        snd.play("half");
        snd.set_muted(true);
        snd.play("half");
        assert_eq!(voice_count(&snd), 1); // no new sound
        assert!(mixer.mix().iter().all(|&x| x == 0)); // and playing ones are cut
        assert_eq!(voice_count(&snd), 0);
    }

    #[test]
    fn minor_voice_limit() {
        let (snd, _) = mixer_sounds();
        for _ in 0..MAX_MINOR + 5 {
            snd.play("half");
        }
        assert_eq!(voice_count(&snd), MAX_MINOR);
    }

    // --- Priorities

    fn priority_sounds() -> (Sound, Mixer) {
        let long = vec![0.5; CHUNK * 10];
        sound_with(&[
            ("level", long.clone()),
            ("achievement", long.clone()),
            ("boom", long.clone()),
            ("click0", long),
        ])
    }

    #[test]
    fn one_jingle_at_a_time() {
        let (snd, _) = priority_sounds();
        snd.play("achievement");
        snd.play("achievement");
        snd.play("level"); // less important: ignored
        assert_eq!(voice_count(&snd), 1);
    }

    #[test]
    fn more_important_jingle_cuts_previous() {
        let (snd, mut mixer) = priority_sounds();
        snd.play("level");
        snd.play("boom");
        let fading: Vec<bool> = snd.shared.voices().iter().map(|v| v.fading).collect();
        assert_eq!(fading, [true, false]); // level fading out
        mixer.mix();
        assert_eq!(voice_count(&snd), 1);
        assert_eq!(snd.shared.voices()[0].prio, priority("boom"));
        snd.play("level"); // the boom is still playing
        assert_eq!(voice_count(&snd), 1);
    }

    #[test]
    fn minor_sounds_ducked_during_jingle() {
        let (snd, mut mixer) = priority_sounds();
        snd.play("click0");
        let alone = mixer.mix()[0];
        snd.play("level");
        {
            // replace the jingle by a silent one, to hear only the ducked click
            let mut voices = snd.shared.voices();
            voices.retain(|v| v.prio < JINGLE);
            let silence: Arc<[f32]> = Arc::from(vec![0.0; CHUNK * 5]);
            voices.push(Voice {
                samples: silence,
                pos: 0,
                vol: 1.0,
                prio: JINGLE,
                fading: false,
            });
        }
        assert_eq!(f64::from(mixer.mix()[0]), f64::from(alone) * DUCK);
    }

    #[test]
    fn minor_sounds_coexist_with_jingle() {
        let (snd, _) = priority_sounds();
        snd.play("level");
        snd.play("click0");
        assert_eq!(voice_count(&snd), 2);
    }
}
