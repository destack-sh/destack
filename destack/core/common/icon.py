from typing import TYPE_CHECKING, Optional

from ..builtin import (
    Enum,
    EnumType,
    StructFrozen,
    StructType,
    builtin_enum,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    from destack import Color, File

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
