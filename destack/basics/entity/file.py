from datetime import timedelta
from typing import (
    TYPE_CHECKING,
    Optional,
)

from destack.core import (
    Entity,
    EnumType,
    Float32,
    NodeType,
    OptionEnum,
    TraitType,
    UInt32,
    UInt64,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
    declare_property_parent,
)

if TYPE_CHECKING:
    from destack import File, Space


FILE_HASH_LENGTH = 64  # 256 bits


# nocheckin: turn FileType into sub-Entities (Image, Audio, Video, etc.)


@declare_enum(EnumType.FILE_TYPE)
class FileType(OptionEnum):
    TEXT = declare_option(1, "Text", description="A text file")
    CODE = declare_option(2, "Code", description="A code file")
    IMAGE = declare_option(3, "Image", description="An image file")
    AUDIO = declare_option(4, "Audio", description="An audio file")
    VIDEO = declare_option(5, "Video", description="A video file")
    DOCUMENT = declare_option(6, "Document", description="A document file")
    DATA = declare_option(7, "Data", description="A data file")
    ARCHIVE = declare_option(8, "Archive", description="An archive file")
    EXECUTABLE = declare_option(9, "Executable", description="An executable file")
    GENERIC = declare_option(99, "Generic", description="A generic file")


@declare_entity(NodeType.FILE, traits=(TraitType.RESOURCE,))
class File(Entity):
    """
    A File stored somewhere.
    """

    parent: Optional["Space"] = declare_property_parent()
    type: FileType = declare_property(100, is_repr=True)

    # meta
    mime_type: str | None = declare_property(120, is_repr=True)
    format: FileType | None = declare_property(121, is_repr=True)
    size: UInt64 | None = declare_property(122, is_repr=True)
    sha256: str | None = declare_property(123)
    width: UInt32 | None = declare_property(124)
    height: UInt32 | None = declare_property(125)
    aspect_ratio: Float32 | None = declare_property(126)
    codec: str | None = declare_property(127)
    duration: Optional[timedelta] = declare_property(128)

    # content
    url: str | None = declare_property(130, is_repr=True)  # if external
    content_url: str | None = declare_property(131)  # if external
    thumbnail_url: str | None = declare_property(132)  # if external
    favicon_url: str | None = declare_property(133)
    thumbnail_width: UInt32 | None = declare_property(134)
    thumbnail_height: UInt32 | None = declare_property(135)
    content: bytes | None = declare_property(136)
