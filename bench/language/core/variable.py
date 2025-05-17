from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.core import BuiltinEnum, EnumType, Struct, StructType, enum_, p_regular, struct_

if TYPE_CHECKING:
    from bench.language import Field


@enum_(EnumType.VARIABLE_TYPE)
class VariableType(BuiltinEnum):
    """The type of a variable."""

    FIELD = 10


@struct_(StructType.VARIABLE)
class Variable[T: Any](Struct):
    """A variable value / reference (to be resolved at runtime)."""

    field: Optional["Field"] = p_regular(10)

    def read(self) -> T | None:
        """Read the value of the variable."""
        raise NotImplementedError


type MaybeVariable[T] = Union[T, Variable[T], None]
