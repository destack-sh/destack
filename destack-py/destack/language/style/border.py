from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    BuiltinEnum,
    BuiltinObjectMutable,
    EnumType,
    Insets,
    Node,
    NodeType,
    StructMutable,
    StructType,
    enum_,
    node_,
    object_,
    property_,
    struct_,
)
from destack.pb2 import BorderStyleData

from .color import Color
from .style import Style

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.BORDER_TYPE)
class BorderType(BuiltinEnum):
    """Built-in border types."""

    NONE = 1
    STYLE = 2
    FIELD = 3
    SOLID = 10
    DASHED = 11
    DOTTED = 12
    DOUBLE = 13


@object_()
class BorderBase(BuiltinObjectMutable):
    type: BorderType = property_(30, default=BorderType.SOLID, is_repr=True)
    style: Optional["BorderStyle"] = property_(41, is_repr=True)
    color: Optional["Color"] = property_(50, is_repr=True)
    width: Optional[Insets] = property_(51, is_repr=True)


@struct_(
    StructType.BORDER,
)
class Border(BorderBase, StructMutable):
    """A border value."""

    pass


@node_(NodeType.BORDER_STYLE)
class BorderStyle(
    Style,
    BorderBase,
    Node[BorderStyleData],
):
    """A border style."""

    pass
