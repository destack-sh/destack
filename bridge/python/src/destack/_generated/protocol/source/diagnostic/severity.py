# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

"""The level of a diagnostic."""
DiagnosticSeverity: TypeAlias = Literal["note"] | Literal["warning"] | Literal["error"]


def encode_diagnostic_severity(writer: Writer, value: DiagnosticSeverity) -> None:
    if value == "note":
        writer.write_unsigned(0)
    elif value == "warning":
        writer.write_unsigned(1)
    elif value == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_diagnostic_severity(reader: Reader) -> DiagnosticSeverity:
    variant = reader.read_number()

    if variant == 0:
        return "note"
    elif variant == 1:
        return "warning"
    elif variant == 2:
        return "error"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


"""Extra semantic tag for a diagnostic."""
DiagnosticTag: TypeAlias = Literal["unnecessary"] | Literal["deprecated"]


def encode_diagnostic_tag(writer: Writer, value: DiagnosticTag) -> None:
    if value == "unnecessary":
        writer.write_unsigned(0)
    elif value == "deprecated":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_diagnostic_tag(reader: Reader) -> DiagnosticTag:
    variant = reader.read_number()

    if variant == 0:
        return "unnecessary"
    elif variant == 1:
        return "deprecated"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "DiagnosticSeverity",
    "encode_diagnostic_severity",
    "decode_diagnostic_severity",
    "DiagnosticTag",
    "encode_diagnostic_tag",
    "decode_diagnostic_tag",
]
