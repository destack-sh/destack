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
# tetris/Game
# ===============================================


@entity
class TetrisGame(Entity):
    pass


# ===============================================
# tetris/Board
# ===============================================


@entity
class TetrisBoard(Entity):
    pass


# ===============================================
# tetris/Piece
# ===============================================


@enum
class TetrisShape(Enum):
    I = 1  # noqa: E741
    J = 2
    L = 3
    O = 4  # noqa: E741
    S = 5
    T = 6
    Z = 7


@entity
class TetrisPiece(KinematicBody2D):
    shape: TetrisShape
    rotation: Int8
    x: Int8
    y: Int8


# ===============================================
# tetris/Cell
# ===============================================


@entity
class TetrisCell(StaticBody2D):
    occupied: bool
    piece: TetrisPiece | None
