from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Enum,
    EnumType,
    StructFrozen,
    StructType,
    builtin_enum,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    from destack.language import Color, File

# pyright: reportIncompatibleVariableOverride=false


#
# Icon
#


@builtin_enum(EnumType.ICON_TYPE)
class IconType(Enum):
    EMOJI = 1
    FONT_AWESOME = 3
    VS_CODE = 4
    FILE = 10
    FILE_URL = 11


@builtin_struct(StructType.ICON, frozen=True)
class Icon(StructFrozen):
    """An icon to be displayed in some view."""

    type: IconType = builtin_property(100)
    # content
    emoji: str | None = builtin_property(101)
    fa_name: str | None = builtin_property(102)
    vsc_name: str | None = builtin_property(103)
    file: Optional["File"] = builtin_property(104)
    file_url: str | None = builtin_property(105)
    # style
    color: Optional["Color"] = builtin_property(110)


IconIn = Union[Icon, "File", str]


def to_icon(icon: IconIn, color: "Color | None" = None) -> "Icon":
    """Turn something that could be an Icon into an Icon."""
    from destack.language import File

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


icon = to_icon
