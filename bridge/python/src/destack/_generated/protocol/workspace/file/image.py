# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.diagnostic.diagnostic
import destack._generated.protocol.source.edit.text
import destack._generated.protocol.source.file.model.file
import destack._generated.protocol.source.file.model.module
import destack._generated.protocol.source.file.model.type
import destack._generated.protocol.source.file.path.uri

if TYPE_CHECKING:
    from destack._generated.protocol.source.diagnostic.diagnostic import (
        Diagnostic,
    )

    from destack._generated.protocol.source.edit.text import (
        TextChange,
    )

    from destack._generated.protocol.source.file.model.file import (
        FileId,
    )

    from destack._generated.protocol.source.file.model.module import (
        ModuleId,
    )

    from destack._generated.protocol.source.file.model.type import (
        FileType,
    )

    from destack._generated.protocol.source.file.path.uri import (
        Uri,
    )


@dataclass(frozen=True, slots=True)
class FileOperationOpenText:
    """Open editor text content."""

    """Path being opened."""
    path: str
    """Editor document URI."""
    uri: Uri
    """Editor document version."""
    version: int
    """Current text content."""
    content: str
    kind: Literal["openText"] = "openText"


@dataclass(frozen=True, slots=True)
class FileOperationOpenBytes:
    """Open editor binary content."""

    """Path being opened."""
    path: str
    """Editor document URI."""
    uri: Uri
    """Editor document version."""
    version: int
    """Current binary content."""
    content: bytes | bytearray | Sequence[int]
    kind: Literal["openBytes"] = "openBytes"


@dataclass(frozen=True, slots=True)
class FileOperationChangeText:
    """Change editor text content."""

    """Path being changed."""
    path: str
    """Editor document URI."""
    uri: Uri
    """Editor document version."""
    version: int
    """Current text content."""
    content: str
    kind: Literal["changeText"] = "changeText"


@dataclass(frozen=True, slots=True)
class FileOperationChangeBytes:
    """Change editor binary content."""

    """Path being changed."""
    path: str
    """Editor document URI."""
    uri: Uri
    """Editor document version."""
    version: int
    """Current binary content."""
    content: bytes | bytearray | Sequence[int]
    kind: Literal["changeBytes"] = "changeBytes"


@dataclass(frozen=True, slots=True)
class FileOperationPatchText:
    """Patch editor text content."""

    """Path being patched."""
    path: str
    """Editor document URI."""
    uri: Uri
    """Editor document version."""
    version: int
    """Incremental text changes."""
    changes: Sequence[TextChange]
    kind: Literal["patchText"] = "patchText"


@dataclass(frozen=True, slots=True)
class FileOperationSaveText:
    """Save editor text content."""

    """Path being saved."""
    path: str
    """Current text content."""
    content: str | None
    kind: Literal["saveText"] = "saveText"


@dataclass(frozen=True, slots=True)
class FileOperationSaveBytes:
    """Save editor binary content."""

    """Path being saved."""
    path: str
    """Current binary content."""
    content: bytes | bytearray | Sequence[int] | None
    kind: Literal["saveBytes"] = "saveBytes"


@dataclass(frozen=True, slots=True)
class FileOperationClose:
    """Close editor overlay state and restore filesystem truth."""

    """Path being closed."""
    path: str
    kind: Literal["close"] = "close"


@dataclass(frozen=True, slots=True)
class FileOperationWriteText:
    """Write text content to disk and workspace state."""

    """Path being written."""
    path: str
    """Current text content."""
    content: str
    kind: Literal["writeText"] = "writeText"


@dataclass(frozen=True, slots=True)
class FileOperationWriteBytes:
    """Write binary content to disk and workspace state."""

    """Path being written."""
    path: str
    """Current binary content."""
    content: bytes | bytearray | Sequence[int]
    kind: Literal["writeBytes"] = "writeBytes"


@dataclass(frozen=True, slots=True)
class FileOperationRemove:
    """Remove a file from disk and workspace state."""

    """Path being removed."""
    path: str
    kind: Literal["remove"] = "remove"


@dataclass(frozen=True, slots=True)
class FileOperationMove:
    """Move a file on disk and workspace state."""

    """Source path."""
    from_: str
    """Destination path."""
    to: str
    kind: Literal["move"] = "move"


"""File operation applied through a workspace."""
FileOperation: TypeAlias = (
    FileOperationOpenText
    | FileOperationOpenBytes
    | FileOperationChangeText
    | FileOperationChangeBytes
    | FileOperationPatchText
    | FileOperationSaveText
    | FileOperationSaveBytes
    | FileOperationClose
    | FileOperationWriteText
    | FileOperationWriteBytes
    | FileOperationRemove
    | FileOperationMove
)


