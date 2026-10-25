"""Sound effects synthesized on the fly, played through PulseAudio (libpulse-simple, no Python dependency).

Sounds are generated once at startup (stdlib only), then mixed in a thread that writes small blocks
to a PulseAudio stream. Without PulseAudio, everything is silently disabled.
"""

import ctypes
import ctypes.util
import math
import random
import threading
from array import array

RATE = 22050
CHUNK = 441  # 20 ms
MAX_VOICES = 10
MAX_MINOR = 4  # max short sound effects (presses, ticks...) playing at once
DUCK = 0.5  # sound effect volume while a jingle plays
# Sound priorities. From JINGLE up, they are exclusive jingles: one at a time, a more important jingle
# cuts off the current one, a less (or equally) important one is dropped.
# The others (1 by default) are sound effects, capped at MAX_MINOR voices.
JINGLE = 2
PRIORITY = dict(supernova=6, prestige=6, boom=5, perfect=5, fanfare=4, synergy=4, achievement=4,
                mutation=3, alarm=3, freeze=3, chased=3, milestone=3, chest=3, enemy=3,
                talent=2, drop=2, partial=2, fizzle=2, level=2, buy=2)
MUSIC_VOL = 0.25
# Target peak per effect after normalization (DEFAULT_LEVEL otherwise): frequent effects stay quieter.
LEVEL = dict(tick=0.3, click=0.45, jam=0.45, plane=0.5, charge=0.5, alarm=0.6)
DEFAULT_LEVEL = 0.8
MUSIC_PEAK = 0.9
MUSIC_FADE_SECONDS = 1.0
FADE_PER_CHUNK = CHUNK / RATE / MUSIC_FADE_SECONDS


# --- Synthesis -----------------------------------------------------------------------

def _osc(kind, phase):
    p = phase % 1.0
    if kind == "sine":
        return math.sin(2 * math.pi * p)
    if kind == "square":
        return 1.0 if p < 0.5 else -1.0
    if kind == "tri":
        return 4 * abs(p - 0.5) - 1
    return 2 * p - 1  # saw


def tone(f0, f1, dur, kind="square", vol=0.3, attack=0.005, decay=None, vibrato=(0, 0)):
    """Note with an exponential glide f0 → f1, attack + decay envelope."""
    length = int(dur * RATE)
    out, phase = [], 0.0
    vibrato_freq, vibrato_depth = vibrato
    for i in range(length):
        t = i / RATE
        freq = f0 * (f1 / f0) ** (i / length)
        if vibrato_freq:
            freq *= 1 + vibrato_depth * math.sin(2 * math.pi * vibrato_freq * t)
        phase += freq / RATE
        fade_in = min(1.0, t / attack) if attack else 1.0
        fade_out = math.exp(-t / decay) if decay else 1 - i / length
        out.append(_osc(kind, phase) * (fade_in * fade_out) * vol)
    return out


def noise(dur, vol=0.3, decay=0.2, cutoff=0.2, seed=0):
    """Low-pass filtered (one-pole) white noise with exponential decay."""
    rng = random.Random(seed)
    out, filtered = [], 0.0
    for i in range(int(dur * RATE)):
        filtered += cutoff * (rng.uniform(-1, 1) - filtered)
        out.append(filtered * vol * math.exp(-i / RATE / decay))
    return out


def mix(*parts):
    """Overlays (offset in s, samples) pairs."""
    out = [0.0] * max(int(offset * RATE) + len(samples) for offset, samples in parts)
    for offset, samples in parts:
        start = int(offset * RATE)
        for i, x in enumerate(samples):
            out[start + i] += x
    return out


def seq(*notes, gap=0.0):
    """Plays sounds one after another."""
    parts, t = [], 0.0
    for samples in notes:
        parts.append((t, samples))
        t += len(samples) / RATE + gap
    return mix(*parts)


def _normalize(samples, peak):
    gain = peak / max(map(abs, samples))
    return array("f", (x * gain for x in samples))


