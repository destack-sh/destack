from typing import TYPE_CHECKING

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

from .scaffold import *  # noqa: F403

# ruff: noqa: F405

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


# ===============================================
# tetris/Game [Entity]
# ===============================================


@entity
class TetrisGame(IsStarable, Record):
    pass


# ===============================================
