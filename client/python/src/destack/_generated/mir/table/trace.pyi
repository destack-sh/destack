# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class TraceMapEmpty:
    """The payload contains no references."""

    kind: typing.Literal["empty"] = "empty"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TraceMapFixed:
    """The payload stores reference words at fixed byte offsets."""

    # byte offsets of encoded local heap references
    local_offsets: Sequence[int]
    # byte offsets of encoded shared heap references
    shared_offsets: Sequence[int]
    # byte offsets of encoded frame references
    frame_offsets: Sequence[int]
    kind: typing.Literal["fixed"] = "fixed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TraceMapNested:
    """The payload stores one nested map at a byte offset."""

    # the byte offset of the nested payload
    byte_offset: int
    # the nested trace map
    map: TraceMap
    kind: typing.Literal["nested"] = "nested"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TraceMapComposite:
    """The payload stores multiple nested maps."""

    # the nested trace maps
    maps: Sequence[TraceMap]
    kind: typing.Literal["composite"] = "composite"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TraceMapRepeated:
    """The payload stores repeated elements with one nested trace map."""

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
class TraceMapTagged:
    """The payload stores a tagged variant with variant-specific trace maps."""

    # the byte width of the variant tag
    tag_bytes: int
    # variant trace maps keyed by normalized tag value
    variants: Sequence[TraceVariant]
    kind: typing.Literal["tagged"] = "tagged"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Reference trace map for one value layout."""
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
]
