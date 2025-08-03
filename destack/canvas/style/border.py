from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    EnumDeclaration,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    declare_entity,
    declare_enum,
    declare_property,
    declare_struct,
)

from .color import Color
from .style import Style

if TYPE_CHECKING:
    from destack import Inset2


# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.BORDER_TYPE)
class BorderType(EnumDeclaration):
    """Built-in border types."""

    STYLE = 2
    SOLID = 10
    DASHED = 11
    DOTTED = 12
    DOUBLE = 13


@declare_struct(
    StructType.BORDER,
    frozen=True,
    is_final=True,
)
@final
class Border(StructFrozen):
    """A border value."""

    type: BorderType = declare_property(100, default=BorderType.SOLID, is_repr=True)
    color: Optional["Color"] = declare_property(101, is_repr=True)
    width: Optional["Inset2"] = declare_property(102, is_repr=True)
    style: Optional["BorderStyle"] = declare_property(103, is_repr=True)


@declare_entity(NodeType.BORDER_STYLE)
class BorderStyle(Style):
    """A border style."""

    type: BorderType = declare_property(100, default=BorderType.SOLID, is_repr=True)
    color: Optional["Color"] = declare_property(200, is_repr=True)
    width: Optional["Inset2"] = declare_property(201, is_repr=True)
    style: Optional["BorderStyle"] = declare_property(202, is_repr=True)
