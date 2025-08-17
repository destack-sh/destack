from typing import TYPE_CHECKING, Optional

from ..builtin import (
    EnumType,
    OptionEnum,
    ReferenceType,
    Struct,
    StructType,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    from destack import Color, File


#
# Icon
#


@declare_enum(EnumType.ICON_TYPE)
class IconType(OptionEnum):
    EMOJI = declare_option(1)
    FILE = declare_option(10)
    FILE_URL = declare_option(11)


@declare_struct(StructType.ICON)
class Icon(Struct):
    """An icon to be displayed in some view."""

    type: IconType = declare_property(100, tag=None)
    # content
    emoji: str | None = declare_property(101, tag=None)
    fa_name: str | None = declare_property(102, tag=None)
    vsc_name: str | None = declare_property(103, tag=None)
    file: Optional["File"] = declare_property(
        104,
        reference_type=ReferenceType.IDENTITY,
        tag=None,
    )
    file_url: str | None = declare_property(105, tag=None)
    # style
    color: Optional["Color"] = declare_property(110, tag=None)
