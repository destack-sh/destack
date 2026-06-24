# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    bytes_from_json,
    bytes_to_json,
    json_array,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.source.diagnostic.diagnostic
import destack._generated.source.edit.text
import destack._generated.source.file.model.file
import destack._generated.source.file.model.module
import destack._generated.source.file.model.type
import destack._generated.source.file.path.uri


@dataclass(frozen=True, slots=True)
class FileOperationOpenText:
    """Open editor text content."""

    # path being opened
    path: str
    # editor document URI
    uri: destack._generated.source.file.path.uri.Uri
    # editor document version
    version: int
    # current text content
    content: str
    kind: typing.Literal["openText"] = "openText"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


@dataclass(frozen=True, slots=True)
class FileOperationOpenBytes:
    """Open editor binary content."""

    # path being opened
    path: str
    # editor document URI
    uri: destack._generated.source.file.path.uri.Uri
    # editor document version
    version: int
    # current binary content
    content: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["openBytes"] = "openBytes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


@dataclass(frozen=True, slots=True)
class FileOperationChangeText:
    """Change editor text content."""

    # path being changed
    path: str
    # editor document URI
    uri: destack._generated.source.file.path.uri.Uri
    # editor document version
    version: int
    # current text content
    content: str
    kind: typing.Literal["changeText"] = "changeText"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


@dataclass(frozen=True, slots=True)
class FileOperationChangeBytes:
    """Change editor binary content."""

    # path being changed
    path: str
    # editor document URI
    uri: destack._generated.source.file.path.uri.Uri
    # editor document version
    version: int
    # current binary content
    content: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["changeBytes"] = "changeBytes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


@dataclass(frozen=True, slots=True)
class FileOperationPatchText:
    """Patch editor text content."""

    # path being patched
    path: str
    # editor document URI
    uri: destack._generated.source.file.path.uri.Uri
    # editor document version
    version: int
    # incremental text changes
    changes: Sequence[destack._generated.source.edit.text.TextChange]
    kind: typing.Literal["patchText"] = "patchText"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


@dataclass(frozen=True, slots=True)
class FileOperationSaveText:
    """Save editor text content."""

    # path being saved
    path: str
    # current text content
    content: str | None
    kind: typing.Literal["saveText"] = "saveText"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


@dataclass(frozen=True, slots=True)
class FileOperationSaveBytes:
    """Save editor binary content."""

    # path being saved
    path: str
    # current binary content
    content: builtins.bytes | bytearray | Sequence[int] | None
    kind: typing.Literal["saveBytes"] = "saveBytes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


@dataclass(frozen=True, slots=True)
class FileOperationClose:
    """Close editor overlay state and restore filesystem truth."""

    # path being closed
    path: str
    kind: typing.Literal["close"] = "close"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


@dataclass(frozen=True, slots=True)
class FileOperationWriteText:
    """Write text content to disk and workspace state."""

    # path being written
    path: str
    # current text content
    content: str
    kind: typing.Literal["writeText"] = "writeText"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


@dataclass(frozen=True, slots=True)
class FileOperationWriteBytes:
    """Write binary content to disk and workspace state."""

    # path being written
    path: str
    # current binary content
    content: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["writeBytes"] = "writeBytes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


@dataclass(frozen=True, slots=True)
class FileOperationRemove:
    """Remove a file from disk and workspace state."""

    # path being removed
    path: str
    kind: typing.Literal["remove"] = "remove"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


@dataclass(frozen=True, slots=True)
class FileOperationMove:
    """Move a file on disk and workspace state."""

    # source path
    from_: str
    # destination path
    to: str
    kind: typing.Literal["move"] = "move"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_operation(self)


