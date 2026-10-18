import os
import random
import re
import unittest
from array import array

import sound
import ui

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def used_sound_names():
    """Noms de sons émis dans le code (g.sfx.append("x") / self.sfx("x"))."""
    names = set()
    for name in ("game.py", "events.py", "ui.py"):
        with open(os.path.join(ROOT, name)) as f:
            names |= set(re.findall(r'sfx(?:\.append)?\("(\w+)"\)', f.read()))
    return names


class SynthTest(unittest.TestCase):
    def test_tone_length_and_envelope(self):
        s = sound.tone(440, 440, 0.1, "sine", 0.5)
        self.assertEqual(len(s), int(0.1 * sound.RATE))
        self.assertAlmostEqual(s[0], 0, places=3)
        self.assertLessEqual(max(map(abs, s)), 0.5)

    def test_oscillators_bounded(self):
        for kind in ("sine", "square", "tri", "saw"):
            values = [sound._osc(kind, p / 100) for p in range(100)]
            self.assertLessEqual(max(values), 1.0, kind)
            self.assertGreaterEqual(min(values), -1.0, kind)

    def test_noise_deterministic(self):
        self.assertEqual(sound.noise(0.05, seed=3), sound.noise(0.05, seed=3))
        self.assertNotEqual(sound.noise(0.05, seed=3), sound.noise(0.05, seed=4))

    def test_mix_and_seq(self):
        a, b = [1.0] * 10, [1.0] * 10
        m = sound.mix((0, a), (5 / sound.RATE, b))
        self.assertEqual(len(m), 15)
        self.assertEqual(m[7], 2.0)
        self.assertEqual(len(sound.seq(a, b)), 20)


class BankTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.bank = sound._build()

    def test_levels(self):
        for name, s in self.bank.items():
            target = sound.LEVEL.get(name.rstrip("0123456789"), 0.8)
            self.assertAlmostEqual(max(map(abs, s)), target, places=4, msg=name)

    def test_level_keys_exist(self):
        names = {name.rstrip("0123456789") for name in self.bank}
        self.assertLessEqual(set(sound.LEVEL), names)

    def test_limiter_transparent_below_ceiling(self):
        buf = [0.5, -0.9, 0.2]
        self.assertEqual(sound._limiter(buf, 1.0), (buf, 1.0))
        self.assertEqual(sound._limiter([], 1.0), ([], 1.0))

    def test_limiter_attack_ramps_before_peak(self):
        buf = [0.5] * 10 + [1.9] + [0.5] * 9
        out, gain = sound._limiter(buf, 1.0)
        self.assertAlmostEqual(gain, 0.5)
        self.assertAlmostEqual(max(map(abs, out)), sound.CEIL)
        gains = [o / x for o, x in zip(out, buf)]
        self.assertGreater(gains[0], 0.9)  # pas de chute brutale en début de bloc
        self.assertTrue(all(b <= a + 1e-12 for a, b in zip(gains, gains[1:])))
        self.assertTrue(all(abs(g - 0.5) < 1e-9 for g in gains[10:]))

    def test_limiter_never_exceeds_ceiling(self):
        rng = random.Random(0)
        gain = 1.0
        for _ in range(200):
            buf = [rng.uniform(-3, 3) * rng.random() ** 3 for _ in range(50)]
            vol = rng.choice((1.0, 0.5, 0.2))
            out, gain = sound._limiter(buf, gain, vol)
            self.assertLessEqual(max(map(abs, out)), sound.CEIL + 1e-9)

    def test_limiter_releases_gradually(self):
        gain, gains, last = 0.5, [], 0.0
        for _ in range(30):
            out, gain = sound._limiter([0.5] * 10, gain)
            gains.append(gain)
            self.assertTrue(all(b >= a for a, b in zip([last] + out, out)))  # rampe sans saut, même entre blocs
            self.assertAlmostEqual(out[-1], 0.5 * gain)  # le bloc finit au gain renvoyé
            last = out[-1]
        self.assertAlmostEqual(gains[0], 0.5 + sound.RELEASE)
        self.assertEqual(gains[-1], 1.0)

    def test_every_emitted_sound_exists(self):
        missing = used_sound_names() - set(self.bank)
        self.assertFalse(missing, f"sons émis mais jamais synthétisés : {missing}")
        for i in range(8):
            self.assertIn(f"click{i}", self.bank)

    def test_throttled_sounds_exist(self):
        self.assertLessEqual(set(ui.SFX_GAP), set(self.bank))


class MusicTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tracks = sound._build_music()

    def setUp(self):
        self.snd = sound.Sound(enabled=False)
        self.snd.ok = True
        self.snd.tracks = self.tracks

    def test_tracks_same_length_no_clipping(self):
        lengths = {len(t) for t in self.tracks.values()}
        self.assertEqual(len(lengths), 1)  # même longueur : on change de piste sans perdre le temps
        self.assertEqual(lengths.pop(), int(sound.BARS * 4 * sound.BEAT * sound.RATE))
        for name, t in self.tracks.items():
            self.assertLessEqual(max(map(abs, t)), 0.9 + 1e-6, name)

    def test_every_track_has_chords_and_style(self):
        self.assertEqual(set(sound.STYLES), set(sound.PROGRESSIONS))

    def run_chunks(self, n):
        return [array("h", self.snd._mix()) for _ in range(n)]

    def test_fade_in(self):
        self.snd.set_music("calm")
        self.snd._mix()
        self.assertLess(self.snd.music_gain, 0.1)
        self.run_chunks(60)
        self.assertEqual(self.snd.music_gain, 1.0)

    def test_fade_waits_for_tracks(self):
        self.snd.tracks = {}
        self.snd.set_music("calm")
        self.run_chunks(80)
        self.assertEqual(self.snd.music_gain, 0.0)
        self.snd.tracks = self.tracks
        self.snd._mix()
        self.assertLess(self.snd.music_gain, 0.1)

    def test_mute_cuts_music_at_once(self):
        self.snd.set_music("calm")
        self.run_chunks(60)
        self.snd.muted = True
        self.assertTrue(all(x == 0 for x in self.run_chunks(1)[0]))

    def test_crossfade(self):
        self.snd.set_music("calm")
        self.run_chunks(60)
        self.snd.set_music("epic")
        self.run_chunks(5)
        self.assertEqual(self.snd.music, "calm")  # fondu sortant d'abord
        self.run_chunks(60)
        self.assertEqual(self.snd.music, "epic")

    def test_loops(self):
        self.snd.set_music("calm")
        n = len(self.tracks["calm"]) // sound.CHUNK + 2
        self.run_chunks(n)
        self.assertLess(self.snd.music_pos, len(self.tracks["calm"]))
        self.assertGreater(max(map(abs, self.run_chunks(1)[0])), 0)

    def test_music_off_and_mute(self):
        self.snd.set_music("calm")
        self.run_chunks(60)
        self.snd.music_on = False
        self.run_chunks(60)
        self.assertIsNone(self.snd.music)
        self.assertTrue(all(x == 0 for x in self.run_chunks(1)[0]))
        self.snd.music_on, self.snd.muted = True, True
        self.run_chunks(60)
        self.assertIsNone(self.snd.music)


class MixerTest(unittest.TestCase):
    def setUp(self):
        self.snd = sound.Sound(enabled=False)
        self.snd.ok = True
        self.snd.sounds = {"half": array("f", [0.5] * (sound.CHUNK + 10)), "loud": array("f", [0.8] * 5)}

    def pcm(self):
        return array("h", self.snd._mix())

    def test_disabled_is_silent_noop(self):
        snd = sound.Sound(enabled=False)
        self.assertFalse(snd.ok)
        snd.play("boom")
        self.assertEqual(snd.voices, [])

    def test_voice_plays_then_ends(self):
        self.snd.play("half")
        first = self.pcm()
        self.assertEqual(len(first), sound.CHUNK)
        self.assertTrue(all(x == 16000 for x in first))
        second = self.pcm()
        self.assertEqual(list(second[:10]), [16000] * 10)
        self.assertEqual(second[10], 0)
        self.assertEqual(self.snd.voices, [])

    def test_volume_and_clamp(self):
        self.snd.play("loud", vol=0.5)
        self.assertEqual(self.pcm()[0], 12800)
        self.snd.play("loud")
        self.snd.play("loud")
        self.assertAlmostEqual(self.pcm()[0], sound.CEIL * 32000, delta=2)  # limiteur : plafonné à CEIL

    def test_master_volume(self):
        self.snd.volume = 0.5
        self.snd.play("half")
        self.assertEqual(self.pcm()[0], 8000)

    def test_unknown_sound_ignored(self):
        self.snd.play("inconnu")
        self.assertEqual(self.snd.voices, [])

    def test_mute(self):
        self.snd.play("half")
        self.snd.muted = True
        self.snd.play("half")
        self.assertEqual(len(self.snd.voices), 1)  # pas de nouveau son
        self.assertTrue(all(x == 0 for x in self.pcm()))  # et ceux en cours sont coupés
        self.assertEqual(self.snd.voices, [])

    def test_minor_voice_limit(self):
        for _ in range(sound.MAX_MINOR + 5):
            self.snd.play("half")
        self.assertEqual(len(self.snd.voices), sound.MAX_MINOR)


class PriorityTest(unittest.TestCase):
    def setUp(self):
        self.snd = sound.Sound(enabled=False)
        self.snd.ok = True
        long = array("f", [0.5] * (sound.CHUNK * 10))
        self.snd.sounds = {k: long for k in ("level", "achievement", "boom", "click0")}

    def test_one_jingle_at_a_time(self):
        self.snd.play("achievement")
        self.snd.play("achievement")
        self.snd.play("level")  # moins important : ignoré
        self.assertEqual(len(self.snd.voices), 1)

    def test_more_important_jingle_cuts_previous(self):
        self.snd.play("level")
        self.snd.play("boom")
        self.assertEqual([v[4] for v in self.snd.voices], [True, False])  # level en fondu de sortie
        self.snd._mix()
        self.assertEqual(len(self.snd.voices), 1)
        self.assertEqual(self.snd.voices[0][3], sound.PRIORITY["boom"])
        self.snd.play("level")  # le boom joue encore
        self.assertEqual(len(self.snd.voices), 1)

    def test_minor_sounds_ducked_during_jingle(self):
        self.snd.play("click0")
        alone = array("h", self.snd._mix())[0]
        self.snd.play("level")
        self.snd.voices = [v for v in self.snd.voices if v[3] < 2] + [[array("f", [0.0] * sound.CHUNK * 5), 0, 1, 2, False]]
        self.assertEqual(array("h", self.snd._mix())[0], alone * sound.DUCK)

    def test_minor_sounds_coexist_with_jingle(self):
        self.snd.play("level")
        self.snd.play("click0")
        self.assertEqual(len(self.snd.voices), 2)


if __name__ == "__main__":
    unittest.main()
