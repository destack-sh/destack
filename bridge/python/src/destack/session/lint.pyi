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

    @property
    def module_module(self) -> Module | None: ...

    @property
    def package_package(self) -> PackageId | None: ...

    @property
    def profile(self) -> ProfileId | None: ...

class LintRequest:
    """One linter request."""

    def __init__(self, scope: Scope) -> None: ...

    """Scope to lint."""
    @property
    def scope(self) -> Scope: ...

class LintOutput:
    """One linter output."""

    def __init__(self, diagnostics: Sequence[Diagnostic]) -> None: ...

    """Diagnostics emitted by lint rules."""
    @property
    def diagnostics(self) -> list[Diagnostic]: ...