def _build():
    s = {}
    for i in range(8):
        pitch = 220 * 2 ** (i / 6)
        s[f"click{i}"] = tone(pitch * 1.5, pitch, 0.035, "square", 0.12)
    s["crit"] = mix((0, tone(1400, 300, 0.15, "saw", 0.25)), (0, noise(0.12, 0.25, 0.05, 0.6, 1)))
    s["level"] = seq(*(tone(f, f, 0.05, "square", 0.15) for f in (523, 659, 784)))
    s["boom"] = mix((0, noise(1.6, 0.8, 0.45, 0.06, 2)), (0, tone(90, 30, 1.2, "sine", 0.55, decay=0.4)),
                    (0, noise(0.08, 0.5, 0.03, 0.7, 3)))
    s["overheat"] = mix((0, tone(700, 120, 0.5, "saw", 0.22)), (0, noise(0.6, 0.15, 0.3, 0.8, 4)))
    s["nope"] = tone(110, 100, 0.12, "square", 0.15)
    s["jam"] = tone(140, 90, 0.08, "square", 0.15)  # press refused during overheat
    s["buy"] = seq(tone(988, 988, 0.06, "square", 0.15), tone(1319, 1319, 0.22, "square", 0.15, decay=0.08))
    s["milestone"] = mix((0, seq(*(tone(f, f, 0.06, "square", 0.12) for f in (784, 988, 1175, 1568)))),
                         (0.24, tone(2093, 2093, 0.3, "tri", 0.2, decay=0.1)))
    s["chest"] = mix((0, tone(300, 300, 0.05, "square", 0.12)), (0.06, noise(0.08, 0.2, 0.03, 0.5, 13)),
                     (0.12, seq(*(tone(f, f, 0.05, "tri", 0.3) for f in (1047, 1319, 1568, 2093)))))
    s["talent"] = seq(*(tone(f, f, 0.07, "tri", 0.3) for f in (392, 523, 784)))
    s["tick"] = tone(1800, 1800, 0.008, "sine", 0.06)
    s["achievement"] = seq(*(tone(f, f, d, "tri", 0.35, decay=d) for f, d in
                             ((523, 0.09), (659, 0.09), (784, 0.09), (1047, 0.45))))
    s["plane"] = mix((0, tone(110, 95, 1.6, "saw", 0.12, attack=0.4, vibrato=(7, 0.02))),
                     (0, noise(1.6, 0.12, 1.0, 0.05, 5)))
    s["drop"] = tone(400, 1600, 0.16, "square", 0.15)
    s["alarm"] = seq(*(tone(f, f, 0.22, "square", 0.12, attack=0.02) for f in (660, 880, 660, 880)))
    s["hit"] = mix((0, tone(160, 60, 0.1, "sine", 0.6, decay=0.04)), (0, noise(0.06, 0.4, 0.02, 0.5, 6)))
    s["enemy"] = seq(*(tone(f, f, 0.14, "saw", 0.15, attack=0.01) for f in (392, 294, 392, 294)))
    s["chased"] = tone(1300, 250, 0.5, "sine", 0.3, vibrato=(9, 0.03))
    s["freeze"] = seq(*(tone(f, f, 0.12, "tri", 0.25, decay=0.1) for f in (2093, 1568, 1047, 784)))
    s["charge"] = tone(200, 900, 0.45, "saw", 0.12, attack=0.05)
    s["perfect"] = mix((0, s["boom"]), (0.15, s["achievement"]))
    s["partial"] = seq(tone(659, 659, 0.08, "tri", 0.3), tone(880, 880, 0.2, "tri", 0.3, decay=0.1))
    s["fizzle"] = mix((0, noise(0.5, 0.35, 0.2, 0.3, 7)), (0, tone(300, 80, 0.5, "sine", 0.2)))
    s["mutation"] = tone(300, 600, 0.7, "sine", 0.35, attack=0.05, vibrato=(6, 0.15))
    s["prestige"] = mix((0, tone(100, 1600, 1.4, "saw", 0.15, attack=0.3)), (1.2, s["boom"]))
    s["supernova"] = mix((0, tone(80, 2400, 2.2, "saw", 0.15, attack=0.5)), (2.0, s["boom"]), (2.6, s["boom"]))
    s["synergy"] = mix((0, s["mutation"]), (0.35, seq(*(tone(f, f, 0.08, "tri", 0.3) for f in (659, 831, 988, 1319)))))
    s["fanfare"] = seq(*(tone(f, f, d, "square", 0.15) for f, d in
                         ((392, 0.1), (392, 0.1), (392, 0.1), (523, 0.5))), gap=0.03)
    # Bring each sound to its target peak: stacked layers don't clip, faint sounds stay audible.
    return {name: _normalize(samples, LEVEL.get(name.rstrip("0123456789"), DEFAULT_LEVEL))
            for name, samples in s.items()}