def encode_file_operation(writer: Writer, value: FileOperation) -> None:
    if value.kind == "openText":
        writer.write_unsigned(0)
        writer.write_string(value.path)
        destack._generated.protocol.source.file.path.uri.encode_uri(writer, value.uri)
        writer.write_signed(value.version)
        writer.write_string(value.content)
    elif value.kind == "openBytes":
        writer.write_unsigned(1)
        writer.write_string(value.path)
        destack._generated.protocol.source.file.path.uri.encode_uri(writer, value.uri)
        writer.write_signed(value.version)
        writer.write_byte_slice(value.content)
    elif value.kind == "changeText":
        writer.write_unsigned(2)
        writer.write_string(value.path)
        destack._generated.protocol.source.file.path.uri.encode_uri(writer, value.uri)
        writer.write_signed(value.version)
        writer.write_string(value.content)
    elif value.kind == "changeBytes":
        writer.write_unsigned(3)
        writer.write_string(value.path)
        destack._generated.protocol.source.file.path.uri.encode_uri(writer, value.uri)
        writer.write_signed(value.version)
        writer.write_byte_slice(value.content)
    elif value.kind == "patchText":
        writer.write_unsigned(4)
        writer.write_string(value.path)
        destack._generated.protocol.source.file.path.uri.encode_uri(writer, value.uri)
        writer.write_signed(value.version)
        writer.write_unsigned(len(value.changes))
        for item_0 in value.changes:
            destack._generated.protocol.source.edit.text.encode_text_change(
                writer, item_0
            )
    elif value.kind == "saveText":
        writer.write_unsigned(5)
        writer.write_string(value.path)
        if value.content is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_string(value.content)
    elif value.kind == "saveBytes":
        writer.write_unsigned(6)
        writer.write_string(value.path)
        if value.content is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_byte_slice(value.content)
    elif value.kind == "close":
        writer.write_unsigned(7)
        writer.write_string(value.path)
    elif value.kind == "writeText":
        writer.write_unsigned(8)
        writer.write_string(value.path)
        writer.write_string(value.content)
    elif value.kind == "writeBytes":
        writer.write_unsigned(9)
        writer.write_string(value.path)
        writer.write_byte_slice(value.content)
    elif value.kind == "remove":
        writer.write_unsigned(10)
        writer.write_string(value.path)
    elif value.kind == "move":
        writer.write_unsigned(11)
        writer.write_string(value.from_)
        writer.write_string(value.to)
    else:
        raise SerdeError("unknown enum variant")


def decode_file_operation(reader: Reader) -> FileOperation:
    variant = reader.read_number()

    if variant == 0:
        field_0 = reader.read_string()
        field_1 = destack._generated.protocol.source.file.path.uri.decode_uri(reader)
        field_2 = reader.read_signed_number()
        field_3 = reader.read_string()

        return FileOperationOpenText(
            path=field_0,
            uri=field_1,
            version=field_2,
            content=field_3,
        )
    elif variant == 1:
        field_0 = reader.read_string()
        field_1 = destack._generated.protocol.source.file.path.uri.decode_uri(reader)
        field_2 = reader.read_signed_number()
        field_3 = reader.read_byte_slice()

        return FileOperationOpenBytes(
            path=field_0,
            uri=field_1,
            version=field_2,
            content=field_3,
        )
    elif variant == 2:
        field_0 = reader.read_string()
        field_1 = destack._generated.protocol.source.file.path.uri.decode_uri(reader)
        field_2 = reader.read_signed_number()
        field_3 = reader.read_string()

        return FileOperationChangeText(
            path=field_0,
            uri=field_1,
            version=field_2,
            content=field_3,
        )
    elif variant == 3:
        field_0 = reader.read_string()
        field_1 = destack._generated.protocol.source.file.path.uri.decode_uri(reader)
        field_2 = reader.read_signed_number()
        field_3 = reader.read_byte_slice()

        return FileOperationChangeBytes(
            path=field_0,
            uri=field_1,
            version=field_2,
            content=field_3,
        )
    elif variant == 4:
        field_0 = reader.read_string()
        field_1 = destack._generated.protocol.source.file.path.uri.decode_uri(reader)
        field_2 = reader.read_signed_number()
        field_3 = [
            destack._generated.protocol.source.edit.text.decode_text_change(reader)
            for _ in range(reader.read_number())
        ]

        return FileOperationPatchText(
            path=field_0,
            uri=field_1,
            version=field_2,
            changes=field_3,
        )
    elif variant == 5:
        field_0 = reader.read_string()
        field_1 = reader.read_option(lambda: reader.read_string())

        return FileOperationSaveText(
            path=field_0,
            content=field_1,
        )
    elif variant == 6:
        field_0 = reader.read_string()
        field_1 = reader.read_option(lambda: reader.read_byte_slice())

        return FileOperationSaveBytes(
            path=field_0,
            content=field_1,
        )
    elif variant == 7:
        field_0 = reader.read_string()

        return FileOperationClose(
            path=field_0,
        )
    elif variant == 8:
        field_0 = reader.read_string()
        field_1 = reader.read_string()

        return FileOperationWriteText(
            path=field_0,
            content=field_1,
        )
    elif variant == 9:
        field_0 = reader.read_string()
        field_1 = reader.read_byte_slice()

        return FileOperationWriteBytes(
            path=field_0,
            content=field_1,
        )
    elif variant == 10:
        field_0 = reader.read_string()

        return FileOperationRemove(
            path=field_0,
        )
    elif variant == 11:
        field_0 = reader.read_string()
        field_1 = reader.read_string()

        return FileOperationMove(
            from_=field_0,
            to=field_1,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class FileUpdate:
    """File update emitted by the workspace."""

    """Updated module id when known."""
    module_id: ModuleId | None
    """Updated file id."""
    file_id: FileId
    """Diagnostic uri for this update."""
    diagnostic_uri: Uri
    """Protocol file version for diagnostics when the file is open."""
    diagnostic_version: int | None
    """Updated file image when the file still exists."""
    file: FileImage | None
    """Whether this update removed the file."""
    is_removed: bool
    """The coarse change kind for this file."""
    kind: UpdateKind
    """Diagnostics for this file."""
    diagnostics: Sequence[Diagnostic]


def encode_file_update(writer: Writer, value: FileUpdate) -> None:
    if value.module_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module_id
        )
    destack._generated.protocol.source.file.model.file.encode_file_id(
        writer, value.file_id
    )
    destack._generated.protocol.source.file.path.uri.encode_uri(
        writer, value.diagnostic_uri
    )
    if value.diagnostic_version is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_signed(value.diagnostic_version)
    if value.file is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_file_image(writer, value.file)
    writer.write_bool(value.is_removed)
    encode_update_kind(writer, value.kind)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )


