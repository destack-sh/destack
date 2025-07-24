from typing import TYPE_CHECKING

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

from destack import *  # noqa: F403

from .scaffold import *  # noqa: F403

# ruff: noqa: F405

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


# ===============================================
# tictactoe/Game [Entity]
# ===============================================


@entity
class TicTacToeGame(IsStarable, Record):
    pass


# ===============================================
# TicTacToePlayer [Entity]
# ===============================================


@entity
class TicTacToeBoard(Record):
    pass


# ===============================================
# TicTacToeCell [Entity]
# ===============================================


@entity
class TicTacToeCell(IsOwnable, Record):
    parent: TicTacToeBoard
    owned_by: IsActor
