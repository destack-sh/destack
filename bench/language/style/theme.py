from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsInstantiable,
    IsModal,
    IsNamed,
    NodeType,
    PageNode,
    enum_,
    node_,
    p_regular,
)
from bench.pb2 import ThemeData

if TYPE_CHECKING:
    from .color import Color

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.THEME_COLOR)
class ThemeColor(BuiltinEnum):
    """A color in the theme."""

    PRIMARY = 50
    SECONDARY = 51
    ACCENT = 52
    MUTED = 53
    SUCCESS = 54
    WARNING = 55
    ERROR = 56


@node_(NodeType.THEME)
class Theme(IsInstantiable, IsModal, IsNamed, PageNode[ThemeData]):
    """A Theme with common styles."""

    # colors
    primary_color: Optional["Color"] = p_regular(50)
    secondary_color: Optional["Color"] = p_regular(51)
    accent_color: Optional["Color"] = p_regular(52)
    muted_color: Optional["Color"] = p_regular(53)
    success_color: Optional["Color"] = p_regular(54)
    warning_color: Optional["Color"] = p_regular(55)
    error_color: Optional["Color"] = p_regular(56)

    # fonts
    # ...