"""File operation applied through a workspace."""
FileOperation: typing.TypeAlias = (
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


def encode_file_operation(writer: BinaryWriter, value: FileOperation) -> None:
    """Encode one FileOperation."""
    if value.kind == "openText":
        writer.write_unsigned(0)
        writer.write_string(value.path)
        destack._generated.source.file.path.uri.encode_uri(writer, value.uri)
        writer.write_signed(value.version)
        writer.write_string(value.content)
    elif value.kind == "openBytes":
        writer.write_unsigned(1)
        writer.write_string(value.path)
        destack._generated.source.file.path.uri.encode_uri(writer, value.uri)
        writer.write_signed(value.version)
        writer.write_byte_slice(value.content)
    elif value.kind == "changeText":
        writer.write_unsigned(2)
        writer.write_string(value.path)
        destack._generated.source.file.path.uri.encode_uri(writer, value.uri)
        writer.write_signed(value.version)
        writer.write_string(value.content)
    elif value.kind == "changeBytes":
        writer.write_unsigned(3)
        writer.write_string(value.path)
        destack._generated.source.file.path.uri.encode_uri(writer, value.uri)
        writer.write_signed(value.version)
        writer.write_byte_slice(value.content)
    elif value.kind == "patchText":
        writer.write_unsigned(4)
        writer.write_string(value.path)
        destack._generated.source.file.path.uri.encode_uri(writer, value.uri)
        writer.write_signed(value.version)
        writer.write_unsigned(len(value.changes))
        for item_value_changes_0 in value.changes:
            destack._generated.source.edit.text.encode_text_change(
                writer, item_value_changes_0
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


def decode_file_operation(reader: BinaryReader) -> FileOperation:
    """Decode one FileOperation."""
    variant = reader.read_number()

    if variant == 0:
        path = reader.read_string()
        uri = destack._generated.source.file.path.uri.decode_uri(reader)
        version = reader.read_signed_number()
        content = reader.read_string()

        return FileOperationOpenText(
            path=path,
            uri=uri,
            version=version,
            content=content,
        )
    elif variant == 1:
        path = reader.read_string()
        uri = destack._generated.source.file.path.uri.decode_uri(reader)
        version = reader.read_signed_number()
        content = reader.read_byte_slice()

        return FileOperationOpenBytes(
            path=path,
            uri=uri,
            version=version,
            content=content,
        )
    elif variant == 2:
        path = reader.read_string()
        uri = destack._generated.source.file.path.uri.decode_uri(reader)
        version = reader.read_signed_number()
        content = reader.read_string()

        return FileOperationChangeText(
            path=path,
            uri=uri,
            version=version,
            content=content,
        )
    elif variant == 3:
        path = reader.read_string()
        uri = destack._generated.source.file.path.uri.decode_uri(reader)
        version = reader.read_signed_number()
        content = reader.read_byte_slice()

        return FileOperationChangeBytes(
            path=path,
            uri=uri,
            version=version,
            content=content,
        )
    elif variant == 4:
        path = reader.read_string()
        uri = destack._generated.source.file.path.uri.decode_uri(reader)
        version = reader.read_signed_number()
        changes = [
            destack._generated.source.edit.text.decode_text_change(reader)
            for _ in range(reader.read_number())
        ]

        return FileOperationPatchText(
            path=path,
            uri=uri,
            version=version,
            changes=changes,
        )
    elif variant == 5:
        path = reader.read_string()
        content = reader.read_option(lambda: reader.read_string())

        return FileOperationSaveText(
            path=path,
            content=content,
        )
    elif variant == 6:
        path = reader.read_string()
        content = reader.read_option(lambda: reader.read_byte_slice())

        return FileOperationSaveBytes(
            path=path,
            content=content,
        )
    elif variant == 7:
        path = reader.read_string()

        return FileOperationClose(
            path=path,
        )
    elif variant == 8:
        path = reader.read_string()
        content = reader.read_string()

        return FileOperationWriteText(
            path=path,
            content=content,
        )
    elif variant == 9:
        path = reader.read_string()
        content = reader.read_byte_slice()

        return FileOperationWriteBytes(
            path=path,
            content=content,
        )
    elif variant == 10:
        path = reader.read_string()

        return FileOperationRemove(
            path=path,
        )
    elif variant == 11:
        from_ = reader.read_string()
        to = reader.read_string()

        return FileOperationMove(
            from_=from_,
            to=to,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_file_operation(value: FileOperation) -> Json:
    """Return one JSON value for one FileOperation."""
    if value.kind == "openText":
        return {
            "kind": "openText",
            "path": value.path,
            "uri": destack._generated.source.file.path.uri.to_json_uri(value.uri),
            "version": value.version,
            "content": value.content,
        }
    elif value.kind == "openBytes":
        return {
            "kind": "openBytes",
            "path": value.path,
            "uri": destack._generated.source.file.path.uri.to_json_uri(value.uri),
            "version": value.version,
            "content": bytes_to_json(value.content),
        }
    elif value.kind == "changeText":
        return {
            "kind": "changeText",
            "path": value.path,
            "uri": destack._generated.source.file.path.uri.to_json_uri(value.uri),
            "version": value.version,
            "content": value.content,
        }
    elif value.kind == "changeBytes":
        return {
            "kind": "changeBytes",
            "path": value.path,
            "uri": destack._generated.source.file.path.uri.to_json_uri(value.uri),
            "version": value.version,
            "content": bytes_to_json(value.content),
        }
    elif value.kind == "patchText":
        return {
            "kind": "patchText",
            "path": value.path,
            "uri": destack._generated.source.file.path.uri.to_json_uri(value.uri),
            "version": value.version,
            "changes": [
                destack._generated.source.edit.text.to_json_text_change(item_0)
                for item_0 in value.changes
            ],
        }
    elif value.kind == "saveText":
        return {
            "kind": "saveText",
            "path": value.path,
            **({} if value.content is None else {"content": value.content}),
        }
    elif value.kind == "saveBytes":
        return {
            "kind": "saveBytes",
            "path": value.path,
            **(
                {}
                if value.content is None
                else {"content": bytes_to_json(value.content)}
            ),
        }
    elif value.kind == "close":
        return {
            "kind": "close",
            "path": value.path,
        }
    elif value.kind == "writeText":
        return {
            "kind": "writeText",
            "path": value.path,
            "content": value.content,
        }
    elif value.kind == "writeBytes":
        return {
            "kind": "writeBytes",
            "path": value.path,
            "content": bytes_to_json(value.content),
        }
    elif value.kind == "remove":
        return {
            "kind": "remove",
            "path": value.path,
        }
    elif value.kind == "move":
        return {
            "kind": "move",
            "from": value.from_,
            "to": value.to,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_file_operation(value: Json) -> FileOperation:
    """Return one FileOperation from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "openText":
        return FileOperationOpenText(
            path=json_string(json_field(object_, "path")),
            uri=destack._generated.source.file.path.uri.from_json_uri(
                json_field(object_, "uri")
            ),
            version=json_int(json_field(object_, "version")),
            content=json_string(json_field(object_, "content")),
        )
    elif kind == "openBytes":
        return FileOperationOpenBytes(
            path=json_string(json_field(object_, "path")),
            uri=destack._generated.source.file.path.uri.from_json_uri(
                json_field(object_, "uri")
            ),
            version=json_int(json_field(object_, "version")),
            content=bytes_from_json(json_field(object_, "content")),
        )
    elif kind == "changeText":
        return FileOperationChangeText(
            path=json_string(json_field(object_, "path")),
            uri=destack._generated.source.file.path.uri.from_json_uri(
                json_field(object_, "uri")
            ),
            version=json_int(json_field(object_, "version")),
            content=json_string(json_field(object_, "content")),
        )
    elif kind == "changeBytes":
        return FileOperationChangeBytes(
            path=json_string(json_field(object_, "path")),
            uri=destack._generated.source.file.path.uri.from_json_uri(
                json_field(object_, "uri")
            ),
            version=json_int(json_field(object_, "version")),
            content=bytes_from_json(json_field(object_, "content")),
        )
    elif kind == "patchText":
        return FileOperationPatchText(
            path=json_string(json_field(object_, "path")),
            uri=destack._generated.source.file.path.uri.from_json_uri(
                json_field(object_, "uri")
            ),
            version=json_int(json_field(object_, "version")),
            changes=[
                destack._generated.source.edit.text.from_json_text_change(item_0)
                for item_0 in json_array(json_field(object_, "changes"))
            ],
        )
    elif kind == "saveText":
        return FileOperationSaveText(
            path=json_string(json_field(object_, "path")),
            content=json_optional(object_, "content", lambda value: json_string(value)),
        )
    elif kind == "saveBytes":
        return FileOperationSaveBytes(
            path=json_string(json_field(object_, "path")),
            content=json_optional(
                object_, "content", lambda value: bytes_from_json(value)
            ),
        )
    elif kind == "close":
        return FileOperationClose(
            path=json_string(json_field(object_, "path")),
        )
    elif kind == "writeText":
        return FileOperationWriteText(
            path=json_string(json_field(object_, "path")),
            content=json_string(json_field(object_, "content")),
        )
    elif kind == "writeBytes":
        return FileOperationWriteBytes(
            path=json_string(json_field(object_, "path")),
            content=bytes_from_json(json_field(object_, "content")),
        )
    elif kind == "remove":
        return FileOperationRemove(
            path=json_string(json_field(object_, "path")),
        )
    elif kind == "move":
        return FileOperationMove(
            from_=json_string(json_field(object_, "from")),
            to=json_string(json_field(object_, "to")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class FileUpdate:
    """File update emitted by the workspace."""

    # updated module id when known
    module_id: destack._generated.source.file.model.module.ModuleId | None
    # updated file id
    file_id: destack._generated.source.file.model.file.FileId
    # diagnostic uri for this update
    diagnostic_uri: destack._generated.source.file.path.uri.Uri
    # protocol file version for diagnostics when the file is open
    diagnostic_version: int | None
    # updated file image when the file still exists
    file: FileImage | None
    # whether this update removed the file
    is_removed: bool
    # the coarse change kind for this file
    kind: UpdateKind
    # diagnostics for this file
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_update(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FileUpdate:
        """Decode one FileUpdate."""
        return decode_file_update(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_update(self)

    @classmethod
    def from_json(cls, value: Json) -> FileUpdate:
        """Return one FileUpdate from one JSON value."""
        return from_json_file_update(value)


def encode_file_update(writer: BinaryWriter, value: FileUpdate) -> None:
    """Encode one FileUpdate."""
    if value.module_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module_id
        )
    destack._generated.source.file.model.file.encode_file_id(writer, value.file_id)
    destack._generated.source.file.path.uri.encode_uri(writer, value.diagnostic_uri)
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
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )


def decode_file_update(reader: BinaryReader) -> FileUpdate:
    """Decode one FileUpdate."""
    module_id = reader.read_option(
        lambda: destack._generated.source.file.model.module.decode_module_id(reader)
    )
    file_id = destack._generated.source.file.model.file.decode_file_id(reader)
    diagnostic_uri = destack._generated.source.file.path.uri.decode_uri(reader)
    diagnostic_version = reader.read_option(lambda: reader.read_signed_number())
    file = reader.read_option(lambda: decode_file_image(reader))
    is_removed = reader.read_bool()
    kind = decode_update_kind(reader)
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]

    return FileUpdate(
        module_id=module_id,
        file_id=file_id,
        diagnostic_uri=diagnostic_uri,
        diagnostic_version=diagnostic_version,
        file=file,
        is_removed=is_removed,
        kind=kind,
        diagnostics=diagnostics,
    )


def to_json_file_update(value: FileUpdate) -> Json:
    """Return one JSON value for one FileUpdate."""
    return {
        **(
            {}
            if value.module_id is None
            else {
                "moduleId": destack._generated.source.file.model.module.to_json_module_id(
                    value.module_id
                )
            }
        ),
        "fileId": destack._generated.source.file.model.file.to_json_file_id(
            value.file_id
        ),
        "diagnosticUri": destack._generated.source.file.path.uri.to_json_uri(
            value.diagnostic_uri
        ),
        **(
            {}
            if value.diagnostic_version is None
            else {"diagnosticVersion": value.diagnostic_version}
        ),
        **({} if value.file is None else {"file": to_json_file_image(value.file)}),
        "isRemoved": value.is_removed,
        "kind": to_json_update_kind(value.kind),
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
    }


def from_json_file_update(value: Json) -> FileUpdate:
    """Return one FileUpdate from one JSON value."""
    object_ = json_object(value)

    return FileUpdate(
        module_id=json_optional(
            object_,
            "moduleId",
            lambda value: (
                destack._generated.source.file.model.module.from_json_module_id(value)
            ),
        ),
        file_id=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "fileId")
        ),
        diagnostic_uri=destack._generated.source.file.path.uri.from_json_uri(
            json_field(object_, "diagnosticUri")
        ),
        diagnostic_version=json_optional(
            object_, "diagnosticVersion", lambda value: json_int(value)
        ),
        file=json_optional(object_, "file", lambda value: from_json_file_image(value)),
        is_removed=json_bool(json_field(object_, "isRemoved")),
        kind=from_json_update_kind(json_field(object_, "kind")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
    )


@dataclass(frozen=True, slots=True)
class FileImage:
    """In-memory image for one updated file."""

    # file id in the registry
    id: destack._generated.source.file.model.file.FileId
    # file name
    name: str
    # file uri
    uri: destack._generated.source.file.path.uri.Uri
    # optional file path
    path: str | None
    # file type
    file_type: destack._generated.source.file.model.type.FileType
    # optional text content
    content: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_image(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FileImage:
        """Decode one FileImage."""
        return decode_file_image(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_image(self)

    @classmethod
    def from_json(cls, value: Json) -> FileImage:
        """Return one FileImage from one JSON value."""
        return from_json_file_image(value)


def encode_file_image(writer: BinaryWriter, value: FileImage) -> None:
    """Encode one FileImage."""
    destack._generated.source.file.model.file.encode_file_id(writer, value.id)
    writer.write_string(value.name)
    destack._generated.source.file.path.uri.encode_uri(writer, value.uri)
    if value.path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.path)
    destack._generated.source.file.model.type.encode_file_type(writer, value.file_type)
    if value.content is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.content)


def decode_file_image(reader: BinaryReader) -> FileImage:
    """Decode one FileImage."""
    id = destack._generated.source.file.model.file.decode_file_id(reader)
    name = reader.read_string()
    uri = destack._generated.source.file.path.uri.decode_uri(reader)
    path = reader.read_option(lambda: reader.read_string())
    file_type = destack._generated.source.file.model.type.decode_file_type(reader)
    content = reader.read_option(lambda: reader.read_string())

    return FileImage(
        id=id,
        name=name,
        uri=uri,
        path=path,
        file_type=file_type,
        content=content,
    )


def to_json_file_image(value: FileImage) -> Json:
    """Return one JSON value for one FileImage."""
    return {
        "id": destack._generated.source.file.model.file.to_json_file_id(value.id),
        "name": value.name,
        "uri": destack._generated.source.file.path.uri.to_json_uri(value.uri),
        **({} if value.path is None else {"path": value.path}),
        "fileType": destack._generated.source.file.model.type.to_json_file_type(
            value.file_type
        ),
        **({} if value.content is None else {"content": value.content}),
    }


def from_json_file_image(value: Json) -> FileImage:
    """Return one FileImage from one JSON value."""
    object_ = json_object(value)

    return FileImage(
        id=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "id")
        ),
        name=json_string(json_field(object_, "name")),
        uri=destack._generated.source.file.path.uri.from_json_uri(
            json_field(object_, "uri")
        ),
        path=json_optional(object_, "path", lambda value: json_string(value)),
        file_type=destack._generated.source.file.model.type.from_json_file_type(
            json_field(object_, "fileType")
        ),
        content=json_optional(object_, "content", lambda value: json_string(value)),
    )


"""One coarse kind for a workspace file update."""
UpdateKind: typing.TypeAlias = typing.Literal["source"] | typing.Literal["config"]


def encode_update_kind(writer: BinaryWriter, value: UpdateKind) -> None:
    """Encode one UpdateKind."""
    if value == "source":
        writer.write_unsigned(0)
    elif value == "config":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_update_kind(reader: BinaryReader) -> UpdateKind:
    """Decode one UpdateKind."""
    variant = reader.read_number()

    if variant == 0:
        return "source"
    elif variant == 1:
        return "config"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_update_kind(value: UpdateKind) -> Json:
    """Return one JSON value for one UpdateKind."""
    return value


def from_json_update_kind(value: Json) -> UpdateKind:
    """Return one UpdateKind from one JSON value."""
    variant = json_string(value)

    if variant == "source":
        return "source"
    elif variant == "config":
        return "config"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "FileOperation",
    "encode_file_operation",
    "decode_file_operation",
    "to_json_file_operation",
    "from_json_file_operation",
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
    "to_json_file_update",
    "from_json_file_update",
    "FileImage",
    "encode_file_image",
    "decode_file_image",
    "to_json_file_image",
    "from_json_file_image",
    "UpdateKind",
    "encode_update_kind",
    "decode_update_kind",
    "to_json_update_kind",
    "from_json_update_kind",
]
