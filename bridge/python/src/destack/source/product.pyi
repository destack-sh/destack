# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.package import (
    PackageId,
)

class ProductId:
    """External product id crossing bridge boundaries."""

    def __init__(self, package: PackageId, key: str) -> None: ...

    """Owning package."""
    @property
    def package(self) -> PackageId: ...

    """Canonical lowercase hex product key within the package."""
    @property
    def key(self) -> str: ...
