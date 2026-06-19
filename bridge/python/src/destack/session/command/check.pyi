# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.diagnostic.diagnostic import (
    Diagnostic,
)

from destack.dir.checked import (
    DirChecked,
)

class CheckOutput:
    """One checker output."""

    """Checked DIR artifact projection."""
    @property
    def checked(self) -> DirChecked: ...

    """Diagnostics emitted by checking."""
    @property
    def diagnostics(self) -> list[Diagnostic]: ...
