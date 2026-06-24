from __future__ import annotations

from collections.abc import Iterable

from .._generated.core.string import StringId, StringPoolData

class StringPool:
    """Pool for resolving stable string ids to strings."""

    def __init__(self, strings: Iterable[tuple[StringId, str]]) -> None: ...
    @classmethod
    def from_data(cls, data: StringPoolData) -> StringPool:
        """Create one string pool from serialized string pool data."""
        ...

    def has(self, id_: StringId) -> bool:
        """Return whether one string id is present."""
        ...

    def get(self, id_: StringId) -> str:
        """Return the string for one string id."""
        ...

    def get_maybe(self, id_: StringId | None) -> str | None:
        """Return the string for one string id when present."""
        ...

    def intern(self, text: str) -> StringId:
        """Return the stable string id for one already interned string."""
        ...

    def values(self, ids: Iterable[StringId]) -> list[str]:
        """Return all strings for a sequence of string ids."""
        ...

    def join(self, ids: Iterable[StringId], separator: str = "") -> str:
        """Join all strings for a sequence of string ids."""
        ...

    def data(self) -> StringPoolData:
        """Return this pool as serialized data."""
        ...
