from datetime import datetime
from typing import TYPE_CHECKING, Optional

import structlog

from bench.language.bench import Drive
from bench.language.const import EnumType, NodeType, PrimitiveType, StructType, enum_
from bench.language.node import BenchNode, InlineStruct, node_, struct_
from bench.language.property import p_internal, p_node_parent, p_regular
from bench.language.validation import TITLE_CONSTRAINT, constrain
from bench.proto.wire import FileData, FileReferenceData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Color

logger = structlog.get_logger(__name__)

FILE_HASH_LENGTH = 128  # 512 bits
MIME_TYPE_CONSTRAINT = constrain(min_length=1, max_length=255)
SHA512_CONSTRAINT = constrain(min_length=FILE_HASH_LENGTH, max_length=FILE_HASH_LENGTH)

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.FILE_KIND)
class FileKind(IdEnum):
    BLOB = 1
    EXTERNAL = 2


@enum_(EnumType.FILE_RETENTION_MODE)
class FileRetentionMode(IdEnum):
    AUTOMATIC = 1  # garbage collected if no references
    MANUAL = 2  # never garbage collected
    TIMED = 3  # delete after a certain time


@node_(NodeType.FILE, unique=(("parent_id", "sha512"),))
class File(BenchNode[FileData]):
    """
    A file stored in a Drive.
    De-duplicated so that there's only one File per unique file content (sha512).
    """

    parent: Drive | None = p_node_parent(4, NodeType.DRIVE, is_system=True)

    # meta
    title: str = p_regular(33, constraint=TITLE_CONSTRAINT)
    size: int = p_internal(34, primitive_type=PrimitiveType.INT64)
    sha512: str = p_internal(
        35, constraint=constrain(min_length=FILE_HASH_LENGTH, max_length=FILE_HASH_LENGTH)
    )
    mime_type: str = p_internal(36, constraint=MIME_TYPE_CONSTRAINT)
    retention: FileRetentionMode = p_regular(37)
    expires_at: Optional[datetime] = p_regular(38)

    # type-specific metadata (image size, audio/video length, thumbnail, ...)
    ...


@struct_(StructType.FILE_REFERENCE, inline=True)
class FileReference(InlineStruct[FileReferenceData]):
    """
    A reference to a file stored somewhere.
    Like a NodeReference with file-specific metadata.
    """

    # meta
    kind: FileKind = p_internal(30)
    title: str = p_regular(33, constraint=TITLE_CONSTRAINT)
    size: Optional[int] = p_internal(34)
    sha512: Optional[str] = p_internal(35)
    mime_type: Optional[str] = p_internal(36)

    # content
    file: Optional["File"] = p_internal(40, require=False, array=False, references=NodeType.FILE)
    external_url: Optional[str] = p_internal(41)


@enum_(EnumType.ICON_KIND)
class IconKind(IdEnum):
    EMOJI = 1
    FILE = 2
    FONT_AWESOME = 3


@struct_(StructType.ICON, inline=True)
class Icon(InlineStruct):
    """An icon to be displayed in some view."""

    kind: IconKind = p_internal(30, default=False)
    # content
    emoji: Optional[str] = p_internal(31, require=False)
    file: Optional["File"] = p_internal(32, require=False, array=False, references=NodeType.FILE)
    fa_name: Optional[str] = p_internal(33, require=False)
    # style
    color: Optional["Color"] = p_internal(40, require=False, array=False, struct=StructType.COLOR)

    @staticmethod
    def new(icon: "IconIn") -> "Icon":
        return to_icon(icon)


IconIn = Icon | str


def to_icon(icon: IconIn) -> Icon:
    if isinstance(icon, str):
        if icon.startswith("fa-"):
            return Icon(kind=IconKind.FONT_AWESOME, fa_name=icon)
        else:
            return Icon(kind=IconKind.EMOJI, emoji=icon)
    else:
        return icon
