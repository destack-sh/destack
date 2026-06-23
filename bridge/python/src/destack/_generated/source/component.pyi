# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

@dataclass(frozen=True, slots=True)
class ComponentId:
    """External component id crossing bridge boundaries."""

    """Canonical lowercase hex component id."""
    id: str

def encode_component_id(writer: Writer, value: ComponentId) -> None: ...
def decode_component_id(reader: Reader) -> ComponentId: ...

__all__ = [
    "ComponentId",
    "encode_component_id",
    "decode_component_id",
]
