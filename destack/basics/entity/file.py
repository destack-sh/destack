from datetime import timedelta
from typing import (
    TYPE_CHECKING,
    Optional,
)

from destack.core import (
    Entity,
    Enum,
    EnumType,
    Float32,
    NodeType,
    TraitType,
    UInt32,
    UInt64,
    builtin_entity,
    builtin_enum,
    builtin_property,
    builtin_property_parent,
)
from destack.utils.env import get_from_env

if TYPE_CHECKING:
    from destack import File, Space


FILE_HASH_LENGTH = 64  # 256 bits
MAX_FILE_SIZE = get_from_env(
    "MAX_FILE_SIZE", typ=int, default=1024 * 1024 * 128, description="Max file size (in bytes)"
)


# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.FILE_RETENTION_MODE)
class FileRetentionMode(Enum):
    AUTOMATIC = 1  # garbage collected if no references
    MANUAL = 2  # never garbage collected
    TIMED = 3  # delete after a certain time


@builtin_enum(EnumType.FILE_TYPE)
class FileType(Enum):
    TEXT = 1, None, None, "fas fa-file-lines"
    CODE = 2, None, None, "fas fa-file-code"
    IMAGE = 3, None, None, "fas fa-image"
    AUDIO = 4, None, None, "fas fa-volume"
    VIDEO = 5, None, None, "fas fa-video"
    DOCUMENT = 6, None, None, "fas fa-file-invoice"
    DATA = 7, None, None, "fas fa-database"
    ARCHIVE = 8, None, None, "fas fa-file-zipper"
    EXECUTABLE = 9, None, None, "fas fa-file-binary"
    GENERIC = 99, None, None, "fas fa-file"


@builtin_entity(NodeType.FILE, traits=(TraitType.RESOURCE,))
class File(Entity):
    """
    A File stored somewhere.
    """

    parent: Optional["Space"] = builtin_property_parent()
    type: FileType = builtin_property(100, is_repr=True)

    # meta
    mime_type: str | None = builtin_property(120, is_repr=True)
    format: FileType | None = builtin_property(121, is_repr=True)
    size: UInt64 | None = builtin_property(122, is_repr=True)
    sha256: str | None = builtin_property(123)
    width: UInt32 | None = builtin_property(124)
    height: UInt32 | None = builtin_property(125)
    aspect_ratio: Float32 | None = builtin_property(126)
    codec: str | None = builtin_property(127)
    duration: Optional[timedelta] = builtin_property(128)

    # content
    url: str | None = builtin_property(130, is_repr=True)  # if external
    content_url: str | None = builtin_property(131)  # if external
    thumbnail_url: str | None = builtin_property(132)  # if external
    favicon_url: str | None = builtin_property(133)
    thumbnail_width: UInt32 | None = builtin_property(134)
    thumbnail_height: UInt32 | None = builtin_property(135)
    content: bytes | None = builtin_property(136)
