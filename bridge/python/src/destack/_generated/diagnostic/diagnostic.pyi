# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

import destack._generated.diagnostic.edit
import destack._generated.source.file
import destack._generated.source.span

if TYPE_CHECKING:
    from destack._generated.diagnostic.edit import (
        PatchSet,
    )

    from destack._generated.source.file import (
        ContentId,
    )

    from destack._generated.source.span import (
        Span,
    )

"""Diagnostic severity crossing bridge boundaries."""
DiagnosticSeverity: TypeAlias = Literal["note"] | Literal["warning"] | Literal["error"]

def encode_diagnostic_severity(writer: Writer, value: DiagnosticSeverity) -> None: ...
def decode_diagnostic_severity(reader: Reader) -> DiagnosticSeverity: ...

"""Extra semantic diagnostic tag crossing bridge boundaries."""
DiagnosticTag: TypeAlias = Literal["unnecessary"] | Literal["deprecated"]

def encode_diagnostic_tag(writer: Writer, value: DiagnosticTag) -> None: ...
def decode_diagnostic_tag(reader: Reader) -> DiagnosticTag: ...

"""Whether a suggestion can be applied automatically."""
Applicability: TypeAlias = (
    Literal["automatic"] | Literal["unsafe"] | Literal["dangerous"]
)

def encode_applicability(writer: Writer, value: Applicability) -> None: ...
def decode_applicability(reader: Reader) -> Applicability: ...

@dataclass(frozen=True, slots=True)
class DiagnosticLabel:
    """One concrete source label in a diagnostic."""

    """Exact content containing the span."""
    content: ContentId
    """Concrete source span."""
    span: Span
    """Optional label shown on the span."""
    message: str | None

def encode_diagnostic_label(writer: Writer, value: DiagnosticLabel) -> None: ...
def decode_diagnostic_label(reader: Reader) -> DiagnosticLabel: ...

@dataclass(frozen=True, slots=True)
class DiagnosticNote:
    """Extra context for understanding a diagnostic."""

    """Note message."""
    message: str

def encode_diagnostic_note(writer: Writer, value: DiagnosticNote) -> None: ...
def decode_diagnostic_note(reader: Reader) -> DiagnosticNote: ...

@dataclass(frozen=True, slots=True)
class DiagnosticHelp:
    """Guidance for fixing or avoiding a diagnostic."""

    """Help message."""
    message: str

def encode_diagnostic_help(writer: Writer, value: DiagnosticHelp) -> None: ...
def decode_diagnostic_help(reader: Reader) -> DiagnosticHelp: ...

@dataclass(frozen=True, slots=True)
class DiagnosticSuggestion:
    """One suggested source change for a diagnostic."""

    """Exact source patches for machine application."""
    patches: PatchSet
    """Source labels to show with the suggestion."""
    labels: Sequence[DiagnosticLabel]
    """Suggestion message."""
    message: str
    """Suggestion applicability."""
    applicability: Applicability

def encode_diagnostic_suggestion(
    writer: Writer, value: DiagnosticSuggestion
) -> None: ...
def decode_diagnostic_suggestion(reader: Reader) -> DiagnosticSuggestion: ...

@dataclass(frozen=True, slots=True)
class Diagnostic:
    """One final renderable diagnostic."""

    """Stable diagnostic code."""
    code: str
    """Diagnostic severity."""
    severity: DiagnosticSeverity
    """Diagnostic message."""
    message: str
    """Main source label."""
    primary: DiagnosticLabel
    """Additional source labels."""
    labels: Sequence[DiagnosticLabel]
    """Extra context."""
    notes: Sequence[DiagnosticNote]
    """Fixing or avoidance guidance."""
    helps: Sequence[DiagnosticHelp]
    """Suggested source changes."""
    suggestions: Sequence[DiagnosticSuggestion]
    """Extra semantic tags."""
    tags: Sequence[DiagnosticTag]

def encode_diagnostic(writer: Writer, value: Diagnostic) -> None: ...
def decode_diagnostic(reader: Reader) -> Diagnostic: ...

__all__ = [
    "DiagnosticSeverity",
    "encode_diagnostic_severity",
    "decode_diagnostic_severity",
    "DiagnosticTag",
    "encode_diagnostic_tag",
    "decode_diagnostic_tag",
    "Applicability",
    "encode_applicability",
    "decode_applicability",
    "DiagnosticLabel",
    "encode_diagnostic_label",
    "decode_diagnostic_label",
    "DiagnosticNote",
    "encode_diagnostic_note",
    "decode_diagnostic_note",
    "DiagnosticHelp",
    "encode_diagnostic_help",
    "decode_diagnostic_help",
    "DiagnosticSuggestion",
    "encode_diagnostic_suggestion",
    "decode_diagnostic_suggestion",
    "Diagnostic",
    "encode_diagnostic",
    "decode_diagnostic",
]
