from typing import TYPE_CHECKING

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    HasIcon,
    HasName,
    IsArchivable,
    IsDeletable,
    IsTaggable,
    IsTracked,
    IsVisual,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
)
from destack.pb2 import ThemeData

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
class Theme(
    HasName,
    HasIcon,
    IsVisual,
    IsTaggable,
    IsDeletable,
    IsArchivable,
    IsTracked,
    Node[ThemeData],
):
    """A Theme with common styles."""

    # colors
    colors: dict[ThemeColor, "Color"] = property_(50, is_repr=True)

    # fonts
    # ...
