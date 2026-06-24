# generated bridge target, do not edit

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
    json_array_length,
    json_bool,
    json_field,
    json_int,
    json_number,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.repository.revision
import destack._generated.source.file.model.target
import destack._generated.source.file.model.type


@dataclass(frozen=True, slots=True)
class CommandRevisionCurrent:
    """Execute from the current root revision."""

    kind: typing.Literal["current"] = "current"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_command_revision(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_command_revision(self)


@dataclass(frozen=True, slots=True)
class CommandRevisionExact:
    """Execute only if the root is still at this revision."""

    exact: destack._generated.repository.revision.Revision
    kind: typing.Literal["exact"] = "exact"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_command_revision(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_command_revision(self)


"""Revision selection for command execution."""
CommandRevision: typing.TypeAlias = CommandRevisionCurrent | CommandRevisionExact


def encode_command_revision(writer: BinaryWriter, value: CommandRevision) -> None:
    """Encode one CommandRevision."""
    if value.kind == "current":
        writer.write_unsigned(0)
    elif value.kind == "exact":
        writer.write_unsigned(1)
        destack._generated.repository.revision.encode_revision(writer, value.exact)
    else:
        raise SerdeError("unknown enum variant")


def decode_command_revision(reader: BinaryReader) -> CommandRevision:
    """Decode one CommandRevision."""
    variant = reader.read_number()

    if variant == 0:
        return CommandRevisionCurrent()
    elif variant == 1:
        exact = destack._generated.repository.revision.decode_revision(reader)

        return CommandRevisionExact(exact=exact)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_command_revision(value: CommandRevision) -> Json:
    """Return one JSON value for one CommandRevision."""
    if value.kind == "current":
        return {
            "kind": "current",
        }
    elif value.kind == "exact":
        return {
            "kind": "exact",
            "exact": destack._generated.repository.revision.to_json_revision(
                value.exact
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_command_revision(value: Json) -> CommandRevision:
    """Return one CommandRevision from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "current":
        return CommandRevisionCurrent()
    elif kind == "exact":
        return CommandRevisionExact(
            exact=destack._generated.repository.revision.from_json_revision(
                json_field(object_, "exact")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class CommandInputFile:
    """A file path input."""

    path: str
    kind: typing.Literal["file"] = "file"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_command_input(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_command_input(self)


@dataclass(frozen=True, slots=True)
class CommandInputInline:
    """Inline source input."""

    # input label
    name: str
    # inline content
    content: str
    # explicit file type
    file_type: destack._generated.source.file.model.type.FileType
    kind: typing.Literal["inline"] = "inline"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_command_input(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_command_input(self)


@dataclass(frozen=True, slots=True)
class CommandInputStdin:
    """Stdin source input."""

    # input label
    name: str
    # stdin content
    content: str
    # explicit file type
    file_type: destack._generated.source.file.model.type.FileType
    kind: typing.Literal["stdin"] = "stdin"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_command_input(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_command_input(self)


"""Command input sources."""
CommandInput: typing.TypeAlias = (
    CommandInputFile | CommandInputInline | CommandInputStdin
)


def encode_command_input(writer: BinaryWriter, value: CommandInput) -> None:
    """Encode one CommandInput."""
    if value.kind == "file":
        writer.write_unsigned(0)
        writer.write_string(value.path)
    elif value.kind == "inline":
        writer.write_unsigned(1)
        writer.write_string(value.name)
        writer.write_string(value.content)
        destack._generated.source.file.model.type.encode_file_type(
            writer, value.file_type
        )
    elif value.kind == "stdin":
        writer.write_unsigned(2)
        writer.write_string(value.name)
        writer.write_string(value.content)
        destack._generated.source.file.model.type.encode_file_type(
            writer, value.file_type
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_command_input(reader: BinaryReader) -> CommandInput:
    """Decode one CommandInput."""
    variant = reader.read_number()

    if variant == 0:
        path = reader.read_string()

        return CommandInputFile(
            path=path,
        )
    elif variant == 1:
        name = reader.read_string()
        content = reader.read_string()
        file_type = destack._generated.source.file.model.type.decode_file_type(reader)

        return CommandInputInline(
            name=name,
            content=content,
            file_type=file_type,
        )
    elif variant == 2:
        name = reader.read_string()
        content = reader.read_string()
        file_type = destack._generated.source.file.model.type.decode_file_type(reader)

        return CommandInputStdin(
            name=name,
            content=content,
            file_type=file_type,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_command_input(value: CommandInput) -> Json:
    """Return one JSON value for one CommandInput."""
    if value.kind == "file":
        return {
            "kind": "file",
            "path": value.path,
        }
    elif value.kind == "inline":
        return {
            "kind": "inline",
            "name": value.name,
            "content": value.content,
            "fileType": destack._generated.source.file.model.type.to_json_file_type(
                value.file_type
            ),
        }
    elif value.kind == "stdin":
        return {
            "kind": "stdin",
            "name": value.name,
            "content": value.content,
            "fileType": destack._generated.source.file.model.type.to_json_file_type(
                value.file_type
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_command_input(value: Json) -> CommandInput:
    """Return one CommandInput from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "file":
        return CommandInputFile(
            path=json_string(json_field(object_, "path")),
        )
    elif kind == "inline":
        return CommandInputInline(
            name=json_string(json_field(object_, "name")),
            content=json_string(json_field(object_, "content")),
            file_type=destack._generated.source.file.model.type.from_json_file_type(
                json_field(object_, "fileType")
            ),
        )
    elif kind == "stdin":
        return CommandInputStdin(
            name=json_string(json_field(object_, "name")),
            content=json_string(json_field(object_, "content")),
            file_type=destack._generated.source.file.model.type.from_json_file_type(
                json_field(object_, "fileType")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class CommandTargetOverrides:
    """Target overrides for command execution."""

    # output directory override
    out_dir: str | None
    # output file override
    out_file: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_command_target_overrides(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CommandTargetOverrides:
        """Decode one CommandTargetOverrides."""
        return decode_command_target_overrides(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_command_target_overrides(self)

    @classmethod
    def from_json(cls, value: Json) -> CommandTargetOverrides:
        """Return one CommandTargetOverrides from one JSON value."""
        return from_json_command_target_overrides(value)


def encode_command_target_overrides(
    writer: BinaryWriter, value: CommandTargetOverrides
) -> None:
    """Encode one CommandTargetOverrides."""
    if value.out_dir is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.out_dir)
    if value.out_file is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.out_file)


def decode_command_target_overrides(reader: BinaryReader) -> CommandTargetOverrides:
    """Decode one CommandTargetOverrides."""
    out_dir = reader.read_option(lambda: reader.read_string())
    out_file = reader.read_option(lambda: reader.read_string())

    return CommandTargetOverrides(
        out_dir=out_dir,
        out_file=out_file,
    )


def to_json_command_target_overrides(value: CommandTargetOverrides) -> Json:
    """Return one JSON value for one CommandTargetOverrides."""
    return {
        **({} if value.out_dir is None else {"outDir": value.out_dir}),
        **({} if value.out_file is None else {"outFile": value.out_file}),
    }


def from_json_command_target_overrides(value: Json) -> CommandTargetOverrides:
    """Return one CommandTargetOverrides from one JSON value."""
    object_ = json_object(value)

    return CommandTargetOverrides(
        out_dir=json_optional(object_, "outDir", lambda value: json_string(value)),
        out_file=json_optional(object_, "outFile", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class CommandEnvVar:
    """Environment variable override for commands."""

    # environment variable name
    key: str
    # environment variable value
    value: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_command_env_var(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CommandEnvVar:
        """Decode one CommandEnvVar."""
        return decode_command_env_var(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_command_env_var(self)

    @classmethod
    def from_json(cls, value: Json) -> CommandEnvVar:
        """Return one CommandEnvVar from one JSON value."""
        return from_json_command_env_var(value)


def encode_command_env_var(writer: BinaryWriter, value: CommandEnvVar) -> None:
    """Encode one CommandEnvVar."""
    writer.write_string(value.key)
    writer.write_string(value.value)


def decode_command_env_var(reader: BinaryReader) -> CommandEnvVar:
    """Decode one CommandEnvVar."""
    key = reader.read_string()
    value_ = reader.read_string()

    return CommandEnvVar(
        key=key,
        value=value_,
    )


def to_json_command_env_var(value: CommandEnvVar) -> Json:
    """Return one JSON value for one CommandEnvVar."""
    return {
        "key": value.key,
        "value": value.value,
    }


def from_json_command_env_var(value: Json) -> CommandEnvVar:
    """Return one CommandEnvVar from one JSON value."""
    object_ = json_object(value)

    return CommandEnvVar(
        key=json_string(json_field(object_, "key")),
        value=json_string(json_field(object_, "value")),
    )


@dataclass(frozen=True, slots=True)
class ManifestOverride:
    """Manifest override applied to one command invocation."""

    # manifest path, such as `compiler.target`
    path: str
    # override payload value
    value: JsonValue

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_manifest_override(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ManifestOverride:
        """Decode one ManifestOverride."""
        return decode_manifest_override(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_manifest_override(self)

    @classmethod
    def from_json(cls, value: Json) -> ManifestOverride:
        """Return one ManifestOverride from one JSON value."""
        return from_json_manifest_override(value)


def encode_manifest_override(writer: BinaryWriter, value: ManifestOverride) -> None:
    """Encode one ManifestOverride."""
    writer.write_string(value.path)
    encode_json_value(writer, value.value)


def decode_manifest_override(reader: BinaryReader) -> ManifestOverride:
    """Decode one ManifestOverride."""
    path = reader.read_string()
    value_ = decode_json_value(reader)

    return ManifestOverride(
        path=path,
        value=value_,
    )


def to_json_manifest_override(value: ManifestOverride) -> Json:
    """Return one JSON value for one ManifestOverride."""
    return {
        "path": value.path,
        "value": to_json_json_value(value.value),
    }


def from_json_manifest_override(value: Json) -> ManifestOverride:
    """Return one ManifestOverride from one JSON value."""
    object_ = json_object(value)

    return ManifestOverride(
        path=json_string(json_field(object_, "path")),
        value=from_json_json_value(json_field(object_, "value")),
    )


@dataclass(frozen=True, slots=True)
class JsonValueNull:
    """Null value."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_json_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_json_value(self)


@dataclass(frozen=True, slots=True)
class JsonValueBool:
    """Boolean value."""

    bool: bool
    kind: typing.Literal["bool"] = "bool"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_json_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_json_value(self)


@dataclass(frozen=True, slots=True)
class JsonValueI64:
    """Signed integer value."""

    i64: int
    kind: typing.Literal["i64"] = "i64"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_json_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_json_value(self)


@dataclass(frozen=True, slots=True)
class JsonValueU64:
    """Unsigned integer value."""

    u64: int
    kind: typing.Literal["u64"] = "u64"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_json_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_json_value(self)


@dataclass(frozen=True, slots=True)
class JsonValueI128:
    """Wide signed integer value."""

    i128: int
    kind: typing.Literal["i128"] = "i128"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_json_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_json_value(self)


@dataclass(frozen=True, slots=True)
class JsonValueU128:
    """Wide unsigned integer value."""

    u128: int
    kind: typing.Literal["u128"] = "u128"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_json_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_json_value(self)


@dataclass(frozen=True, slots=True)
class JsonValueF64:
    """Floating point value."""

    f64: float
    kind: typing.Literal["f64"] = "f64"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_json_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_json_value(self)


@dataclass(frozen=True, slots=True)
class JsonValueString:
    """String value."""

    string: str
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_json_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_json_value(self)


@dataclass(frozen=True, slots=True)
class JsonValueArray:
    """Array value."""

    array: Sequence[JsonValue]
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_json_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_json_value(self)


@dataclass(frozen=True, slots=True)
class JsonValueObject:
    """Object entries in source order."""

    object: Sequence[tuple[str, JsonValue]]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_json_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_json_value(self)


"""JSON-compatible value carried by command protocol messages."""
JsonValue: typing.TypeAlias = (
    JsonValueNull
    | JsonValueBool
    | JsonValueI64
    | JsonValueU64
    | JsonValueI128
    | JsonValueU128
    | JsonValueF64
    | JsonValueString
    | JsonValueArray
    | JsonValueObject
)


def encode_json_value(writer: BinaryWriter, value: JsonValue) -> None:
    """Encode one JsonValue."""
    if value.kind == "null":
        writer.write_unsigned(0)
    elif value.kind == "bool":
        writer.write_unsigned(1)
        writer.write_bool(value.bool)
    elif value.kind == "i64":
        writer.write_unsigned(2)
        writer.write_signed(value.i64)
    elif value.kind == "u64":
        writer.write_unsigned(3)
        writer.write_unsigned(value.u64)
    elif value.kind == "i128":
        writer.write_unsigned(4)
        writer.write_signed(value.i128)
    elif value.kind == "u128":
        writer.write_unsigned(5)
        writer.write_unsigned(value.u128)
    elif value.kind == "f64":
        writer.write_unsigned(6)
        writer.write_f64(value.f64)
    elif value.kind == "string":
        writer.write_unsigned(7)
        writer.write_string(value.string)
    elif value.kind == "array":
        writer.write_unsigned(8)
        writer.write_unsigned(len(value.array))
        for item_value_array_0 in value.array:
            encode_json_value(writer, item_value_array_0)
    elif value.kind == "object":
        writer.write_unsigned(9)
        writer.write_unsigned(len(value.object))
        for item_value_object_0 in value.object:
            writer.write_string(item_value_object_0[0])
            encode_json_value(writer, item_value_object_0[1])
    else:
        raise SerdeError("unknown enum variant")


def decode_json_value(reader: BinaryReader) -> JsonValue:
    """Decode one JsonValue."""
    variant = reader.read_number()

    if variant == 0:
        return JsonValueNull()
    elif variant == 1:
        bool = reader.read_bool()

        return JsonValueBool(bool=bool)
    elif variant == 2:
        i64 = reader.read_signed_number()

        return JsonValueI64(i64=i64)
    elif variant == 3:
        u64 = reader.read_number()

        return JsonValueU64(u64=u64)
    elif variant == 4:
        i128 = reader.read_signed_number()

        return JsonValueI128(i128=i128)
    elif variant == 5:
        u128 = reader.read_unsigned()

        return JsonValueU128(u128=u128)
    elif variant == 6:
        f64 = reader.read_f64()

        return JsonValueF64(f64=f64)
    elif variant == 7:
        string = reader.read_string()

        return JsonValueString(string=string)
    elif variant == 8:
        array = [decode_json_value(reader) for _ in range(reader.read_number())]

        return JsonValueArray(array=array)
    elif variant == 9:
        object = [
            (
                reader.read_string(),
                decode_json_value(reader),
            )
            for _ in range(reader.read_number())
        ]

        return JsonValueObject(object=object)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_json_value(value: JsonValue) -> Json:
    """Return one JSON value for one JsonValue."""
    if value.kind == "null":
        return {
            "kind": "null",
        }
    elif value.kind == "bool":
        return {
            "kind": "bool",
            "bool": value.bool,
        }
    elif value.kind == "i64":
        return {
            "kind": "i64",
            "i64": value.i64,
        }
    elif value.kind == "u64":
        return {
            "kind": "u64",
            "u64": value.u64,
        }
    elif value.kind == "i128":
        return {
            "kind": "i128",
            "i128": value.i128,
        }
    elif value.kind == "u128":
        return {
            "kind": "u128",
            "u128": value.u128,
        }
    elif value.kind == "f64":
        return {
            "kind": "f64",
            "f64": value.f64,
        }
    elif value.kind == "string":
        return {
            "kind": "string",
            "string": value.string,
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "array": [to_json_json_value(item_0) for item_0 in value.array],
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "object": [
                [item_0[0], to_json_json_value(item_0[1])] for item_0 in value.object
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_json_value(value: Json) -> JsonValue:
    """Return one JsonValue from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "null":
        return JsonValueNull()
    elif kind == "bool":
        return JsonValueBool(bool=json_bool(json_field(object_, "bool")))
    elif kind == "i64":
        return JsonValueI64(i64=json_int(json_field(object_, "i64")))
    elif kind == "u64":
        return JsonValueU64(u64=json_int(json_field(object_, "u64")))
    elif kind == "i128":
        return JsonValueI128(i128=json_int(json_field(object_, "i128")))
    elif kind == "u128":
        return JsonValueU128(u128=json_int(json_field(object_, "u128")))
    elif kind == "f64":
        return JsonValueF64(f64=json_number(json_field(object_, "f64")))
    elif kind == "string":
        return JsonValueString(string=json_string(json_field(object_, "string")))
    elif kind == "array":
        return JsonValueArray(
            array=[
                from_json_json_value(item_0)
                for item_0 in json_array(json_field(object_, "array"))
            ]
        )
    elif kind == "object":
        return JsonValueObject(
            object=[
                (
                    lambda items: (
                        json_string(items[0]),
                        from_json_json_value(items[1]),
                    )
                )(json_array_length(item_0, 2))
                for item_0 in json_array(json_field(object_, "object"))
            ]
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class CommandOutputChunk:
    """Output chunk from command execution."""

    # output stream kind
    stream: OutputStream
    # output bytes
    bytes: builtins.bytes | bytearray | Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_command_output_chunk(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CommandOutputChunk:
        """Decode one CommandOutputChunk."""
        return decode_command_output_chunk(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_command_output_chunk(self)

    @classmethod
    def from_json(cls, value: Json) -> CommandOutputChunk:
        """Return one CommandOutputChunk from one JSON value."""
        return from_json_command_output_chunk(value)


def encode_command_output_chunk(
    writer: BinaryWriter, value: CommandOutputChunk
) -> None:
    """Encode one CommandOutputChunk."""
    encode_output_stream(writer, value.stream)
    writer.write_byte_slice(value.bytes)


def decode_command_output_chunk(reader: BinaryReader) -> CommandOutputChunk:
    """Decode one CommandOutputChunk."""
    stream = decode_output_stream(reader)
    bytes = reader.read_byte_slice()

    return CommandOutputChunk(
        stream=stream,
        bytes=bytes,
    )


def to_json_command_output_chunk(value: CommandOutputChunk) -> Json:
    """Return one JSON value for one CommandOutputChunk."""
    return {
        "stream": to_json_output_stream(value.stream),
        "bytes": bytes_to_json(value.bytes),
    }


def from_json_command_output_chunk(value: Json) -> CommandOutputChunk:
    """Return one CommandOutputChunk from one JSON value."""
    object_ = json_object(value)

    return CommandOutputChunk(
        stream=from_json_output_stream(json_field(object_, "stream")),
        bytes=bytes_from_json(json_field(object_, "bytes")),
    )


"""Command output stream kind."""
OutputStream: typing.TypeAlias = typing.Literal["stdout"] | typing.Literal["stderr"]


def encode_output_stream(writer: BinaryWriter, value: OutputStream) -> None:
    """Encode one OutputStream."""
    if value == "stdout":
        writer.write_unsigned(0)
    elif value == "stderr":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_output_stream(reader: BinaryReader) -> OutputStream:
    """Decode one OutputStream."""
    variant = reader.read_number()

    if variant == 0:
        return "stdout"
    elif variant == 1:
        return "stderr"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_output_stream(value: OutputStream) -> Json:
    """Return one JSON value for one OutputStream."""
    return value


def from_json_output_stream(value: Json) -> OutputStream:
    """Return one OutputStream from one JSON value."""
    variant = json_string(value)

    if variant == "stdout":
        return "stdout"
    elif variant == "stderr":
        return "stderr"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class CommandOutputFile:
    """Generated output file produced by a command."""

    # output id
    id: int
    # target id
    target: destack._generated.source.file.model.target.TargetId
    # file type for the output
    file_type: destack._generated.source.file.model.type.FileType
    # output path for the generated file
    path: str
    # size in bytes
    size_bytes: int
    # optional content hash
    content_hash: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_command_output_file(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CommandOutputFile:
        """Decode one CommandOutputFile."""
        return decode_command_output_file(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_command_output_file(self)

    @classmethod
    def from_json(cls, value: Json) -> CommandOutputFile:
        """Return one CommandOutputFile from one JSON value."""
        return from_json_command_output_file(value)


def encode_command_output_file(writer: BinaryWriter, value: CommandOutputFile) -> None:
    """Encode one CommandOutputFile."""
    writer.write_unsigned(value.id)
    destack._generated.source.file.model.target.encode_target_id(writer, value.target)
    destack._generated.source.file.model.type.encode_file_type(writer, value.file_type)
    writer.write_string(value.path)
    writer.write_unsigned(value.size_bytes)
    if value.content_hash is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.content_hash)


def decode_command_output_file(reader: BinaryReader) -> CommandOutputFile:
    """Decode one CommandOutputFile."""
    id = reader.read_number()
    target = destack._generated.source.file.model.target.decode_target_id(reader)
    file_type = destack._generated.source.file.model.type.decode_file_type(reader)
    path = reader.read_string()
    size_bytes = reader.read_number()
    content_hash = reader.read_option(lambda: reader.read_number())

    return CommandOutputFile(
        id=id,
        target=target,
        file_type=file_type,
        path=path,
        size_bytes=size_bytes,
        content_hash=content_hash,
    )


def to_json_command_output_file(value: CommandOutputFile) -> Json:
    """Return one JSON value for one CommandOutputFile."""
    return {
        "id": value.id,
        "target": destack._generated.source.file.model.target.to_json_target_id(
            value.target
        ),
        "fileType": destack._generated.source.file.model.type.to_json_file_type(
            value.file_type
        ),
        "path": value.path,
        "sizeBytes": value.size_bytes,
        **({} if value.content_hash is None else {"contentHash": value.content_hash}),
    }


def from_json_command_output_file(value: Json) -> CommandOutputFile:
    """Return one CommandOutputFile from one JSON value."""
    object_ = json_object(value)

    return CommandOutputFile(
        id=json_int(json_field(object_, "id")),
        target=destack._generated.source.file.model.target.from_json_target_id(
            json_field(object_, "target")
        ),
        file_type=destack._generated.source.file.model.type.from_json_file_type(
            json_field(object_, "fileType")
        ),
        path=json_string(json_field(object_, "path")),
        size_bytes=json_int(json_field(object_, "sizeBytes")),
        content_hash=json_optional(
            object_, "contentHash", lambda value: json_int(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class CommandMessagePayload:
    """Standard payload for unimplemented command responses."""

    # message describing the command response
    message: str
    # whether the command is implemented
    implemented: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_command_message_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CommandMessagePayload:
        """Decode one CommandMessagePayload."""
        return decode_command_message_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_command_message_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> CommandMessagePayload:
        """Return one CommandMessagePayload from one JSON value."""
        return from_json_command_message_payload(value)


def encode_command_message_payload(
    writer: BinaryWriter, value: CommandMessagePayload
) -> None:
    """Encode one CommandMessagePayload."""
    writer.write_string(value.message)
    writer.write_bool(value.implemented)


def decode_command_message_payload(reader: BinaryReader) -> CommandMessagePayload:
    """Decode one CommandMessagePayload."""
    message = reader.read_string()
    implemented = reader.read_bool()

    return CommandMessagePayload(
        message=message,
        implemented=implemented,
    )


def to_json_command_message_payload(value: CommandMessagePayload) -> Json:
    """Return one JSON value for one CommandMessagePayload."""
    return {
        "message": value.message,
        "implemented": value.implemented,
    }


def from_json_command_message_payload(value: Json) -> CommandMessagePayload:
    """Return one CommandMessagePayload from one JSON value."""
    object_ = json_object(value)

    return CommandMessagePayload(
        message=json_string(json_field(object_, "message")),
        implemented=json_bool(json_field(object_, "implemented")),
    )


@dataclass(frozen=True, slots=True)
class ProgressEvent:
    """Progress event payload."""

    # identifier for the ongoing task
    task: str
    # stage message for the task
    message: str | None
    # optional progress percent between 0 and 100
    percent: int | None
    # whether this event signals completion
    done: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_progress_event(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgressEvent:
        """Decode one ProgressEvent."""
        return decode_progress_event(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_progress_event(self)

    @classmethod
    def from_json(cls, value: Json) -> ProgressEvent:
        """Return one ProgressEvent from one JSON value."""
        return from_json_progress_event(value)


def encode_progress_event(writer: BinaryWriter, value: ProgressEvent) -> None:
    """Encode one ProgressEvent."""
    writer.write_string(value.task)
    if value.message is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.message)
    if value.percent is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_byte(value.percent)
    writer.write_bool(value.done)


def decode_progress_event(reader: BinaryReader) -> ProgressEvent:
    """Decode one ProgressEvent."""
    task = reader.read_string()
    message = reader.read_option(lambda: reader.read_string())
    percent = reader.read_option(lambda: reader.read_byte())
    done = reader.read_bool()

    return ProgressEvent(
        task=task,
        message=message,
        percent=percent,
        done=done,
    )


def to_json_progress_event(value: ProgressEvent) -> Json:
    """Return one JSON value for one ProgressEvent."""
    return {
        "task": value.task,
        **({} if value.message is None else {"message": value.message}),
        **({} if value.percent is None else {"percent": value.percent}),
        "done": value.done,
    }


def from_json_progress_event(value: Json) -> ProgressEvent:
    """Return one ProgressEvent from one JSON value."""
    object_ = json_object(value)

    return ProgressEvent(
        task=json_string(json_field(object_, "task")),
        message=json_optional(object_, "message", lambda value: json_string(value)),
        percent=json_optional(object_, "percent", lambda value: json_int(value)),
        done=json_bool(json_field(object_, "done")),
    )


__all__ = [
    "CommandRevision",
    "encode_command_revision",
    "decode_command_revision",
    "to_json_command_revision",
    "from_json_command_revision",
    "CommandRevisionCurrent",
    "CommandRevisionExact",
    "CommandInput",
    "encode_command_input",
    "decode_command_input",
    "to_json_command_input",
    "from_json_command_input",
    "CommandInputFile",
    "CommandInputInline",
    "CommandInputStdin",
    "CommandTargetOverrides",
    "encode_command_target_overrides",
    "decode_command_target_overrides",
    "to_json_command_target_overrides",
    "from_json_command_target_overrides",
    "CommandEnvVar",
    "encode_command_env_var",
    "decode_command_env_var",
    "to_json_command_env_var",
    "from_json_command_env_var",
    "ManifestOverride",
    "encode_manifest_override",
    "decode_manifest_override",
    "to_json_manifest_override",
    "from_json_manifest_override",
    "JsonValue",
    "encode_json_value",
    "decode_json_value",
    "to_json_json_value",
    "from_json_json_value",
    "JsonValueNull",
    "JsonValueBool",
    "JsonValueI64",
    "JsonValueU64",
    "JsonValueI128",
    "JsonValueU128",
    "JsonValueF64",
    "JsonValueString",
    "JsonValueArray",
    "JsonValueObject",
    "CommandOutputChunk",
    "encode_command_output_chunk",
    "decode_command_output_chunk",
    "to_json_command_output_chunk",
    "from_json_command_output_chunk",
    "OutputStream",
    "encode_output_stream",
    "decode_output_stream",
    "to_json_output_stream",
    "from_json_output_stream",
    "CommandOutputFile",
    "encode_command_output_file",
    "decode_command_output_file",
    "to_json_command_output_file",
    "from_json_command_output_file",
    "CommandMessagePayload",
    "encode_command_message_payload",
    "decode_command_message_payload",
    "to_json_command_message_payload",
    "from_json_command_message_payload",
    "ProgressEvent",
    "encode_progress_event",
    "decode_progress_event",
    "to_json_progress_event",
    "from_json_progress_event",
]
