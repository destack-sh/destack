from typing import TYPE_CHECKING

import pytest

if not TYPE_CHECKING:
    pytest.skip(allow_module_level=True)

from destack import *  # noqa: F403

from .scaffold import *  # noqa: F403

# ruff: noqa: F405, E741

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


# ===============================================
# tictactoe/Game [Entity]
# ===============================================


@entity
class TicTacToeGame(Entity):
    pass


# ===============================================
# TicTacToePlayer [Entity]
# ===============================================


@entity
class TicTacToeBoard(Entity):
    size: tuple[int, int]

    @method
    def reset(self):
        pass


# ===============================================
# TicTacToeCell [Entity]
# ===============================================


@enum
class TicTacToeCellState(Enum):
    EMPTY = 1
    X = 2
    O = 3


@entity
class TicTacToeCell(Entity):
    parent: TicTacToeBoard
    position: tuple[int, int]
    state: TicTacToeCellState
