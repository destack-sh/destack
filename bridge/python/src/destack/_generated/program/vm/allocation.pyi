# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.heap.core.layout
import destack._generated.mir.metadata.trace
import destack._generated.program.vm.table

@dataclass(frozen=True, slots=True)
class AllocationBranch:
    """Branching allocation consumed by fallible heap allocation instructions."""

    # the destination frame offset
    destination: int
    # the allocation site
    allocation: destack._generated.program.vm.table.AllocationSiteId
    # the success edge
    success: destack._generated.program.vm.table.Edge
    # the failure edge
    failure: destack._generated.program.vm.table.Edge

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AllocationBranch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AllocationBranch: ...

def encode_allocation_branch(writer: BinaryWriter, value: AllocationBranch) -> None: ...
def decode_allocation_branch(reader: BinaryReader) -> AllocationBranch: ...
def to_json_allocation_branch(value: AllocationBranch) -> Json: ...
def from_json_allocation_branch(value: Json) -> AllocationBranch: ...

@dataclass(frozen=True, slots=True)
class SliceAllocationBranch:
    """Branching slice allocation consumed by fallible slice allocation instructions."""

    # the destination frame offset
    destination: int
    # the length cell frame offset
    length: int
    # the backing element allocation site
    element: destack._generated.program.vm.table.AllocationSiteId
    # the slice descriptor projection
    access: destack._generated.program.vm.table.SliceProjectionId
    # the success edge
    success: destack._generated.program.vm.table.Edge
    # the failure edge
    failure: destack._generated.program.vm.table.Edge

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SliceAllocationBranch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SliceAllocationBranch: ...

def encode_slice_allocation_branch(
    writer: BinaryWriter, value: SliceAllocationBranch
) -> None: ...
def decode_slice_allocation_branch(reader: BinaryReader) -> SliceAllocationBranch: ...
def to_json_slice_allocation_branch(value: SliceAllocationBranch) -> Json: ...
def from_json_slice_allocation_branch(value: Json) -> SliceAllocationBranch: ...

@dataclass(frozen=True, slots=True)
class AllocationSite:
    """The allocation site consumed by heap allocation instructions."""

    # the heap-ready allocation site
    heap: destack._generated.heap.core.layout.AllocationSite
    # the exact heap trace map id
    trace_map: destack._generated.mir.metadata.trace.TraceId

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
class SmallAllocationSite:
    """The small allocation site consumed by small heap allocation instructions."""

    # the full heap-ready allocation site for cold allocation
    heap: destack._generated.heap.core.layout.AllocationSite
    # the heap-ready small allocation site for cursor reservation
    small: destack._generated.heap.core.layout.SmallAllocationSite
    # the exact heap trace map id
    trace_map: destack._generated.mir.metadata.trace.TraceId

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
    "AllocationBranch",
    "encode_allocation_branch",
    "decode_allocation_branch",
    "to_json_allocation_branch",
    "from_json_allocation_branch",
    "SliceAllocationBranch",
    "encode_slice_allocation_branch",
    "decode_slice_allocation_branch",
    "to_json_slice_allocation_branch",
    "from_json_slice_allocation_branch",
    "AllocationSite",
    "encode_allocation_site",
    "decode_allocation_site",
    "to_json_allocation_site",
    "from_json_allocation_site",
    "SmallAllocationSite",
    "encode_small_allocation_site",
    "decode_small_allocation_site",
    "to_json_small_allocation_site",
    "from_json_small_allocation_site",
]
