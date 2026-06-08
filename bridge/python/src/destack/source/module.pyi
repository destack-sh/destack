# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.package import (
    PackageId,
)

class ModuleId:
    """External module id crossing bridge boundaries."""

    def __init__(self, package: PackageId, key: str) -> None: ...

    """Owning package."""
    @property
    def package(self) -> PackageId: ...

    """Canonical lowercase hex module key within the package."""
    @property
    def key(self) -> str: ...

