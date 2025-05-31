from typing import TYPE_CHECKING, Any, Optional, Union

from .const import BuiltinEnum, EnumType, StructType, enum_
from .property import property_
from .struct import StructFrozen, struct_

if TYPE_CHECKING:
    from bench.language import Field, Node


@enum_(EnumType.VARIABLE_TYPE)
class VariableType(BuiltinEnum):
    """The type of a variable."""

    FIELD = 10


@struct_(StructType.VARIABLE, frozen=True)
class Variable[T: Any](StructFrozen):
    """A variable value / reference (to be resolved at runtime)."""

    field: Optional["Field"] = property_(40)
    node: Optional["Node"] = property_(41)

    def read(self) -> T | None:
        """Read the value of the variable."""
        raise NotImplementedError


type VariableProperty[T] = Union[T, Variable[T], None]
