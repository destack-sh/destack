# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

import destack._generated.source.file

if TYPE_CHECKING:
    from destack._generated.source.file import (
        FileId,
    )

@dataclass(frozen=True, slots=True)
class Span:
    """Source byte span crossing bridge boundaries."""

    """File containing this span."""
    file: FileId
    """Inclusive start byte offset."""
    start: int
    """Exclusive end byte offset."""
    end: int

def encode_span(writer: Writer, value: Span) -> None: ...
def decode_span(reader: Reader) -> Span: ...

__all__ = [
    "Span",
    "encode_span",
    "decode_span",
]
