# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.repository.revision
import destack._generated.protocol.source.file.model.target
import destack._generated.protocol.source.file.model.type

if TYPE_CHECKING:
    from destack._generated.protocol.repository.revision import (
        Revision,
    )

    from destack._generated.protocol.source.file.model.target import (
        TargetId,
    )

    from destack._generated.protocol.source.file.model.type import (
        FileType,
    )


@dataclass(frozen=True, slots=True)
class CommandRevisionCurrent:
    """Execute from the current root revision."""

    kind: Literal["current"] = "current"


@dataclass(frozen=True, slots=True)
class CommandRevisionExact:
    """Execute only if the root is still at this revision."""

    exact: Revision
    kind: Literal["exact"] = "exact"


"""Revision selection for command execution."""
CommandRevision: TypeAlias = CommandRevisionCurrent | CommandRevisionExact


def encode_command_revision(writer: Writer, value: CommandRevision) -> None:
    if value.kind == "current":
        writer.write_unsigned(0)
    elif value.kind == "exact":
        writer.write_unsigned(1)
        destack._generated.protocol.repository.revision.encode_revision(
            writer, value.exact
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_command_revision(reader: Reader) -> CommandRevision:
    variant = reader.read_number()

    if variant == 0:
        return CommandRevisionCurrent()
    elif variant == 1:
        return CommandRevisionExact(
            exact=destack._generated.protocol.repository.revision.decode_revision(
                reader
            )
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class CommandInputFile:
    """A file path input."""

    path: str
    kind: Literal["file"] = "file"


@dataclass(frozen=True, slots=True)
class CommandInputInline:
    """Inline source input."""

    """Input label."""
    name: str
    """Inline content."""
    content: str
    """Explicit file type."""
    file_type: FileType
    kind: Literal["inline"] = "inline"


@dataclass(frozen=True, slots=True)
class CommandInputStdin:
    """Stdin source input."""

    """Input label."""
    name: str
    """Stdin content."""
    content: str
    """Explicit file type."""
    file_type: FileType
    kind: Literal["stdin"] = "stdin"


"""Command input sources."""
CommandInput: TypeAlias = CommandInputFile | CommandInputInline | CommandInputStdin


def encode_command_input(writer: Writer, value: CommandInput) -> None:
    if value.kind == "file":
        writer.write_unsigned(0)
        writer.write_string(value.path)
    elif value.kind == "inline":
        writer.write_unsigned(1)
        writer.write_string(value.name)
        writer.write_string(value.content)
        destack._generated.protocol.source.file.model.type.encode_file_type(
            writer, value.file_type
        )
    elif value.kind == "stdin":
        writer.write_unsigned(2)
        writer.write_string(value.name)
        writer.write_string(value.content)
        destack._generated.protocol.source.file.model.type.encode_file_type(
            writer, value.file_type
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_command_input(reader: Reader) -> CommandInput:
    variant = reader.read_number()

    if variant == 0:
        field_0 = reader.read_string()

        return CommandInputFile(
            path=field_0,
        )
    elif variant == 1:
        field_0 = reader.read_string()
        field_1 = reader.read_string()
        field_2 = destack._generated.protocol.source.file.model.type.decode_file_type(
            reader
        )

        return CommandInputInline(
            name=field_0,
            content=field_1,
            file_type=field_2,
        )
    elif variant == 2:
        field_0 = reader.read_string()
        field_1 = reader.read_string()
        field_2 = destack._generated.protocol.source.file.model.type.decode_file_type(
            reader
        )

        return CommandInputStdin(
            name=field_0,
            content=field_1,
            file_type=field_2,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class CommandTargetOverrides:
    """Target overrides for command execution."""

    """Output directory override."""
    out_dir: str | None
    """Output file override."""
    out_file: str | None


def encode_command_target_overrides(
    writer: Writer, value: CommandTargetOverrides
) -> None:
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


def decode_command_target_overrides(reader: Reader) -> CommandTargetOverrides:
    field_0 = reader.read_option(lambda: reader.read_string())
    field_1 = reader.read_option(lambda: reader.read_string())

    return CommandTargetOverrides(
        out_dir=field_0,
        out_file=field_1,
    )


@dataclass(frozen=True, slots=True)
class CommandEnvVar:
    """Environment variable override for commands."""

    """Environment variable name."""
    key: str
    """Environment variable value."""
    value: str


def encode_command_env_var(writer: Writer, value: CommandEnvVar) -> None:
    writer.write_string(value.key)
    writer.write_string(value.value)


def decode_command_env_var(reader: Reader) -> CommandEnvVar:
    field_0 = reader.read_string()
    field_1 = reader.read_string()

    return CommandEnvVar(
        key=field_0,
        value=field_1,
    )


@dataclass(frozen=True, slots=True)
class ManifestOverride:
    """Manifest override applied to one command invocation."""

    """Manifest path, such as `compiler.target`."""
    path: str
    """Override payload value."""
    value: JsonValue


def encode_manifest_override(writer: Writer, value: ManifestOverride) -> None:
    writer.write_string(value.path)
    encode_json_value(writer, value.value)


def decode_manifest_override(reader: Reader) -> ManifestOverride:
    field_0 = reader.read_string()
    field_1 = decode_json_value(reader)

    return ManifestOverride(
        path=field_0,
        value=field_1,
    )


@dataclass(frozen=True, slots=True)
class JsonValueNull:
    """Null value."""

    kind: Literal["null"] = "null"


@dataclass(frozen=True, slots=True)
class JsonValueBool:
    """Boolean value."""

    bool: bool
    kind: Literal["bool"] = "bool"


@dataclass(frozen=True, slots=True)
class JsonValueI64:
    """Signed integer value."""

    i64: int
    kind: Literal["i64"] = "i64"


@dataclass(frozen=True, slots=True)
class JsonValueU64:
    """Unsigned integer value."""

    u64: int
    kind: Literal["u64"] = "u64"


@dataclass(frozen=True, slots=True)
class JsonValueI128:
    """Wide signed integer value."""

    i128: int
    kind: Literal["i128"] = "i128"


@dataclass(frozen=True, slots=True)
class JsonValueU128:
    """Wide unsigned integer value."""

    u128: int
    kind: Literal["u128"] = "u128"


@dataclass(frozen=True, slots=True)
class JsonValueF64:
    """Floating point value."""

    f64: float
    kind: Literal["f64"] = "f64"


@dataclass(frozen=True, slots=True)
class JsonValueString:
    """String value."""

    string: str
    kind: Literal["string"] = "string"


@dataclass(frozen=True, slots=True)
class JsonValueArray:
    """Array value."""

    array: Sequence[JsonValue]
    kind: Literal["array"] = "array"


@dataclass(frozen=True, slots=True)
class JsonValueObject:
    """Object entries in source order."""

    object: Sequence[tuple[str, JsonValue]]
    kind: Literal["object"] = "object"


"""JSON-compatible value carried by command protocol messages."""
JsonValue: TypeAlias = (
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


def encode_json_value(writer: Writer, value: JsonValue) -> None:
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
        for item_0 in value.array:
            encode_json_value(writer, item_0)
    elif value.kind == "object":
        writer.write_unsigned(9)
        writer.write_unsigned(len(value.object))
        for item_0 in value.object:
            writer.write_string(item_0[0])
            encode_json_value(writer, item_0[1])
    else:
        raise SerdeError("unknown enum variant")


def decode_json_value(reader: Reader) -> JsonValue:
    variant = reader.read_number()

    if variant == 0:
        return JsonValueNull()
    elif variant == 1:
        return JsonValueBool(bool=reader.read_bool())
    elif variant == 2:
        return JsonValueI64(i64=reader.read_signed_number())
    elif variant == 3:
        return JsonValueU64(u64=reader.read_number())
    elif variant == 4:
        return JsonValueI128(i128=reader.read_signed_number())
    elif variant == 5:
        return JsonValueU128(u128=reader.read_unsigned())
    elif variant == 6:
        return JsonValueF64(f64=reader.read_f64())
    elif variant == 7:
        return JsonValueString(string=reader.read_string())
    elif variant == 8:
        return JsonValueArray(
            array=[decode_json_value(reader) for _ in range(reader.read_number())]
        )
    elif variant == 9:
        return JsonValueObject(
            object=[
                (
                    reader.read_string(),
                    decode_json_value(reader),
                )
                for _ in range(reader.read_number())
            ]
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class CommandOutputChunk:
    """Output chunk from command execution."""

    """Output stream kind."""
    stream: OutputStream
    """Output bytes."""
    bytes: bytes | bytearray | Sequence[int]


def encode_command_output_chunk(writer: Writer, value: CommandOutputChunk) -> None:
    encode_output_stream(writer, value.stream)
    writer.write_byte_slice(value.bytes)


def decode_command_output_chunk(reader: Reader) -> CommandOutputChunk:
    field_0 = decode_output_stream(reader)
    field_1 = reader.read_byte_slice()

    return CommandOutputChunk(
        stream=field_0,
        bytes=field_1,
    )


"""Command output stream kind."""
OutputStream: TypeAlias = Literal["stdout"] | Literal["stderr"]


def encode_output_stream(writer: Writer, value: OutputStream) -> None:
    if value == "stdout":
        writer.write_unsigned(0)
    elif value == "stderr":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_output_stream(reader: Reader) -> OutputStream:
    variant = reader.read_number()

    if variant == 0:
        return "stdout"
    elif variant == 1:
        return "stderr"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class CommandOutputFile:
    """Generated output file produced by a command."""

    """Output id."""
    id: int
    """Target id."""
    target: TargetId
    """File type for the output."""
    file_type: FileType
    """Output path for the generated file."""
    path: str
    """Size in bytes."""
    size_bytes: int
    """Optional content hash."""
    content_hash: int | None


def encode_command_output_file(writer: Writer, value: CommandOutputFile) -> None:
    writer.write_unsigned(value.id)
    destack._generated.protocol.source.file.model.target.encode_target_id(
        writer, value.target
    )
    destack._generated.protocol.source.file.model.type.encode_file_type(
        writer, value.file_type
    )
    writer.write_string(value.path)
    writer.write_unsigned(value.size_bytes)
    if value.content_hash is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.content_hash)


def decode_command_output_file(reader: Reader) -> CommandOutputFile:
    field_0 = reader.read_number()
    field_1 = destack._generated.protocol.source.file.model.target.decode_target_id(
        reader
    )
    field_2 = destack._generated.protocol.source.file.model.type.decode_file_type(
        reader
    )
    field_3 = reader.read_string()
    field_4 = reader.read_number()
    field_5 = reader.read_option(lambda: reader.read_number())

    return CommandOutputFile(
        id=field_0,
        target=field_1,
        file_type=field_2,
        path=field_3,
        size_bytes=field_4,
        content_hash=field_5,
    )


@dataclass(frozen=True, slots=True)
class CommandMessagePayload:
    """Standard payload for unimplemented command responses."""

    """Message describing the command response."""
    message: str
    """Whether the command is implemented."""
    implemented: bool


def encode_command_message_payload(
    writer: Writer, value: CommandMessagePayload
) -> None:
    writer.write_string(value.message)
    writer.write_bool(value.implemented)


def decode_command_message_payload(reader: Reader) -> CommandMessagePayload:
    field_0 = reader.read_string()
    field_1 = reader.read_bool()

    return CommandMessagePayload(
        message=field_0,
        implemented=field_1,
    )


@dataclass(frozen=True, slots=True)
class ProgressEvent:
    """Progress event payload."""

    """Identifier for the ongoing task."""
    task: str
    """Stage message for the task."""
    message: str | None
    """Optional progress percent between 0 and 100."""
    percent: int | None
    """Whether this event signals completion."""
    done: bool


def encode_progress_event(writer: Writer, value: ProgressEvent) -> None:
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


def decode_progress_event(reader: Reader) -> ProgressEvent:
    field_0 = reader.read_string()
    field_1 = reader.read_option(lambda: reader.read_string())
    field_2 = reader.read_option(lambda: reader.read_byte())
    field_3 = reader.read_bool()

    return ProgressEvent(
        task=field_0,
        message=field_1,
        percent=field_2,
        done=field_3,
    )


__all__ = [
    "CommandRevision",
    "encode_command_revision",
    "decode_command_revision",
    "CommandRevisionCurrent",
    "CommandRevisionExact",
    "CommandInput",
    "encode_command_input",
    "decode_command_input",
    "CommandInputFile",
    "CommandInputInline",
    "CommandInputStdin",
    "CommandTargetOverrides",
    "encode_command_target_overrides",
    "decode_command_target_overrides",
    "CommandEnvVar",
    "encode_command_env_var",
    "decode_command_env_var",
    "ManifestOverride",
    "encode_manifest_override",
    "decode_manifest_override",
    "JsonValue",
    "encode_json_value",
    "decode_json_value",
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
    "OutputStream",
    "encode_output_stream",
    "decode_output_stream",
    "CommandOutputFile",
    "encode_command_output_file",
    "decode_command_output_file",
    "CommandMessagePayload",
    "encode_command_message_payload",
    "decode_command_message_payload",
    "ProgressEvent",
    "encode_progress_event",
    "decode_progress_event",
]
