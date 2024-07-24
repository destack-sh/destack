from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union, override

import structlog

from bench.language.bench import Drive
from bench.language.const import EnumType, NodeType, PrimitiveType, StructType, enum_
from bench.language.node import (
    BuiltinObject,
    NodeReference,
    NodeReferenceBase,
    RemoteNode,
    Struct,
    local_node_,
    object_component,
    struct_,
)
from bench.language.property import Property, p_internal, p_node_parent, p_regular
from bench.language.validation import TITLE_CONSTRAINT, ValidationHandler, constraint
from bench.proto.wire import FileData, FileReferenceData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Color, Package

logger = structlog.get_logger(__name__)

FILE_HASH_LENGTH = 64  # 256 bits
MIME_TYPE_CONSTRAINT = constraint(min_length=1, max_length=255)
SHA256_CONSTRAINT = constraint(min_length=FILE_HASH_LENGTH, max_length=FILE_HASH_LENGTH)

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.FILE_KIND)
class FileKind(IdEnum):
    DRIVE = 1
    DRIVE_INLINE = 2
    EXTERNAL = 3


@enum_(EnumType.FILE_RETENTION_MODE)
class FileRetentionMode(IdEnum):
    AUTOMATIC = 1  # garbage collected if no references
    MANUAL = 2  # never garbage collected
    TIMED = 3  # delete after a certain time


@enum_(EnumType.FILE_TYPE)
class FileType(IdEnum):
    TEXT = 1
    IMAGE = 2
    AUDIO = 3
    VIDEO = 4
    DOCUMENT = 5
    DATA = 6
    EXECUTABLE = 7
    GENERIC = 10


@object_component()
class FileInfoBase(BuiltinObject):
    """
    Base class for file info.
    """

    # content
    kind: FileKind = p_internal(40, default=FileKind.DRIVE, default_sql=None)
    drive: Drive | None = p_regular(
        41, references=NodeType.DRIVE, require=False, array=False, same_bench=True
    )
    url: Optional[str] = p_regular(42, default=None)
    title: str = p_regular(43, constraint=TITLE_CONSTRAINT)
    content: Optional[bytes] = p_regular(44, default=None, constraint=constraint(min_length=1))
    ...  # thumbnail/preview/...?

    # common meta
    coarse_type: FileType = p_internal(50)
    mime_type: str = p_internal(51, constraint=MIME_TYPE_CONSTRAINT)
    size: int = p_internal(
        52, primitive_type=PrimitiveType.INT64, constraint=constraint(min_value=0)
    )
    sha256: str | None = p_internal(53, constraint=SHA256_CONSTRAINT)

    # multimedia
    width: Optional[int] = p_internal(55, default=None)
    height: Optional[int] = p_internal(56, default=None)
    aspect_ratio: Optional[float] = p_internal(57, default=None)
    codec: Optional[str] = p_internal(58, default=None)
    duration: Optional[float] = p_internal(60, default=None)
    bitrate: Optional[int] = p_internal(61, default=None)
    channels: Optional[int] = p_internal(62, default=None)
    sample_rate: Optional[int] = p_internal(63, default=None)


@struct_(StructType.FILE_INFO)
class FileInfo(Struct, FileInfoBase):
    """
    File metadata.
    """

    ...


@local_node_(NodeType.FILE, indexes=(("drive_id", "sha256"),))
class File(RemoteNode[FileData], FileInfoBase):
    """
    A file stored in a Drive (or externally).
    De-duplicated so that there's only one File per unique file content for our own files.
    """

    parent: Union["Package", "Block", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.BLOCK, is_system=True
    )

    # meta
    retention: FileRetentionMode | None = p_regular(
        30, default=FileRetentionMode.AUTOMATIC, default_sql=None
    )
    expires_at: Optional[datetime] = p_regular(31)

    # content/info
    # ...FileInfoBase[40-69]


@struct_(StructType.FILE_REFERENCE)
class FileReference(
    Struct[FileReferenceData],
    FileInfoBase,
    NodeReferenceBase[File, FileData, "FileReference", FileReferenceData],
):
    """
    A reference to a File. Extends NodeReference with file-specific metadata.
    """  # :RichReferences

    # ...NodeReferenceBase[30-39]

    # content/info
    # ...FileInfoBase[40-69]

    def _validate_component(self, properties: tuple[Property, ...], invalid: ValidationHandler):
        if self.type != NodeType.FILE:
            invalid("type", f"referenced node must be File, got {self.type}", (FileReference.type,))

    @override
    @staticmethod
    def from_node(node: File) -> "FileReference":
        node_ref = NodeReference.from_node(node)
        file_ref = FileReference._clone_ref(FileReference, node_ref)
        for prop in FileInfoBase.__declared_properties__.values():
            if hasattr(file_ref, prop.name):
                setattr(file_ref, prop.name, getattr(node, prop.name))
        return file_ref

    @override
    @staticmethod
    def from_node_data(node_data: FileData) -> FileReferenceData:
        node_ref = NodeReference.from_node_data(node_data)
        file_ref = FileReference._clone_ref(FileReferenceData, node_ref)
        for prop in FileInfoBase.__declared_properties__.values():
            if hasattr(file_ref, prop.name):
                setattr(file_ref, prop.name, getattr(node_data, prop.name))
        return file_ref

    @override
    @staticmethod
    def from_node_as_data(node: File) -> FileReferenceData:
        node_ref = NodeReference.from_node_as_data(node)
        file_ref = FileReference._clone_ref(FileReferenceData, node_ref)
        for prop in FileInfoBase.__declared_properties__.values():
            if hasattr(file_ref, prop.name):
                setattr(file_ref, prop.name, getattr(node, prop.name))
        return file_ref


@enum_(EnumType.ICON_KIND)
class IconKind(IdEnum):
    EMOJI = 1
    FILE = 2
    FONT_AWESOME = 3


@struct_(StructType.ICON)
class Icon(Struct):
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
