from datetime import datetime
from typing import TYPE_CHECKING, Optional, override

import structlog

from bench.language.bench import Drive
from bench.language.const import EnumType, NodeType, PrimitiveType, StructType, enum_
from bench.language.node import (
    BenchNode,
    InlineStruct,
    Node,
    NodeReference,
    NodeReferenceBase,
    node_,
    struct_,
)
from bench.language.property import Property, p_internal, p_node_parent, p_regular
from bench.language.validation import TITLE_CONSTRAINT, ValidationHandler, constraint
from bench.proto.wire import FileData, FileReferenceData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Color

logger = structlog.get_logger(__name__)

FILE_HASH_LENGTH = 128  # 512 bits
MIME_TYPE_CONSTRAINT = constraint(min_length=1, max_length=255)
SHA512_CONSTRAINT = constraint(min_length=FILE_HASH_LENGTH, max_length=FILE_HASH_LENGTH)

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.FILE_KIND)
class FileKind(IdEnum):
    DRIVE = 1
    EXTERNAL = 2


@enum_(EnumType.FILE_RETENTION_MODE)
class FileRetentionMode(IdEnum):
    AUTOMATIC = 1  # garbage collected if no references
    MANUAL = 2  # never garbage collected
    TIMED = 3  # delete after a certain time


@node_(NodeType.FILE, unique=(("parent_id", "sha512"),))
class File(BenchNode[FileData]):
    """
    A file stored in a Drive (or externally).
    De-duplicated so that there's only one File per unique file content for our own files (sha512).
    """

    parent: Drive | None = p_node_parent(4, NodeType.DRIVE, is_system=True)

    # meta
    kind: FileKind = p_internal(30, default=FileKind.DRIVE, default_sql=None)
    title: str = p_regular(33, constraint=TITLE_CONSTRAINT)
    size: int = p_internal(
        34, primitive_type=PrimitiveType.INT64, constraint=constraint(min_value=0)
    )
    mime_type: str = p_internal(35, constraint=MIME_TYPE_CONSTRAINT)
    sha512: str | None = p_internal(
        36, constraint=constraint(min_length=FILE_HASH_LENGTH, max_length=FILE_HASH_LENGTH)
    )
    retention: FileRetentionMode | None = p_regular(
        37, default=FileRetentionMode.AUTOMATIC, default_sql=None
    )
    expires_at: Optional[datetime] = p_regular(38)

    # type-specific metadata (image size, audio/video length, thumbnail, ...)
    ...


@struct_(StructType.FILE_REFERENCE, inline=True)
class FileReference(
    InlineStruct[FileReferenceData],
    NodeReferenceBase[File, FileData, "FileReference", FileReferenceData],
):
    """
    A reference to a file stored somewhere. Like a NodeReference with file-specific metadata.
    """  # :RichReferences

    # ...NodeReferenceBase[30-39]

    kind: FileKind = p_internal(40)
    title: str = p_regular(43, constraint=TITLE_CONSTRAINT)
    size: Optional[int] = p_internal(44)
    sha512: Optional[str] = p_internal(45)
    mime_type: Optional[str] = p_internal(46)
    external_url: Optional[str] = p_internal(47)

    def _validate_component(self, properties: tuple[Property, ...], invalid: ValidationHandler):
        if self.type != NodeType.FILE:
            invalid("type", f"referenced node must be File, got {self.type}", (FileReference.type,))

    @override
    @staticmethod
    def from_node(node: Node) -> "FileReference":
        node_ref = NodeReference.from_node(node)
        file_ref = FileReference._copy_ref(FileReference, node_ref)
        for prop in FileReference.__declared_properties__.values():
            if hasattr(file_ref, prop.name):
                setattr(file_ref, prop.name, getattr(node, prop.name))
        return file_ref

    @override
    @staticmethod
    def from_node_data(node_data: FileData) -> FileReferenceData:
        node_ref = NodeReference.from_node_data(node_data)
        file_ref = FileReference._copy_ref(FileReferenceData, node_ref)
        for prop in FileReference.__declared_properties__.values():
            if hasattr(file_ref, prop.name):
                setattr(file_ref, prop.name, getattr(node_data, prop.name))
        return file_ref

    @override
    @staticmethod
    def from_node_as_data(node: File) -> FileReferenceData:
        node_ref = NodeReference.from_node_as_data(node)
        file_ref = FileReference._copy_ref(FileReferenceData, node_ref)
        for prop in FileReference.__declared_properties__.values():
            if hasattr(file_ref, prop.name):
                setattr(file_ref, prop.name, getattr(node, prop.name))
        return file_ref


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
    file: Optional["File"] = p_internal(
        32, require=False, array=False, references=NodeType.FILE, rich=True
    )
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
