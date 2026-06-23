# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class ComponentId:
    """Stable identifier for one source component."""

    field_0: int


def encode_component_id(writer: Writer, value: ComponentId) -> None:
    writer.write_unsigned(value.field_0)


def decode_component_id(reader: Reader) -> ComponentId:
    field_0 = reader.read_unsigned()

    return ComponentId(
        field_0=field_0,
    )


__all__ = [
    "ComponentId",
    "encode_component_id",
    "decode_component_id",
]
