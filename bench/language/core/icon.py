from typing import TYPE_CHECKING, Optional, Union

from .color import ColorIn, to_color
from .const import BuiltinEnum, EnumType, NodeType, StructType, enum_
from .property import p_regular
from .struct import Struct, struct_

if TYPE_CHECKING:
    from bench.language import Color, File

# pyright: reportIncompatibleVariableOverride=false


#
# Icon
#


@enum_(EnumType.ICON_TYPE)
class IconType(BuiltinEnum):
    EMOJI = 1
    FONT_AWESOME = 3
    VS_CODE = 4
    FILE = 10
    FILE_URL = 11


@struct_(StructType.ICON)
class Icon(Struct):
    """An icon to be displayed in some view."""

    type: IconType = p_regular(30, default=False)
    # content
    emoji: str | None = p_regular(31, require=False)
    fa_name: str | None = p_regular(33, require=False)
    vsc_name: str | None = p_regular(34, require=False)
    file: Optional["File"] = p_regular(35, require=False, references=NodeType.FILE)
    file_url: str | None = p_regular(36, require=False)
    # style
    color: Optional["Color"] = p_regular(40, require=False, array=False, struct=StructType.COLOR)

    @staticmethod
    def new(icon: "IconIn", color: ColorIn | None = None) -> "Icon":
        return to_icon(icon, color)


IconIn = Union[Icon, "File", str]


def to_icon(icon: IconIn, color: ColorIn | None = None) -> Icon:
    """Turn something that could be an Icon into an Icon."""
    from bench.language import File

    color = to_color(color) if color else None
    if isinstance(icon, str):
        if (
            icon.endswith(".svg")
            or icon.endswith(".png")
            or icon.endswith(".jpg")
            or icon.endswith(".jpeg")
            or icon.endswith(".ico")
        ):
            return Icon(type=IconType.FILE_URL, file_url=icon, color=color)
        elif icon.startswith("fa"):
            return Icon(type=IconType.FONT_AWESOME, fa_name=icon, color=color)
        else:
            return Icon(type=IconType.EMOJI, emoji=icon, color=color)
    elif isinstance(icon, File):
        return Icon(type=IconType.FILE, file=icon, color=color)
    else:
        return icon


def reverse_icon(icon: Icon) -> IconIn:
    """Turn an Icon back into something simpler that can be turned back into an Icon."""
    if icon.type == IconType.FONT_AWESOME:
        assert icon.fa_name, f"no fa_name for {icon!r}"
        return icon.fa_name
    elif icon.type == IconType.FILE:
        assert icon.file, f"no file for {icon!r}"
        return icon.file
    elif icon.type == IconType.EMOJI:
        assert icon.emoji, f"no emoji for {icon!r}"
        return icon.emoji
    else:
        return icon


icon = to_icon
