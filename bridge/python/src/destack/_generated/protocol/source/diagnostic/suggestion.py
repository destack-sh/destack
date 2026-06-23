# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.diagnostic.label
import destack._generated.protocol.source.edit.edit

if TYPE_CHECKING:
    from destack._generated.protocol.source.diagnostic.label import (
        DiagnosticLabel,
    )

    from destack._generated.protocol.source.edit.edit import (
        PatchSet,
    )


@dataclass(frozen=True, slots=True)
class DiagnosticSuggestion:
    """One suggested source change for a diagnostic."""

    """Exact source patches for machine application."""
    patches: PatchSet
    """Source labels to show with the suggestion."""
    labels: Sequence[DiagnosticLabel]
    """The message of the suggestion."""
    message: str
    """The applicability of the suggestion."""
    applicability: Applicability


def encode_diagnostic_suggestion(writer: Writer, value: DiagnosticSuggestion) -> None:
    destack._generated.protocol.source.edit.edit.encode_patch_set(writer, value.patches)
    writer.write_unsigned(len(value.labels))
    for item_0 in value.labels:
        destack._generated.protocol.source.diagnostic.label.encode_diagnostic_label(
            writer, item_0
        )
    writer.write_string(value.message)
    encode_applicability(writer, value.applicability)


def decode_diagnostic_suggestion(reader: Reader) -> DiagnosticSuggestion:
    field_0 = destack._generated.protocol.source.edit.edit.decode_patch_set(reader)
    field_1 = [
        destack._generated.protocol.source.diagnostic.label.decode_diagnostic_label(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_2 = reader.read_string()
    field_3 = decode_applicability(reader)

    return DiagnosticSuggestion(
        patches=field_0,
        labels=field_1,
        message=field_2,
        applicability=field_3,
    )


"""Whether a suggestion can be applied automatically."""
Applicability: TypeAlias = (
    Literal["automatic"] | Literal["unsafe"] | Literal["dangerous"]
)


def encode_applicability(writer: Writer, value: Applicability) -> None:
    if value == "automatic":
        writer.write_unsigned(0)
    elif value == "unsafe":
        writer.write_unsigned(1)
    elif value == "dangerous":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_applicability(reader: Reader) -> Applicability:
    variant = reader.read_number()

    if variant == 0:
        return "automatic"
    elif variant == 1:
        return "unsafe"
    elif variant == 2:
        return "dangerous"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "DiagnosticSuggestion",
    "encode_diagnostic_suggestion",
    "decode_diagnostic_suggestion",
    "Applicability",
    "encode_applicability",
    "decode_applicability",
]