def decode_file_update(reader: Reader) -> FileUpdate:
    field_0 = reader.read_option(
        lambda: destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
    )
    field_1 = destack._generated.protocol.source.file.model.file.decode_file_id(reader)
    field_2 = destack._generated.protocol.source.file.path.uri.decode_uri(reader)
    field_3 = reader.read_option(lambda: reader.read_signed_number())
    field_4 = reader.read_option(lambda: decode_file_image(reader))
    field_5 = reader.read_bool()
    field_6 = decode_update_kind(reader)
    field_7 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]

    return FileUpdate(
        module_id=field_0,
        file_id=field_1,
        diagnostic_uri=field_2,
        diagnostic_version=field_3,
        file=field_4,
        is_removed=field_5,
        kind=field_6,
        diagnostics=field_7,
    )


@dataclass(frozen=True, slots=True)
class FileImage:
    """In-memory image for one updated file."""

    """File id in the registry."""
    id: FileId
    """File name."""
    name: str
    """File uri."""
    uri: Uri
    """Optional file path."""
    path: str | None
    """File type."""
    file_type: FileType
    """Optional text content."""
    content: str | None


def encode_file_image(writer: Writer, value: FileImage) -> None:
    destack._generated.protocol.source.file.model.file.encode_file_id(writer, value.id)
    writer.write_string(value.name)
    destack._generated.protocol.source.file.path.uri.encode_uri(writer, value.uri)
    if value.path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.path)
    destack._generated.protocol.source.file.model.type.encode_file_type(
        writer, value.file_type
    )
    if value.content is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.content)


def decode_file_image(reader: Reader) -> FileImage:
    field_0 = destack._generated.protocol.source.file.model.file.decode_file_id(reader)
    field_1 = reader.read_string()
    field_2 = destack._generated.protocol.source.file.path.uri.decode_uri(reader)
    field_3 = reader.read_option(lambda: reader.read_string())
    field_4 = destack._generated.protocol.source.file.model.type.decode_file_type(
        reader
    )
    field_5 = reader.read_option(lambda: reader.read_string())

    return FileImage(
        id=field_0,
        name=field_1,
        uri=field_2,
        path=field_3,
        file_type=field_4,
        content=field_5,
    )


"""One coarse kind for a workspace file update."""
UpdateKind: TypeAlias = Literal["source"] | Literal["config"]


def encode_update_kind(writer: Writer, value: UpdateKind) -> None:
    if value == "source":
        writer.write_unsigned(0)
    elif value == "config":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_update_kind(reader: Reader) -> UpdateKind:
    variant = reader.read_number()

    if variant == 0:
        return "source"
    elif variant == 1:
        return "config"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "FileOperation",
    "encode_file_operation",
    "decode_file_operation",
    "FileOperationOpenText",
    "FileOperationOpenBytes",
    "FileOperationChangeText",
    "FileOperationChangeBytes",
    "FileOperationPatchText",
    "FileOperationSaveText",
    "FileOperationSaveBytes",
    "FileOperationClose",
    "FileOperationWriteText",
    "FileOperationWriteBytes",
    "FileOperationRemove",
    "FileOperationMove",
    "FileUpdate",
    "encode_file_update",
    "decode_file_update",
    "FileImage",
    "encode_file_image",
    "decode_file_image",
    "UpdateKind",
    "encode_update_kind",
    "decode_update_kind",
]
