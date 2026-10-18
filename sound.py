"""Effets sonores synthétisés à la volée, joués via PulseAudio (libpulse-simple, sans dépendance Python).

Les sons sont générés une fois au démarrage (stdlib seulement) puis mixés dans un thread qui écrit
des petits blocs dans un flux PulseAudio. Sans PulseAudio, tout est silencieusement désactivé.
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
MAX_MINOR = 4  # bruitages courts (appuis, tics...) joués en même temps au maximum
DUCK = 0.5  # volume des bruitages pendant un jingle
# Priorité des sons. À partir de JINGLE, ce sont des jingles exclusifs : un seul à la fois, un jingle
# plus important coupe le précédent, un jingle moins (ou aussi) important est ignoré.
# Les autres (1 par défaut) sont des bruitages, limités à MAX_MINOR voix.
JINGLE = 2
PRIORITY = dict(supernova=6, prestige=6, boom=5, perfect=5, fanfare=4, synergy=4, achievement=4,
                mutation=3, alarm=3, freeze=3, chased=3, milestone=3, chest=3, enemy=3,
                talent=2, drop=2, partial=2, fizzle=2, level=2, buy=2)
MUSIC_VOL = 0.25
# Crête visée par effet après normalisation (DEFAULT_LEVEL sinon) : les bruitages fréquents restent plus discrets.
LEVEL = dict(tick=0.3, click=0.45, nope=0.45, plane=0.5, charge=0.5, alarm=0.6)
DEFAULT_LEVEL = 0.8
MUSIC_PEAK = 0.9
MUSIC_FADE_SECONDS = 1.0
FADE_PER_CHUNK = CHUNK / RATE / MUSIC_FADE_SECONDS


# --- Synthèse ------------------------------------------------------------------------

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
    """Note avec glissando exponentiel f0 → f1, enveloppe attaque + décroissance."""
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
    """Bruit blanc filtré passe-bas (un pôle) avec décroissance exponentielle."""
    rng = random.Random(seed)
    out, filtered = [], 0.0
    for i in range(int(dur * RATE)):
        filtered += cutoff * (rng.uniform(-1, 1) - filtered)
        out.append(filtered * vol * math.exp(-i / RATE / decay))
    return out


def mix(*parts):
    """Superpose des (décalage en s, échantillons)."""
    out = [0.0] * max(int(offset * RATE) + len(samples) for offset, samples in parts)
    for offset, samples in parts:
        start = int(offset * RATE)
        for i, x in enumerate(samples):
            out[start + i] += x
    return out


def seq(*notes, gap=0.0):
    """Enchaîne des sons les uns après les autres."""
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
    # Ramène chaque son à sa crête visée : les couches superposées ne saturent pas, les sons ténus s'entendent.
    return {name: _normalize(samples, LEVEL.get(name.rstrip("0123456789"), DEFAULT_LEVEL))
            for name, samples in s.items()}


# --- Musique ---------------------------------------------------------------------------

BEAT = 0.5  # 120 bpm
BEATS_PER_BAR = 4
BARS = 8
FIFTH = 7  # demi-tons
A2, AS2, C3, D3, E2, F2, G2 = 110.0, 116.5, 130.8, 146.8, 82.4, 87.3, 98.0
# Accords (fondamentale, tierce en demi-tons), un par mesure.
PROGRESSIONS = {
    "calm": [(A2, 3), (A2, 3), (F2, 4), (F2, 4), (C3, 4), (C3, 4), (G2, 4), (G2, 4)],
    "tension": [(A2, 3), (AS2, 4), (A2, 3), (E2, 4), (A2, 3), (AS2, 4), (D3, 3), (E2, 4)],
    "epic": [(A2, 3), (F2, 4), (C3, 4), (G2, 4), (A2, 3), (F2, 4), (C3, 4), (E2, 4)],
}


def _add_at(out, t, samples):
    """Ajoute les échantillons à partir de t secondes, coupés à la fin de out."""
    start = int(t * RATE)
    for i, x in enumerate(samples[:len(out) - start]):
        out[start + i] += x


def _semitones(n):
    return 2 ** (n / 12)


def _track(name):
    """Boucle de BARS mesures : basse, arpège et (sauf calme) batterie de bruit."""
    out = [0.0] * int(BARS * BEATS_PER_BAR * BEAT * RATE)
    calm, epic = name == "calm", name == "epic"
    kick = tone(120, 40, 0.12, "sine", 0.6, decay=0.05)
    hat = noise(0.04, 0.12, 0.01, 0.9, 11)
    snare = noise(0.15, 0.3, 0.05, 0.5, 12)
    bass_per_beat = 2 if epic else 1
    bass_len = BEAT / bass_per_beat
    bass_wave = "square" if epic else "tri"
    arp_len = BEAT / (2 if calm else 4)
    arp_wave, arp_vol = ("tri", 0.1) if calm else ("square", 0.07)
    for bar, (root, third) in enumerate(PROGRESSIONS[name]):
        bar_start = bar * BEATS_PER_BAR * BEAT
        low = root * 2
        chord = [low, low * _semitones(third), low * _semitones(FIFTH), root * 4]
        bass = tone(root, root, bass_len * 0.9, bass_wave, 0.25, decay=bass_len * 0.6)
        for b in range(BEATS_PER_BAR * bass_per_beat):
            _add_at(out, bar_start + b * bass_len, bass)
        for i in range(int(BEATS_PER_BAR * BEAT / arp_len)):
            going_up = i % 8 < 4
            note = chord[i % 4] if going_up else chord[3 - i % 4]
            arp = tone(note * 2, note * 2, arp_len * 0.8, arp_wave, arp_vol, decay=arp_len * 0.5)
            _add_at(out, bar_start + i * arp_len, arp)
        if not calm:
            for b in range(BEATS_PER_BAR):
                _add_at(out, bar_start + b * BEAT, kick)
                _add_at(out, bar_start + (b + 0.5) * BEAT, hat)
                if epic and b % 2:
                    _add_at(out, bar_start + b * BEAT, snare)
    return _normalize(out, MUSIC_PEAK)


def _build_music():
    return {name: _track(name) for name in PROGRESSIONS}


CEIL = 0.95  # crête maximale en sortie
RELEASE = 0.02  # remontée du gain du limiteur par bloc de CHUNK échantillons


def _limiter(buf, gain, vol=1.0):
    """Limiteur à gain : si la crête du bloc (× vol) dépasse CEIL, le gain descend en rampe jusqu'au premier
    échantillon trop fort, puis remonte de RELEASE par bloc. L'onde n'est jamais déformée (une saturation
    échantillon par échantillon fait craquer la musique dès qu'un effet fort s'y ajoute).
    Renvoie (échantillons, gain final) ; le gain est continu d'un échantillon et d'un bloc à l'autre."""
    peak = max(map(abs, buf), default=0.0) * vol
    safe_gain = CEIL / peak if peak > CEIL else 1.0
    if safe_gain < gain:
        # Les échantillons avant le premier trop fort restent sous CEIL avec un gain ≤ gain : on a jusqu'à lui
        # pour descendre.
        first_loud = next(i for i, x in enumerate(buf) if abs(x) * vol * gain > CEIL)
        return [x * vol * (gain + (safe_gain - gain) * min(1.0, (i + 1) / (first_loud + 1)))
                for i, x in enumerate(buf)], safe_gain
    released = min(safe_gain, gain + RELEASE)
    if released == gain == 1.0:  # cas courant : rien à limiter, recopie seulement si le volume est réduit
        return ([x * vol for x in buf] if vol != 1.0 else buf), released
    n = len(buf)
    return [x * vol * (gain + (released - gain) * (i + 1) / n) for i, x in enumerate(buf)], released


# --- Sortie PulseAudio -------------------------------------------------------------

PA_SAMPLE_S16LE = 3
MONO = 1
PA_STREAM_PLAYBACK = 1
PA_DEFAULT = 0xFFFFFFFF  # (uint32_t) -1 : valeur choisie par le serveur
BYTES_PER_SAMPLE = 2
LATENCY_CHUNKS = 4  # ~80 ms
PCM_SCALE = 32000  # un peu sous 32767, la crête des entiers 16 bits


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
        self.volume = 1.0  # volume général (effets et musique), 0..1
        self.ok = False
        self.voices = []  # [échantillons, position, volume, priorité, en fondu de sortie]
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
        # La musique (~1 s de calcul) est générée à côté pour ne pas retarder les effets.
        threading.Thread(target=lambda: setattr(self, "tracks", _build_music()), daemon=True).start()
        while True:
            pcm = self._mix()
            if lib.pa_simple_write(stream, pcm, len(pcm), ctypes.byref(err)) < 0:
                self.ok = False
                return

    def _mix(self):
        """Mélange les voix actives sur un bloc de CHUNK échantillons, renvoie le PCM 16 bits."""
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
        """Ajoute la piste en boucle, avec fondu sortant puis entrant quand la piste voulue change."""
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
