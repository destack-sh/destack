# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)


@dataclass(frozen=True, slots=True)
class RuntimeDiagnosticOptions:
    """Runtime diagnostics configuration."""

    # minimum diagnostic level recorded by the runtime
    level: RuntimeDiagnosticLevel
    # maximum number of diagnostic entries retained in the runtime ring buffer
    capacity: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_runtime_diagnostic_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeDiagnosticOptions:
        """Decode one RuntimeDiagnosticOptions."""
        return decode_runtime_diagnostic_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_runtime_diagnostic_options(self)

    @classmethod
    def from_json(cls, value: Json) -> RuntimeDiagnosticOptions:
        """Return one RuntimeDiagnosticOptions from one JSON value."""
        return from_json_runtime_diagnostic_options(value)


def encode_runtime_diagnostic_options(
    writer: BinaryWriter, value: RuntimeDiagnosticOptions
) -> None:
    """Encode one RuntimeDiagnosticOptions."""
    encode_runtime_diagnostic_level(writer, value.level)
    if value.capacity is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.capacity)


def decode_runtime_diagnostic_options(reader: BinaryReader) -> RuntimeDiagnosticOptions:
    """Decode one RuntimeDiagnosticOptions."""
    level = decode_runtime_diagnostic_level(reader)
    capacity = reader.read_option(lambda: reader.read_number())

    return RuntimeDiagnosticOptions(
        level=level,
        capacity=capacity,
    )


def to_json_runtime_diagnostic_options(value: RuntimeDiagnosticOptions) -> Json:
    """Return one JSON value for one RuntimeDiagnosticOptions."""
    return {
        "level": to_json_runtime_diagnostic_level(value.level),
        **({} if value.capacity is None else {"capacity": value.capacity}),
    }


def from_json_runtime_diagnostic_options(value: Json) -> RuntimeDiagnosticOptions:
    """Return one RuntimeDiagnosticOptions from one JSON value."""
    object_ = json_object(value)

    return RuntimeDiagnosticOptions(
        level=from_json_runtime_diagnostic_level(json_field(object_, "level")),
        capacity=json_optional(object_, "capacity", lambda value: json_int(value)),
    )


"""Runtime diagnostic verbosity."""
RuntimeDiagnosticLevel: typing.TypeAlias = (
    typing.Literal["off"]
    | typing.Literal["error"]
    | typing.Literal["warn"]
    | typing.Literal["info"]
    | typing.Literal["debug"]
    | typing.Literal["trace"]
)


def encode_runtime_diagnostic_level(
    writer: BinaryWriter, value: RuntimeDiagnosticLevel
) -> None:
    """Encode one RuntimeDiagnosticLevel."""
    if value == "off":
        writer.write_unsigned(0)
    elif value == "error":
        writer.write_unsigned(1)
    elif value == "warn":
        writer.write_unsigned(2)
    elif value == "info":
        writer.write_unsigned(3)
    elif value == "debug":
        writer.write_unsigned(4)
    elif value == "trace":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_runtime_diagnostic_level(reader: BinaryReader) -> RuntimeDiagnosticLevel:
    """Decode one RuntimeDiagnosticLevel."""
    variant = reader.read_number()

    if variant == 0:
        return "off"
    elif variant == 1:
        return "error"
    elif variant == 2:
        return "warn"
    elif variant == 3:
        return "info"
    elif variant == 4:
        return "debug"
    elif variant == 5:
        return "trace"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_runtime_diagnostic_level(value: RuntimeDiagnosticLevel) -> Json:
    """Return one JSON value for one RuntimeDiagnosticLevel."""
    return value


def from_json_runtime_diagnostic_level(value: Json) -> RuntimeDiagnosticLevel:
    """Return one RuntimeDiagnosticLevel from one JSON value."""
    variant = json_string(value)

    if variant == "off":
        return "off"
    elif variant == "error":
        return "error"
    elif variant == "warn":
        return "warn"
    elif variant == "info":
        return "info"
    elif variant == "debug":
        return "debug"
    elif variant == "trace":
        return "trace"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "RuntimeDiagnosticOptions",
    "encode_runtime_diagnostic_options",
    "decode_runtime_diagnostic_options",
    "to_json_runtime_diagnostic_options",
    "from_json_runtime_diagnostic_options",
    "RuntimeDiagnosticLevel",
    "encode_runtime_diagnostic_level",
    "decode_runtime_diagnostic_level",
    "to_json_runtime_diagnostic_level",
    "from_json_runtime_diagnostic_level",
]
