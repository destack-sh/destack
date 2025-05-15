from bench.language.core import BuiltinEnum, EnumType, Struct, StructType, enum_, p_regular, struct_


@enum_(EnumType.POSITION_TYPE)
class PositionType(BuiltinEnum):
    """The position of a View."""

    RELATIVE = 1
    ABSOLUTE = 2
    STICKY = 3


@struct_(StructType.POSITION)
class Position(Struct):
    """The position of a View."""

    type: PositionType = p_regular(40)
    x: float = p_regular(41)
    y: float = p_regular(42)
