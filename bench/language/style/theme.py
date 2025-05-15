from typing import TYPE_CHECKING, Optional

from bench.language.core import NodeType, PageNode, StructType, node_, p_regular
from bench.pb2 import ThemeData

if TYPE_CHECKING:
    from .color import Color


@node_(NodeType.THEME)
class Theme(PageNode[ThemeData]):
    """A Theme with common styles."""

    # colors
    primary_color: Optional["Color"] = p_regular(
        50, require=False, array=False, struct=StructType.COLOR
    )
    secondary_color: Optional["Color"] = p_regular(
        51, require=False, array=False, struct=StructType.COLOR
    )
    accent_color: Optional["Color"] = p_regular(
        52, require=False, array=False, struct=StructType.COLOR
    )
    muted_color: Optional["Color"] = p_regular(
        53, require=False, array=False, struct=StructType.COLOR
    )
    success_color: Optional["Color"] = p_regular(
        54, require=False, array=False, struct=StructType.COLOR
    )
    warning_color: Optional["Color"] = p_regular(
        55, require=False, array=False, struct=StructType.COLOR
    )
    error_color: Optional["Color"] = p_regular(
        56, require=False, array=False, struct=StructType.COLOR
    )

    # fonts
    # ...
