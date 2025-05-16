from typing import TYPE_CHECKING, Optional

from bench.language.core import BuiltinEnum, EnumType, Struct, StructType, enum_, p_regular, struct_

if TYPE_CHECKING:
    from bench.language import Field


@enum_(EnumType.VARIABLE_TYPE)
class VariableType(BuiltinEnum):
    """The type of a variable."""

    FIELD = 10


@struct_(StructType.VARIABLE)
class Variable(Struct):
    """A variable value / reference (to be resolved at runtime)."""

    field: Optional["Field"] = p_regular(10)
