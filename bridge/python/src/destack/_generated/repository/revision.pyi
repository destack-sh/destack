# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

@dataclass(frozen=True, slots=True)
class Revision:
    """External revision value crossing bridge boundaries."""

    """Displayed repository revision id."""
    id: str

def encode_revision(writer: Writer, value: Revision) -> None: ...
def decode_revision(reader: Reader) -> Revision: ...

__all__ = [
    "Revision",
    "encode_revision",
    "decode_revision",
]
