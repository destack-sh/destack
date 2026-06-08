# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.diagnostic.edit import (
    BatchEdit,
)

from destack.source.file import (
    FileContentId,
)

from destack.source.span import (
    Span,
)

class DiagnosticSeverity:
    """Diagnostic severity crossing bridge boundaries."""

    """Informative message."""
    @staticmethod
    def note() -> DiagnosticSeverity: ...

    """Non-critical issue."""
    @staticmethod
    def warning() -> DiagnosticSeverity: ...

    """Critical issue."""
    @staticmethod
    def error() -> DiagnosticSeverity: ...

    @property
    def label(self) -> str: ...

class DiagnosticTag:
    """Extra semantic diagnostic tag crossing bridge boundaries."""

    """Unused or unnecessary source."""
    @staticmethod
    def unnecessary() -> DiagnosticTag: ...

    """Deprecated source."""
    @staticmethod
    def deprecated() -> DiagnosticTag: ...

    @property
    def label(self) -> str: ...

class Applicability:
    """Whether a suggestion can be applied automatically."""

    """Machine-applicable suggestion."""
    @staticmethod
    def automatic() -> Applicability: ...

    """Machine-applicable suggestion that may change behavior."""
    @staticmethod
    def unsafe() -> Applicability: ...

    """Maybe incorrect suggestion."""
    @staticmethod
    def dangerous() -> Applicability: ...

    @property
    def label(self) -> str: ...

class DiagnosticLabel:
    """One concrete source label in a diagnostic."""

    def __init__(self, content: FileContentId, span: Span, message: str | None) -> None: ...

    """Exact file content containing the span."""
    @property
    def content(self) -> FileContentId: ...

    """Concrete source span."""
    @property
    def span(self) -> Span: ...

    """Optional label shown on the span."""
    @property
    def message(self) -> str | None: ...

class DiagnosticNote:
    """Extra context for understanding a diagnostic."""

    def __init__(self, message: str) -> None: ...

    """Note message."""
    @property
    def message(self) -> str: ...

class DiagnosticHelp:
    """Guidance for fixing or avoiding a diagnostic."""

    def __init__(self, message: str) -> None: ...

    """Help message."""
    @property
    def message(self) -> str: ...

class DiagnosticSuggestion:
    """One suggested source change for a diagnostic."""

    def __init__(self, edits: BatchEdit, labels: Sequence[DiagnosticLabel], message: str, applicability: Applicability) -> None: ...

    """Exact source edits for machine application."""
    @property
    def edits(self) -> BatchEdit: ...

    """Source labels to show with the suggestion."""
    @property
    def labels(self) -> list[DiagnosticLabel]: ...

    """Suggestion message."""
    @property
    def message(self) -> str: ...

    """Suggestion applicability."""
    @property
    def applicability(self) -> Applicability: ...

class Diagnostic:
    """One final renderable diagnostic."""

    def __init__(self, code: str, severity: DiagnosticSeverity, message: str, primary: DiagnosticLabel, labels: Sequence[DiagnosticLabel], notes: Sequence[DiagnosticNote], helps: Sequence[DiagnosticHelp], suggestions: Sequence[DiagnosticSuggestion], tags: Sequence[DiagnosticTag]) -> None: ...

    """Stable diagnostic code."""
    @property
    def code(self) -> str: ...

    """Diagnostic severity."""
    @property
    def severity(self) -> DiagnosticSeverity: ...

    """Diagnostic message."""
    @property
    def message(self) -> str: ...

    """Main source label."""
    @property
    def primary(self) -> DiagnosticLabel: ...

    """Additional source labels."""
    @property
    def labels(self) -> list[DiagnosticLabel]: ...

    """Extra context."""
    @property
    def notes(self) -> list[DiagnosticNote]: ...

    """Fixing or avoidance guidance."""
    @property
    def helps(self) -> list[DiagnosticHelp]: ...

    """Suggested source changes."""
    @property
    def suggestions(self) -> list[DiagnosticSuggestion]: ...

    """Extra semantic tags."""
    @property
    def tags(self) -> list[DiagnosticTag]: ...

