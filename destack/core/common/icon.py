from typing import TYPE_CHECKING, Optional

from ..builtin import (
    EnumType,
    OptionEnum,
    StructFrozen,
    StructType,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    from destack import Color, File

# pyright: reportIncompatibleVariableOverride=false


#
# Icon
#


@declare_enum(EnumType.ICON_TYPE)
class IconType(OptionEnum):
    EMOJI = declare_option(1)
    FILE = declare_option(10)
    FILE_URL = declare_option(11)


@declare_struct(StructType.ICON, frozen=True)
class Icon(StructFrozen):
    """An icon to be displayed in some view."""

    type: IconType = declare_property(100)
    # content
    emoji: str | None = declare_property(101)
    fa_name: str | None = declare_property(102)
    vsc_name: str | None = declare_property(103)
    file: Optional["File"] = declare_property(104)
    file_url: str | None = declare_property(105)
    # style
    color: Optional["Color"] = declare_property(110)
