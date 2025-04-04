from typing import TYPE_CHECKING, Optional

from .color import ColorIn, to_color
from .const import BuiltinEnum, EnumType, StructType, enum_
from .property import p_internal
from .struct import Struct, struct_

if TYPE_CHECKING:
    from bench.language import Color

# pyright: reportIncompatibleVariableOverride=false


#
# Icon
#


@enum_(EnumType.ICON_TYPE)
class IconType(BuiltinEnum):
    EMOJI = 1
    FONT_AWESOME = 3
    VS_CODE = 4
    # FILE?


@struct_(StructType.ICON)
class Icon(Struct):
    """An icon to be displayed in some view."""

    type: IconType = p_internal(30, default=False)
    # content
    # NOTE :Robustness: emoji's should have a (regex?) constraint, but that's pretty hard
    #  (we used to have \p{Emoji_Presentation}, but that's too strict)
    emoji: Optional[str] = p_internal(31, require=False)
    fa_name: Optional[str] = p_internal(33, require=False)
    vsc_name: Optional[str] = p_internal(34, require=False)
    # style
    color: Optional["Color"] = p_internal(40, require=False, array=False, struct=StructType.COLOR)

    @staticmethod
    def new(icon: "IconIn", color: ColorIn | None = None) -> "Icon":
        return to_icon(icon, color)


IconIn = Icon | str


def to_icon(icon: IconIn, color: ColorIn | None = None) -> Icon:
    """Turn something that could be an Icon into an Icon."""
    if isinstance(icon, str):
        color = to_color(color) if color else None
        if icon.startswith("fa"):
            return Icon(type=IconType.FONT_AWESOME, fa_name=icon, color=color)
        else:
            return Icon(type=IconType.EMOJI, emoji=icon, color=color)
    else:
        return icon


def reverse_icon(icon: Icon) -> IconIn:
    """Turn an Icon back into something simpler that can be turned back into an Icon."""
    if icon.type == IconType.FONT_AWESOME:
        assert icon.fa_name, f"no fa_name for {icon!r}"
        return icon.fa_name
    elif icon.type == IconType.EMOJI:
        assert icon.emoji, f"no emoji for {icon!r}"
        return icon.emoji
    else:
        return icon


icon = to_icon
