from bench.language.core import BuiltinEnum, EnumType, Struct, StructType, enum_, p_regular, struct_


@enum_(EnumType.LENGTH_UNIT)
class LengthUnit(BuiltinEnum):
    PIXEL = 1
    REM = 2


@struct_(StructType.LENGTH)
class Length(Struct):
    unit: LengthUnit = p_regular(31)
    value: float = p_regular(32)
