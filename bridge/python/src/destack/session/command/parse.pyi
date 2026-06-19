# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.diagnostic.diagnostic import (
    Diagnostic,
)

from destack.dir.parsed import (
    DirParsed,
)

class ParseOutput:
    """One parser output."""

    """Parsed DIR artifact projection."""
    @property
    def parsed(self) -> DirParsed: ...

    """Diagnostics emitted by parsing."""
    @property
    def diagnostics(self) -> list[Diagnostic]: ...
