from typing import TYPE_CHECKING

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    Struct,
    StructType,
    enum_,
    p_regular,
    struct_,
)

from .color import Color
from .gradient import Gradient

if TYPE_CHECKING:
    from bench.language import File


@enum_(EnumType.FILL_TYPE)
class FillType(BuiltinEnum):
    SOLID = 10
    GRADIENT = 11
    IMAGE = 12


@enum_(EnumType.FILL_POSITION)
class FillPosition(BuiltinEnum):
    TOP_LEFT = 1
    TOP_CENTER = 2
    TOP_RIGHT = 3
    LEFT = 10
    CENTER = 11
    RIGHT = 12
    BOTTOM_LEFT = 20
    BOTTOM_CENTER = 21
    BOTTOM_RIGHT = 22


@enum_(EnumType.FILL_SIZE)
class FillSize(BuiltinEnum):
    FILL = 1
    STRETCH = 2
    FIT = 3
    TILE = 4


@struct_(StructType.FILL)
class Fill(Struct):
    type: FillType = p_regular(30)
    color: Color | None = p_regular(40)
    gradient: Gradient | None = p_regular(41)
    image: "File | None" = p_regular(50)
    position: FillPosition | None = p_regular(60)
    size: FillSize | None = p_regular(70)
