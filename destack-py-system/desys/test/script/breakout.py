from typing import TYPE_CHECKING

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

from .scaffold import *  # noqa: F403

# ruff: noqa: F405

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

# ===============================================
# breakout/Common [Entity]
# ===============================================


@entity
class BreakoutService(Service):
    pass


# ===============================================
# breakout/Game [Entity]
# ===============================================


@entity
class BreakoutGame(IsStarable, Record):
    pass


# ===============================================
# breakout/Paddle [Entity]
# ===============================================


@entity
class BreakoutPaddle(Record):
    pass


# ===============================================
# breakout/Brick [Entity]
# ===============================================


@entity
class BreakoutBrick(Record):
    pass


# ===============================================
# breakout/Ball [Entity]
# ===============================================


@entity
class BreakoutBall(Record):
    pass


# ===============================================
# breakout/Wall [Entity]
# ===============================================


@entity
class BreakoutWall(Record):
    pass
