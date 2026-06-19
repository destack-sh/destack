# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.diagnostic.edit import (
    BatchEdit,
)

from destack.source.file import (
    ContentId,
)

from destack.source.span import (
    Span,
)

class DiagnosticSeverity:
    """Diagnostic severity crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class DiagnosticTag:
    """Extra semantic diagnostic tag crossing bridge boundaries."""

    @property
    def label(self) -> str: ...

class Applicability:
    """Whether a suggestion can be applied automatically."""

    @property
    def label(self) -> str: ...

class DiagnosticLabel:
    """One concrete source label in a diagnostic."""

    """Exact content containing the span."""
    @property
    def content(self) -> ContentId: ...

    """Concrete source span."""
    @property
    def span(self) -> Span: ...

    """Optional label shown on the span."""
    @property
    def message(self) -> str | None: ...

class DiagnosticNote:
    """Extra context for understanding a diagnostic."""

    """Note message."""
    @property
    def message(self) -> str: ...

class DiagnosticHelp:
    """Guidance for fixing or avoiding a diagnostic."""

    """Help message."""
    @property
    def message(self) -> str: ...

class DiagnosticSuggestion:
    """One suggested source change for a diagnostic."""

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
