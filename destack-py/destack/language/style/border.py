from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Enum,
    EnumType,
    Insets,
    NodeType,
    StructFrozen,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_struct,
)

from .color import Color
from .style import Style

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.BORDER_TYPE)
class BorderType(Enum):
    """Built-in border types."""

    STYLE = 2
    SOLID = 10
    DASHED = 11
    DOTTED = 12
    DOUBLE = 13


@builtin_struct(StructType.BORDER, frozen=True)
class Border(StructFrozen):
    """A border value."""

    type: BorderType = builtin_property(100, default=BorderType.SOLID, is_repr=True)
    color: Optional["Color"] = builtin_property(101, is_repr=True)
    width: Optional[Insets] = builtin_property(102, is_repr=True)
    style: Optional["BorderStyle"] = builtin_property(103, is_repr=True)


@builtin_node(NodeType.BORDER_STYLE)
class BorderStyle(Style):
    """A border style."""

    type: BorderType = builtin_property(100, default=BorderType.SOLID, is_repr=True)
    color: Optional["Color"] = builtin_property(200, is_repr=True)
    width: Optional[Insets] = builtin_property(201, is_repr=True)
    style: Optional["BorderStyle"] = builtin_property(202, is_repr=True)
