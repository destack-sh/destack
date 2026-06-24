# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_string,
)

"""The level of a diagnostic."""
DiagnosticSeverity: typing.TypeAlias = (
    typing.Literal["note"] | typing.Literal["warning"] | typing.Literal["error"]
)


def encode_diagnostic_severity(writer: BinaryWriter, value: DiagnosticSeverity) -> None:
    """Encode one DiagnosticSeverity."""
    if value == "note":
        writer.write_unsigned(0)
    elif value == "warning":
        writer.write_unsigned(1)
    elif value == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_diagnostic_severity(reader: BinaryReader) -> DiagnosticSeverity:
    """Decode one DiagnosticSeverity."""
    variant = reader.read_number()

    if variant == 0:
        return "note"
    elif variant == 1:
        return "warning"
    elif variant == 2:
        return "error"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_diagnostic_severity(value: DiagnosticSeverity) -> Json:
    """Return one JSON value for one DiagnosticSeverity."""
    return value


def from_json_diagnostic_severity(value: Json) -> DiagnosticSeverity:
    """Return one DiagnosticSeverity from one JSON value."""
    variant = json_string(value)

    if variant == "note":
        return "note"
    elif variant == "warning":
        return "warning"
    elif variant == "error":
        return "error"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Extra semantic tag for a diagnostic."""
DiagnosticTag: typing.TypeAlias = (
    typing.Literal["unnecessary"] | typing.Literal["deprecated"]
)


def encode_diagnostic_tag(writer: BinaryWriter, value: DiagnosticTag) -> None:
    """Encode one DiagnosticTag."""
    if value == "unnecessary":
        writer.write_unsigned(0)
    elif value == "deprecated":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_diagnostic_tag(reader: BinaryReader) -> DiagnosticTag:
    """Decode one DiagnosticTag."""
    variant = reader.read_number()

    if variant == 0:
        return "unnecessary"
    elif variant == 1:
        return "deprecated"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_diagnostic_tag(value: DiagnosticTag) -> Json:
    """Return one JSON value for one DiagnosticTag."""
    return value


def from_json_diagnostic_tag(value: Json) -> DiagnosticTag:
    """Return one DiagnosticTag from one JSON value."""
    variant = json_string(value)

    if variant == "unnecessary":
        return "unnecessary"
    elif variant == "deprecated":
        return "deprecated"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "DiagnosticSeverity",
    "encode_diagnostic_severity",
    "decode_diagnostic_severity",
    "to_json_diagnostic_severity",
    "from_json_diagnostic_severity",
    "DiagnosticTag",
    "encode_diagnostic_tag",
    "decode_diagnostic_tag",
    "to_json_diagnostic_tag",
    "from_json_diagnostic_tag",
]
