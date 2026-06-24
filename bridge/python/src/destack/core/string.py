from __future__ import annotations

from collections.abc import Iterable, Sequence
from dataclasses import dataclass

from .._generated.core.string import StringId, StringPoolData


@dataclass(frozen=True, slots=True)
class StringPool:
    """Pool for resolving stable string ids to strings."""

    _text_by_id: dict[StringId, str]
    _id_by_text: dict[str, StringId]

    def __init__(self, strings: Iterable[tuple[StringId, str]]) -> None:
        # build both lookup directions in one pass
        text_by_id: dict[StringId, str] = {}
        id_by_text: dict[str, StringId] = {}

        # validate all entries while building both lookup directions
        for id_, text in strings:
            existing_text = text_by_id.get(id_)
            if existing_text is not None and existing_text != text:
                raise ValueError(f"string id collision: {id_}")

            existing_id = id_by_text.get(text)
            if existing_id is not None and existing_id != id_:
                raise ValueError(f"duplicate string text with different ids: {text}")

            text_by_id[id_] = text
            id_by_text[text] = id_

        object.__setattr__(self, "_text_by_id", text_by_id)
        object.__setattr__(self, "_id_by_text", id_by_text)

    @classmethod
    def from_data(cls, data: StringPoolData) -> StringPool:
        """Create one string pool from serialized string pool data."""

        return cls(data.strings)

    def has(self, id_: StringId) -> bool:
        """Return whether one string id is present."""

        return id_ in self._text_by_id

    def get(self, id_: StringId) -> str:
        """Return the string for one string id."""

        text = self._text_by_id.get(id_)
        if text is None:
            raise KeyError(f"unknown string id: {id_}")

        return text

    def get_maybe(self, id_: StringId | None) -> str | None:
        """Return the string for one string id when present."""

        return None if id_ is None else self._text_by_id.get(id_)

    def intern(self, text: str) -> StringId:
        """Return the stable string id for one already interned string."""

        id_ = self._id_by_text.get(text)
        if id_ is None:
            raise KeyError(f"unknown string text: {text}")

        return id_

    def values(self, ids: Iterable[StringId]) -> list[str]:
        """Return all strings for a sequence of string ids."""

        values = []

        # resolve ids in caller-provided order
        for id_ in ids:
            values.append(self.get(id_))

        return values

    def join(self, ids: Iterable[StringId], separator: str = "") -> str:
        """Join all strings for a sequence of string ids."""

        return separator.join(self.values(ids))

    def data(self) -> StringPoolData:
        """Return this pool as serialized data."""

        strings: Sequence[tuple[StringId, str]] = sorted(self._text_by_id.items())

        return StringPoolData(strings=strings)
