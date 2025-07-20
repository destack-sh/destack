from typing import TYPE_CHECKING

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

from destack import *  # noqa: F403

from .scaffold import *  # noqa: F403

# ruff: noqa: F405

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


# ===============================================
# breakout/Game [Entity]
# ===============================================


@entity
class BreakoutGame(IsStarable, Record):
    pass


# ===============================================
# breakout/Player [Entity]
# ===============================================


@entity
class BreakoutPlayer(Entity2D):
    pass


# ===============================================
# breakout/Paddle [Entity]
# ===============================================


@entity
class BreakoutPaddle(Entity2D):
    pass


# ===============================================
# breakout/Brick [Entity]
# ===============================================


@entity
class BreakoutBrick(Entity2D):
    pass


# ===============================================
# breakout/Ball [Entity]
# ===============================================


@entity
class BreakoutBall(Entity2D):
    pass


# ===============================================
# breakout/Wall [Entity]
# ===============================================


@entity
class BreakoutWall(Entity2D):
    pass
