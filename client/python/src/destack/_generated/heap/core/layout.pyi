# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.metadata.trace

@dataclass(frozen=True, slots=True)
class AllocationSite:
    """One explicit allocation site."""

    # the exact payload byte length
    byte_len: int
    # the required block base alignment in bytes
    alignment: int
    # the canonical trace id when this site has table-backed metadata
    trace_id: destack._generated.mir.metadata.trace.TraceId | None
    # whether the payload contains no heap references
    is_noscan: bool
    # whether the payload may contain shared heap references
    has_shared_reference: bool
    # the allocator class used by this site
    class_: AllocationClass

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AllocationSite: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AllocationSite: ...

def encode_allocation_site(writer: BinaryWriter, value: AllocationSite) -> None: ...
def decode_allocation_site(reader: BinaryReader) -> AllocationSite: ...
def to_json_allocation_site(value: AllocationSite) -> Json: ...
def from_json_allocation_site(value: Json) -> AllocationSite: ...

@dataclass(frozen=True, slots=True)
class AllocationClassSmall:
    """One block backed by a small-span slot."""

    small: SmallAllocationPlan
    kind: typing.Literal["small"] = "small"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AllocationClassLarge:
    """One block backed by a dedicated page span."""

    kind: typing.Literal["large"] = "large"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One allocator-ready allocation class."""
AllocationClass: typing.TypeAlias = AllocationClassSmall | AllocationClassLarge

def encode_allocation_class(writer: BinaryWriter, value: AllocationClass) -> None: ...
def decode_allocation_class(reader: BinaryReader) -> AllocationClass: ...
def to_json_allocation_class(value: AllocationClass) -> Json: ...
def from_json_allocation_class(value: Json) -> AllocationClass: ...

@dataclass(frozen=True, slots=True)
class SmallAllocationPlan:
    """One allocator-ready small allocation plan."""

    # the exact mutator-cache index for this class
    cache_index: SmallCacheIndex
    # the smallest payload byte length routed to this class
    minimum_byte_len: int
    # the small-span class used by local and shared spaces
    class_: SmallSpanClass

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SmallAllocationPlan: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SmallAllocationPlan: ...

def encode_small_allocation_plan(
    writer: BinaryWriter, value: SmallAllocationPlan
) -> None: ...
def decode_small_allocation_plan(reader: BinaryReader) -> SmallAllocationPlan: ...
def to_json_small_allocation_plan(value: SmallAllocationPlan) -> Json: ...
def from_json_small_allocation_plan(value: Json) -> SmallAllocationPlan: ...

"""Dense mutator cache index for one small allocation site."""
SmallCacheIndex: typing.TypeAlias = int

def encode_small_cache_index(writer: BinaryWriter, value: SmallCacheIndex) -> None: ...
def decode_small_cache_index(reader: BinaryReader) -> SmallCacheIndex: ...
def to_json_small_cache_index(value: SmallCacheIndex) -> Json: ...
def from_json_small_cache_index(value: Json) -> SmallCacheIndex: ...

@dataclass(frozen=True, slots=True)
class SmallSpanClass:
    """One small-span size and scan class."""

    # the slot payload size in bytes
    size_class: int
    # the span byte width for this size class
    span_size_bytes: int
    # the table-backed trace id shared by every slot in this class
    trace_id: destack._generated.mir.metadata.trace.TraceId | None
    # whether every slot in this span has no references
    is_noscan: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SmallSpanClass: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SmallSpanClass: ...

def encode_small_span_class(writer: BinaryWriter, value: SmallSpanClass) -> None: ...
def decode_small_span_class(reader: BinaryReader) -> SmallSpanClass: ...
def to_json_small_span_class(value: SmallSpanClass) -> Json: ...
def from_json_small_span_class(value: Json) -> SmallSpanClass: ...

@dataclass(frozen=True, slots=True)
class SmallAllocationSite:
    """One explicit small allocation site."""

    # the exact payload byte length
    byte_len: int
    # the resolved small allocation plan
    small: SmallAllocationPlan

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SmallAllocationSite: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SmallAllocationSite: ...

def encode_small_allocation_site(
    writer: BinaryWriter, value: SmallAllocationSite
) -> None: ...
def decode_small_allocation_site(reader: BinaryReader) -> SmallAllocationSite: ...
def to_json_small_allocation_site(value: SmallAllocationSite) -> Json: ...
def from_json_small_allocation_site(value: Json) -> SmallAllocationSite: ...

__all__ = [
    "AllocationSite",
    "encode_allocation_site",
    "decode_allocation_site",
    "to_json_allocation_site",
    "from_json_allocation_site",
    "AllocationClass",
    "encode_allocation_class",
    "decode_allocation_class",
    "to_json_allocation_class",
    "from_json_allocation_class",
    "AllocationClassSmall",
    "AllocationClassLarge",
    "SmallAllocationPlan",
    "encode_small_allocation_plan",
    "decode_small_allocation_plan",
    "to_json_small_allocation_plan",
    "from_json_small_allocation_plan",
    "SmallCacheIndex",
    "encode_small_cache_index",
    "decode_small_cache_index",
    "to_json_small_cache_index",
    "from_json_small_cache_index",
    "SmallSpanClass",
    "encode_small_span_class",
    "decode_small_span_class",
    "to_json_small_span_class",
    "from_json_small_span_class",
    "SmallAllocationSite",
    "encode_small_allocation_site",
    "decode_small_allocation_site",
    "to_json_small_allocation_site",
    "from_json_small_allocation_site",
]
