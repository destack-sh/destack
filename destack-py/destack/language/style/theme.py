from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    HasIcon,
    HasName,
    IsDeletable,
    IsOrdered,
    IsTaggable,
    IsVisual,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.pb2 import ThemeData

if TYPE_CHECKING:
    from .color import Color

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.THEME_COLOR)
class ThemeColor(Enum):
    """A color in the theme."""

    PRIMARY = 50
    SECONDARY = 51
    ACCENT = 52
    MUTED = 53
    SUCCESS = 54
    WARNING = 55
    ERROR = 56


@builtin_node(NodeType.THEME)
class Theme(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsVisual,
    IsOrdered,
    IsTaggable,
    IsDeletable,
    Node[ThemeData],
):
    """A Theme with common styles."""

    # colors
    colors: dict[ThemeColor, "Color"] = property_(50, is_repr=True)

    # fonts
    # ...