# --- Music -----------------------------------------------------------------------------

BEAT = 0.5  # 120 bpm
BEATS_PER_BAR = 4
BARS = 8
SWING = 2 / 3  # the off-beat eighth lands two thirds into the beat (triplet swing)
C3, D3, E2, F2, G2, A2, B2 = 130.8, 146.8, 82.4, 87.3, 98.0, 110.0, 123.5
# Chords in semitones above the root: sixths and sevenths of 1940s jazz.
MAJ6, MIN6, MIN7, DOM7 = (0, 4, 7, 9), (0, 3, 7, 9), (0, 3, 7, 10), (0, 4, 7, 10)
# One (root, chord) per bar, fallout-shelter radio style: crooner, film noir, big band.
PROGRESSIONS = {
    "calm": [(C3, MAJ6), (A2, DOM7), (D3, MIN7), (G2, DOM7), (C3, MAJ6), (E2, DOM7), (F2, MAJ6), (G2, DOM7)],
    "tension": [(A2, MIN6), (A2, MIN6), (D3, MIN6), (E2, DOM7), (A2, MIN6), (F2, DOM7), (B2, MIN7), (E2, DOM7)],
    "epic": [(C3, MAJ6), (C3, MAJ6), (F2, MAJ6), (F2, MAJ6), (C3, MAJ6), (A2, DOM7), (D3, MIN7), (G2, DOM7)],
}
# Theme rhythms (note start in beats, length in beats), picked at random bar by bar.
PHRASES = [((0, 1.5), (1.5, 0.5), (2, 2)), ((0, SWING), (SWING, 1 - SWING), (1, 1), (3, 1)),
           ((1, 1), (2, 1), (3, 1)), ((0, 3),), ((0.5 + SWING / 2, 1), (2, 0.5), (2 + SWING, 1.3))]
# Arrangement of each track, in beats of the bar. Bass: beats played among root, third, fifth, approach;
# theme: played one bar out of theme_every.
STYLES = {
    "calm": dict(bass=(0, 2), piano=(1, 3), kick=(), theme_every=1),  # crooner ballad
    "tension": dict(bass=(0, 1, 2, 3), piano=(1, 3), kick=(), theme_every=2),  # suspended theme
    "epic": dict(bass=(0, 1, 2, 3), piano=(1, 3, 2 + SWING, 3 + SWING), kick=(0, 2), theme_every=1),  # big band
}
RIDE = (0, 1, 1 + SWING, 2, 3, 3 + SWING)  # "ding, ding-a ding" on the ride cymbal
BRUSH = (1, 3)  # swept brushes
RADIO_LOW_CUT = 0.93  # one-pole high-pass (~250 Hz): no bass, like a small speaker
RADIO_HIGH_CUT = 0.45  # one-pole low-pass, applied twice (~3 kHz)
CRACKLES_PER_SECOND = 7
CRACKLE_LENGTH, CRACKLE_DECAY = 12, 0.6  # samples, decay per sample
NOTE_RELEASE = 0.01  # note fade-out (s): a note cut dead clicks


def _note(*args, **kwargs):
    """tone() ending with a short fade to zero, for music notes."""
    samples = tone(*args, **kwargs)
    n = min(len(samples), int(NOTE_RELEASE * RATE))
    for i in range(n):
        samples[-1 - i] *= i / n
    return samples


