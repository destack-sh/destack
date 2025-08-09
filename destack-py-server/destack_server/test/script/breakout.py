# type: ignore

from typing import TYPE_CHECKING

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

from destack import *  # noqa: F403

from .scaffold import *  # noqa: F403

# ruff: noqa: F405

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


# ===============================================
# breakout/Game
# ===============================================


@entity
class BreakoutGame(Entity):
    def tick(self):
        raise NotImplementedError


# ===============================================
# breakout/Player
# ===============================================


@entity
class BreakoutPlayer(Entity2D):
    pass


# ===============================================
# breakout/Paddle
# ===============================================


@entity
class BreakoutPaddle(Entity2D):
    pass


# ===============================================
# breakout/Brick
# ===============================================


@entity
class BreakoutBrick(Entity2D):
    width: Int8


# ===============================================
# breakout/Ball
# ===============================================


@entity
class BreakoutBall(Entity2D):
    radius: Int8


# ===============================================
# breakout/Wall
# ===============================================


@entity
class BreakoutWall(Entity2D):
    width: Int8
    height: Int8

    # computed effect?:
    # BreakoutWall:
    #  -> <RectangleCollider2D :width=width :height=height>
