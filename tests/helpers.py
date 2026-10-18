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