def _add_at(out, t, samples):
    """Adds samples starting at t seconds, wrapping to the start of out (the track loops)."""
    start, n = int(t * RATE), len(out)
    for i, x in enumerate(samples):
        out[(start + i) % n] += x


def _semitones(n):
    return 2 ** (n / 12)


def _radio(track, seed):
    """Old radio set: narrow bandwidth, hiss and vinyl crackle. The filters run twice over the loop so
    their state at the end matches the start (no click at the loop point)."""
    rng = random.Random(f"{seed}-craquements")
    n = len(track)
    for _ in range(int(n / RATE * CRACKLES_PER_SECOND)):
        pos, amp = rng.randrange(n), rng.uniform(0.05, 0.3) * rng.choice((-1, 1))
        for i in range(CRACKLE_LENGTH):
            track[(pos + i) % n] += amp * CRACKLE_DECAY ** i
    hiss = noise((n + 1) / RATE, 0.012, 1e9, 0.9, f"{seed}-souffle")[:n]  # +1: never shorter than the track
    track = [x + h for x, h in zip(track, hiss)]
    out = [0.0] * n
    prev_x = high = low1 = low2 = 0.0
    for lap in range(2):
        for i, x in enumerate(track):
            high = RADIO_LOW_CUT * (high + x - prev_x)
            prev_x = x
            low1 += RADIO_HIGH_CUT * (high - low1)
            low2 += RADIO_HIGH_CUT * (low1 - low2)
            if lap:
                out[i] = low2
    return out


def _play(out, bar_start, beats, samples):
    for beat in beats:
        _add_at(out, bar_start + beat * BEAT, samples)


def _bass(out, bar_start, root, chord, next_root, beats):
    """Walking bass: root, third, fifth, then a chromatic approach note to the next chord."""
    approach = next_root * _semitones(-1 if next_root > root else 1)
    walk = [root, root * _semitones(chord[1]), root * _semitones(chord[2]), approach]
    for beat in beats:
        f = walk[beat]
        _add_at(out, bar_start + beat * BEAT, _note(f, f, BEAT * 0.95, "tri", 0.35, attack=0.004, decay=0.2))


def _theme(out, bar_start, root, chord, rng):
    """Chord notes two octaves up, with slight vibrato like a vibraphone."""
    for start, length in rng.choice(PHRASES):
        f = root * 4 * _semitones(rng.choice(chord))
        note = _note(f, f, length * BEAT, "sine", 0.16, attack=0.01, decay=length * BEAT * 0.6, vibrato=(5, 0.006))
        _add_at(out, bar_start + start * BEAT, note)


def _track(name):
    """Swing loop of BARS bars (bass, piano, brushes, vibraphone theme) run through the radio."""
    out = [0.0] * int(BARS * BEATS_PER_BAR * BEAT * RATE)
    style = STYLES[name]
    rng = random.Random(name)
    ride = noise(0.18, 0.1, 0.06, 0.95, 21)
    brush = noise(0.25, 0.12, 0.1, 0.4, 22)
    # Under the radio's high-pass only the kick's click gets through, so give it one.
    kick = mix((0, _note(180, 70, 0.15, "sine", 0.5, decay=0.06)), (0, noise(0.03, 0.4, 0.01, 0.5, 23)))
    progression = PROGRESSIONS[name]
    for bar, (root, chord) in enumerate(progression):
        bar_start = bar * BEATS_PER_BAR * BEAT
        _bass(out, bar_start, root, chord, progression[(bar + 1) % BARS][0], style["bass"])
        voicing = [root * 2 * _semitones(i) for i in chord]
        _play(out, bar_start, style["piano"], mix(*((0, _note(f, f, 0.3, "tri", 0.06, decay=0.1)) for f in voicing)))
        _play(out, bar_start, RIDE, ride)
        _play(out, bar_start, BRUSH, brush)
        _play(out, bar_start, style["kick"], kick)
        if bar % style["theme_every"] == 0:
            _theme(out, bar_start, root, chord, rng)
    return _normalize(_radio(out, name), MUSIC_PEAK)


