from typing import TYPE_CHECKING, Any, Optional

from ..builtin import (
    Enum,
    EnumType,
    StructFrozen,
    StructType,
    builtin_enum,
    builtin_struct,
    property_,
)

if TYPE_CHECKING:
    from destack.language import Field, Node


@builtin_enum(EnumType.VARIABLE_TYPE)
class VariableType(Enum):
    """The type of a variable."""

    FIELD = 10


@builtin_struct(StructType.VARIABLE, frozen=True)
class Variable[T: Any](StructFrozen):
    """A variable value / reference (to be resolved at runtime)."""

    field: Optional["Field"] = property_(40)
    node: Optional["Node"] = property_(41)

    def read(self) -> T | None:
        """Read the value of the variable."""
        raise NotImplementedError
