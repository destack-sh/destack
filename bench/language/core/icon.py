from typing import TYPE_CHECKING, Optional

from .const import BuiltinEnum, EnumType, StructType, enum_
from .property import p_internal
from .struct import Struct, struct_
from .validation import EMOJI_CONSTRAINT

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
    emoji: Optional[str] = p_internal(31, require=False, constraint=EMOJI_CONSTRAINT)
    fa_name: Optional[str] = p_internal(33, require=False)
    vsc_name: Optional[str] = p_internal(34, require=False)
    # style
    color: Optional["Color"] = p_internal(40, require=False, array=False, struct=StructType.COLOR)

    @staticmethod
    def new(icon: "IconIn") -> "Icon":
        return to_icon(icon)


IconIn = Icon | str


def to_icon(icon: IconIn) -> Icon:
    """Turn something that could be an Icon into an Icon."""
    if isinstance(icon, str):
        if icon.startswith("fa"):
            return Icon(type=IconType.FONT_AWESOME, fa_name=icon)
        else:
            return Icon(type=IconType.EMOJI, emoji=icon)
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