def _build_music():
    return {name: _track(name) for name in STYLES}


CEIL = 0.95  # max output peak
RELEASE = 0.02  # limiter gain recovery per block of CHUNK samples


def _limiter(buf, gain, vol=1.0):
    """Gain limiter: if the block peak (× vol) exceeds CEIL, the gain ramps down until the first sample
    that is too loud, then recovers by RELEASE per block. The waveform is never distorted (per-sample
    clipping makes the music crackle as soon as a loud effect is added on top).
    Returns (samples, final gain); the gain is continuous across samples and blocks."""
    peak = max(map(abs, buf), default=0.0) * vol
    safe_gain = CEIL / peak if peak > CEIL else 1.0
    if safe_gain < gain:
        # Samples before the first loud one stay under CEIL with any gain ≤ gain: we have until it
        # to ramp down.
        first_loud = next(i for i, x in enumerate(buf) if abs(x) * vol * gain > CEIL)
        return [x * vol * (gain + (safe_gain - gain) * min(1.0, (i + 1) / (first_loud + 1)))
                for i, x in enumerate(buf)], safe_gain
    released = min(safe_gain, gain + RELEASE)
    if released == gain == 1.0:  # common case: nothing to limit, copy only if the volume is reduced
        return ([x * vol for x in buf] if vol != 1.0 else buf), released
    n = len(buf)
    return [x * vol * (gain + (released - gain) * (i + 1) / n) for i, x in enumerate(buf)], released


# --- PulseAudio output -------------------------------------------------------------

PA_SAMPLE_S16LE = 3
MONO = 1
PA_STREAM_PLAYBACK = 1
PA_DEFAULT = 0xFFFFFFFF  # (uint32_t) -1: value chosen by the server
BYTES_PER_SAMPLE = 2
LATENCY_CHUNKS = 4  # ~80 ms
PCM_SCALE = 32000  # slightly under 32767, the 16-bit integer peak
# Inaudible background hiss (±2 steps out of 32,000, ~-84 dB) added to the output: WSLg's audio output (Windows
# Remote Desktop) cuts a stream of pure silence and is slow to restart it, so effects come late and choppy
# whenever the music is off.
KEEPALIVE_LSB = 2
_rng = random.Random(0)
_KEEPALIVE = array("h", (_rng.randint(-KEEPALIVE_LSB, KEEPALIVE_LSB) for _ in range(RATE)))
del _rng


def _keep_alive(pcm, pos):
    """Adds the background hiss to the PCM block, starting at sample pos of its one-second loop."""
    samples = array("h", pcm)
    n = len(_KEEPALIVE)
    for i in range(len(samples)):
        samples[i] += _KEEPALIVE[(pos + i) % n]
    return samples.tobytes()


class _SampleSpec(ctypes.Structure):
    _fields_ = [("format", ctypes.c_int), ("rate", ctypes.c_uint32), ("channels", ctypes.c_uint8)]


class _BufferAttr(ctypes.Structure):
    _fields_ = [(n, ctypes.c_uint32) for n in ("maxlength", "tlength", "prebuf", "minreq", "fragsize")]


def _open_pulse():
    name = ctypes.util.find_library("pulse-simple") or "libpulse-simple.so.0"
    lib = ctypes.CDLL(name)
    lib.pa_simple_new.restype = ctypes.c_void_p
    lib.pa_simple_write.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_int)]
    spec = _SampleSpec(PA_SAMPLE_S16LE, RATE, MONO)
    target_length = CHUNK * BYTES_PER_SAMPLE * LATENCY_CHUNKS
    attr = _BufferAttr(PA_DEFAULT, target_length, PA_DEFAULT, PA_DEFAULT, PA_DEFAULT)
    err = ctypes.c_int()
    stream = lib.pa_simple_new(None, b"Champignon atomique", PA_STREAM_PLAYBACK, None, b"sfx", ctypes.byref(spec),
                               None, ctypes.byref(attr), ctypes.byref(err))
    if not stream:
        raise OSError(f"pa_simple_new: erreur {err.value}")
    return lib, stream


