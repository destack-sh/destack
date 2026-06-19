# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.diagnostic.diagnostic import (
    Diagnostic,
)

from destack.session.module import (
    Module,
)

from destack.source.package import (
    PackageId,
)

from destack.source.profile import (
    ProfileId,
)

class Scope:
    """Scope accepted by lint operations."""

    """One module profile."""
    @staticmethod
    def module(module: Module, profile: ProfileId) -> Scope: ...

    """One source package."""
    @staticmethod
    def package(package: PackageId) -> Scope: ...

    """Whole workspace."""
    @staticmethod
    def workspace() -> Scope: ...

    @property
    def kind(self) -> str: ...

class LintRequest:
    """One linter request."""

    def __init__(self, scope: Scope) -> None: ...

class LintOutput:
    """One linter output."""

    """Diagnostics emitted by lint rules."""
    @property
    def diagnostics(self) -> list[Diagnostic]: ...
