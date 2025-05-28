from typing import TYPE_CHECKING

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    StructMutable,
    StructType,
    enum_,
    property_,
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
class Fill(StructMutable):
    type: FillType = property_(30, is_repr=True)
    color: Color | None = property_(40, is_repr=True)
    gradient: Gradient | None = property_(41, is_repr=True)
    image: "File | None" = property_(50, is_repr=True)
    position: FillPosition | None = property_(60, is_repr=True)
    size: FillSize | None = property_(70, is_repr=True)