_POS, _PRIO, _FADING = 1, 3, 4


class Sound:
    def __init__(self, enabled=True):
        self.muted = False
        self.music_on = True
        self.volume = 1.0  # master volume (effects and music), 0..1
        self.ok = False
        self.voices = []  # [samples, position, volume, priority, fading out]
        self.tracks = {}
        self.music = self.wanted_music = None
        self.music_gain = 0.0
        self.music_pos = 0
        self.limit_gain = 1.0
        self.lock = threading.Lock()
        self.sounds = {}
        if enabled:
            threading.Thread(target=self._run, daemon=True).start()

    def play(self, name, vol=1.0):
        if self.muted or not self.ok or name not in self.sounds:
            return
        prio = PRIORITY.get(name, 1)
        with self.lock:
            if prio >= JINGLE:
                if any(v[_PRIO] >= prio for v in self.voices if not v[_FADING]):
                    return
                for v in self.voices:
                    if v[_PRIO] >= JINGLE:
                        v[_FADING] = True
            else:
                minor = [v for v in self.voices if v[_PRIO] < JINGLE]
                oldest_over_limit = minor[:max(0, len(minor) - MAX_MINOR + 1)]
                for v in oldest_over_limit:
                    self.voices.remove(v)
            self.voices.append([self.sounds[name], 0, vol, prio, False])
            del self.voices[:-MAX_VOICES]

    def set_music(self, name):
        self.wanted_music = name

    def _run(self):
        try:
            lib, stream = _open_pulse()
            self.sounds = _build()
        except (OSError, AttributeError):
            return
        self.ok = True
        err = ctypes.c_int()
        # Music (~2 s to compute) is generated alongside so it doesn't delay the effects.
        threading.Thread(target=lambda: setattr(self, "tracks", _build_music()), daemon=True).start()
        floor_pos = 0
        while True:
            pcm = _keep_alive(self._mix(), floor_pos)
            floor_pos = (floor_pos + CHUNK) % len(_KEEPALIVE)
            if lib.pa_simple_write(stream, pcm, len(pcm), ctypes.byref(err)) < 0:
                self.ok = False
                return

    def _mix(self):
        """Mixes the active voices over a block of CHUNK samples, returns 16-bit PCM."""
        buf = [0.0] * CHUNK
        with self.lock:
            if self.muted:
                self.voices.clear()
            alive = []
            jingle_playing = any(v[_PRIO] >= JINGLE and not v[_FADING] for v in self.voices)
            for v in self.voices:
                samples, pos, vol, prio, fading = v
                if prio < JINGLE and jingle_playing:
                    vol *= DUCK
                for i, x in enumerate(samples[pos:pos + CHUNK]):
                    buf[i] += x * vol * (1 - i / CHUNK if fading else 1)
                v[_POS] = pos + CHUNK
                if v[_POS] < len(samples) and not fading:
                    alive.append(v)
            self.voices = alive
        self._mix_music(buf)
        out, self.limit_gain = _limiter(buf, self.limit_gain, self.volume)
        return array("h", (int(x * PCM_SCALE) for x in out)).tobytes()

    def _mix_music(self, buf):
        """Adds the looping track, fading out then in when the wanted track changes."""
        if self.muted:
            self.music, self.music_gain = None, 0.0
        wanted = self.wanted_music if self.music_on and not self.muted else None
        if self.music != wanted:
            self.music_gain -= FADE_PER_CHUNK
            if self.music_gain <= 0 or self.music is None:
                self.music, self.music_gain = wanted, 0.0
        elif self.music in self.tracks:
            self.music_gain = min(1.0, self.music_gain + FADE_PER_CHUNK)
        track = self.tracks.get(self.music)
        if track and self.music_gain > 0:
            length, pos, gain = len(track), self.music_pos, self.music_gain * MUSIC_VOL
            for i in range(CHUNK):
                buf[i] += track[(pos + i) % length] * gain
        if track:
            self.music_pos = (self.music_pos + CHUNK) % len(track)
