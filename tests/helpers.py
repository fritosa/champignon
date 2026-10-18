import curses
import random

from game import Game


class FixedRandom(random.Random):
    """random() renvoie toujours la même valeur (0 → tous les tirages « réussissent »)."""

    def __init__(self, value):
        super().__init__(0)
        self.value = value

    def random(self):
        return self.value


def exploded_game(rng=None):
    g = Game(rng=rng or random.Random(0))
    g.click()
    g.sfx.clear()
    g.messages.clear()
    return g


class FakeSound:
    def __init__(self, ok=True):
        self.ok = ok
        self.muted = False
        self.music_on = True
        self.music = None
        self.played = []

    def set_music(self, name):
        self.music = name

    def play(self, name, vol=1.0):
        self.played.append(name)


class FakeScreen:
    def __init__(self, keys, size=(30, 100)):
        self.keys = list(keys)
        self.size = size
        self.text = []
        self.delay = []

    def getmaxyx(self):
        return self.size

    def erase(self):
        self.text = []

    def addnstr(self, y, x, s, n, attr=0):
        # Mêmes refus que curses : hors écran, n <= 0, ou écriture jusqu'au coin bas-droit.
        h, w = self.size
        s = s[:n]
        if n <= 0 or not (0 <= y < h and 0 <= x < w) or x + len(s) > w or (y == h - 1 and x + len(s) >= w):
            raise curses.error("addnstr() returned ERR")
        self.text.append(s)

    def refresh(self):
        pass

    def nodelay(self, flag):
        self.delay.append(flag)

    def getch(self):
        return self.keys.pop(0)
