# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.mir.metadata.trace import (
    TraceMapImpl,
)
from destack._impl.mir.metadata.trace import (
    TraceTableImpl,
)

@dataclass(frozen=True, slots=True)
class TraceMapEmpty(TraceMapImpl):
    """Payload contains no heap references."""

    kind: typing.Literal["empty"] = "empty"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TraceMapFixed(TraceMapImpl):
    """Payload stores heap-reference words at fixed byte offsets."""

    # byte offsets of encoded local heap references
    local_offsets: Sequence[int]
    # byte offsets of encoded shared heap references
    shared_offsets: Sequence[int]
    kind: typing.Literal["fixed"] = "fixed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TraceMapNested(TraceMapImpl):
    """Payload stores one nested map at a byte offset."""

    # the byte offset of the nested payload
    byte_offset: int
    # the nested trace map
    map: TraceMap
    kind: typing.Literal["nested"] = "nested"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TraceMapComposite(TraceMapImpl):
    """Payload stores multiple nested maps."""

    # the nested trace maps
    maps: Sequence[TraceMap]
    kind: typing.Literal["composite"] = "composite"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TraceMapRepeated(TraceMapImpl):
    """Payload stores repeated elements with one nested trace map."""

    # the number of elements in the payload
    count: int
    # the element byte stride
    stride: int
    # the per-element trace map
    element: TraceMap
    kind: typing.Literal["repeated"] = "repeated"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TraceMapTagged(TraceMapImpl):
    """Payload stores a tagged variant with variant-specific trace maps."""

    # the byte width of the variant tag
    tag_bytes: int
    # variant trace maps keyed by normalized tag value
    variants: Sequence[TraceVariant]
    kind: typing.Literal["tagged"] = "tagged"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Heap trace metadata for one runtime payload."""
TraceMap: typing.TypeAlias = (
    TraceMapEmpty
    | TraceMapFixed
    | TraceMapNested
    | TraceMapComposite
    | TraceMapRepeated
    | TraceMapTagged
)

def encode_trace_map(writer: BinaryWriter, value: TraceMap) -> None: ...
def decode_trace_map(reader: BinaryReader) -> TraceMap: ...
def to_json_trace_map(value: TraceMap) -> Json: ...
def from_json_trace_map(value: Json) -> TraceMap: ...

@dataclass(frozen=True, slots=True)
class TraceVariant:
    """One tag-selected trace variant."""

    # the normalized numeric tag value selecting this variant
    tag: int
    # the byte offset of the variant payload
    payload_offset: int
    # the payload trace map for this variant
    map: TraceMap

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceVariant: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TraceVariant: ...

def encode_trace_variant(writer: BinaryWriter, value: TraceVariant) -> None: ...
def decode_trace_variant(reader: BinaryReader) -> TraceVariant: ...
def to_json_trace_variant(value: TraceVariant) -> Json: ...
def from_json_trace_variant(value: Json) -> TraceVariant: ...

@dataclass(frozen=True, slots=True)
class TraceTable(TraceTableImpl):
    """Shared table of heap trace maps for one lowered program."""

    # trace maps indexed by TraceId
    traces: Sequence[TraceMap]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TraceTable: ...

def encode_trace_table(writer: BinaryWriter, value: TraceTable) -> None: ...
def decode_trace_table(reader: BinaryReader) -> TraceTable: ...
def to_json_trace_table(value: TraceTable) -> Json: ...
def from_json_trace_table(value: Json) -> TraceTable: ...

"""Stable non-zero identifier for one heap trace map."""
TraceId: typing.TypeAlias = int

def encode_trace_id(writer: BinaryWriter, value: TraceId) -> None: ...
def decode_trace_id(reader: BinaryReader) -> TraceId: ...
def to_json_trace_id(value: TraceId) -> Json: ...
def from_json_trace_id(value: Json) -> TraceId: ...

__all__ = [
    "TraceMap",
    "encode_trace_map",
    "decode_trace_map",
    "to_json_trace_map",
    "from_json_trace_map",
    "TraceMapEmpty",
    "TraceMapFixed",
    "TraceMapNested",
    "TraceMapComposite",
    "TraceMapRepeated",
    "TraceMapTagged",
    "TraceVariant",
    "encode_trace_variant",
    "decode_trace_variant",
    "to_json_trace_variant",
    "from_json_trace_variant",
    "TraceTable",
    "encode_trace_table",
    "decode_trace_table",
    "to_json_trace_table",
    "from_json_trace_table",
    "TraceId",
    "encode_trace_id",
    "decode_trace_id",
    "to_json_trace_id",
    "from_json_trace_id",
]
