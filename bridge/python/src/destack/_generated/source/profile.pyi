# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

@dataclass(frozen=True, slots=True)
class ProfileId:
    """External profile id crossing bridge boundaries."""

    """Canonical lowercase hex profile id."""
    id: str

def encode_profile_id(writer: Writer, value: ProfileId) -> None: ...
def decode_profile_id(reader: Reader) -> ProfileId: ...

__all__ = [
    "ProfileId",
    "encode_profile_id",
    "decode_profile_id",
]
