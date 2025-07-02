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

    type: BorderType = builtin_property(30, default=BorderType.SOLID, is_repr=True)
    color: Optional["Color"] = builtin_property(50, is_repr=True)
    width: Optional[Insets] = builtin_property(51, is_repr=True)
    style: Optional["BorderStyle"] = builtin_property(41, is_repr=True)


@builtin_node(NodeType.BORDER_STYLE)
class BorderStyle(Style):
    """A border style."""

    type: BorderType = builtin_property(30, default=BorderType.SOLID, is_repr=True)
    color: Optional["Color"] = builtin_property(50, is_repr=True)
    width: Optional[Insets] = builtin_property(51, is_repr=True)
    style: Optional["BorderStyle"] = builtin_property(41, is_repr=True)
