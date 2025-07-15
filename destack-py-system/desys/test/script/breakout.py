from typing import TYPE_CHECKING

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

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
# breakout/Game [Entity]
# ===============================================


@entity
class BreakoutGameObject(Record):
    parent: BreakoutGame


# ===============================================
# breakout/Player [Entity]
# ===============================================


@entity
class BreakoutPlayer(BreakoutGameObject):
    pass


# ===============================================
# breakout/Paddle [Entity]
# ===============================================


@entity
class BreakoutPaddle(BreakoutGameObject):
    pass


# ===============================================
# breakout/Brick [Entity]
# ===============================================


@entity
class BreakoutBrick(BreakoutGameObject):
    pass


# ===============================================
# breakout/Ball [Entity]
# ===============================================


@entity
class BreakoutBall(BreakoutGameObject):
    pass


# ===============================================
# breakout/Wall [Entity]
# ===============================================


@entity
class BreakoutWall(BreakoutGameObject):
    pass
