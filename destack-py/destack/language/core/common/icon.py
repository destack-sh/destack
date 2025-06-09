from typing import TYPE_CHECKING, Optional, Union

from ..builtin import BuiltinEnum, EnumType, StructFrozen, StructType, enum_, property_, struct_

if TYPE_CHECKING:
    from destack.language import Color, ColorIn, File

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


@struct_(StructType.ICON, frozen=True)
class Icon(StructFrozen):
    """An icon to be displayed in some view."""

    type: IconType = property_(30, default=False)
    # content
    emoji: str | None = property_(31)
    fa_name: str | None = property_(33)
    vsc_name: str | None = property_(34)
    file: Optional["File"] = property_(35)
    file_url: str | None = property_(36)
    # style
    color: Optional["Color"] = property_(40)


IconIn = Union[Icon, "File", str]


def to_icon(icon: IconIn, color: "ColorIn | None" = None) -> "Icon":
    """Turn something that could be an Icon into an Icon."""
    from destack.language import File, to_color

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
