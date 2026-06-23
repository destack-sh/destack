# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class ComponentId:
    """External component id crossing bridge boundaries."""

    """Canonical lowercase hex component id."""
    id: str


def encode_component_id(writer: Writer, value: ComponentId) -> None:
    writer.write_string(value.id)


def decode_component_id(reader: Reader) -> ComponentId:
    field_0 = reader.read_string()

    return ComponentId(
        id=field_0,
    )


__all__ = [
    "ComponentId",
    "encode_component_id",
    "decode_component_id",
]
