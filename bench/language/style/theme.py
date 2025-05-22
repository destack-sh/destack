from typing import TYPE_CHECKING

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsBlockable,
    IsInstantiable,
    IsModal,
    IsNamed,
    NodeType,
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
class Theme(IsInstantiable, IsModal, IsNamed, IsBlockable[ThemeData]):
    """A Theme with common styles."""

    # colors
    colors: dict[ThemeColor, "Color"] = p_regular(50)

    # fonts
    # ...
