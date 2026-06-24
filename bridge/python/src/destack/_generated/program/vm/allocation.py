# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_allocation_branch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AllocationBranch:
        """Decode one AllocationBranch."""
        return decode_allocation_branch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_allocation_branch(self)

    @classmethod
    def from_json(cls, value: Json) -> AllocationBranch:
        """Return one AllocationBranch from one JSON value."""
        return from_json_allocation_branch(value)


def encode_allocation_branch(writer: BinaryWriter, value: AllocationBranch) -> None:
    """Encode one AllocationBranch."""
    writer.write_unsigned(value.destination)
    destack._generated.program.vm.table.encode_allocation_site_id(
        writer, value.allocation
    )
    destack._generated.program.vm.table.encode_edge(writer, value.success)
    destack._generated.program.vm.table.encode_edge(writer, value.failure)


def decode_allocation_branch(reader: BinaryReader) -> AllocationBranch:
    """Decode one AllocationBranch."""
    destination = reader.read_number()
    allocation = destack._generated.program.vm.table.decode_allocation_site_id(reader)
    success = destack._generated.program.vm.table.decode_edge(reader)
    failure = destack._generated.program.vm.table.decode_edge(reader)

    return AllocationBranch(
        destination=destination,
        allocation=allocation,
        success=success,
        failure=failure,
    )


def to_json_allocation_branch(value: AllocationBranch) -> Json:
    """Return one JSON value for one AllocationBranch."""
    return {
        "destination": value.destination,
        "allocation": destack._generated.program.vm.table.to_json_allocation_site_id(
            value.allocation
        ),
        "success": destack._generated.program.vm.table.to_json_edge(value.success),
        "failure": destack._generated.program.vm.table.to_json_edge(value.failure),
    }


def from_json_allocation_branch(value: Json) -> AllocationBranch:
    """Return one AllocationBranch from one JSON value."""
    object_ = json_object(value)

    return AllocationBranch(
        destination=json_int(json_field(object_, "destination")),
        allocation=destack._generated.program.vm.table.from_json_allocation_site_id(
            json_field(object_, "allocation")
        ),
        success=destack._generated.program.vm.table.from_json_edge(
            json_field(object_, "success")
        ),
        failure=destack._generated.program.vm.table.from_json_edge(
            json_field(object_, "failure")
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_slice_allocation_branch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SliceAllocationBranch:
        """Decode one SliceAllocationBranch."""
        return decode_slice_allocation_branch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_slice_allocation_branch(self)

    @classmethod
    def from_json(cls, value: Json) -> SliceAllocationBranch:
        """Return one SliceAllocationBranch from one JSON value."""
        return from_json_slice_allocation_branch(value)


def encode_slice_allocation_branch(
    writer: BinaryWriter, value: SliceAllocationBranch
) -> None:
    """Encode one SliceAllocationBranch."""
    writer.write_unsigned(value.destination)
    writer.write_unsigned(value.length)
    destack._generated.program.vm.table.encode_allocation_site_id(writer, value.element)
    destack._generated.program.vm.table.encode_slice_projection_id(writer, value.access)
    destack._generated.program.vm.table.encode_edge(writer, value.success)
    destack._generated.program.vm.table.encode_edge(writer, value.failure)


def decode_slice_allocation_branch(reader: BinaryReader) -> SliceAllocationBranch:
    """Decode one SliceAllocationBranch."""
    destination = reader.read_number()
    length = reader.read_number()
    element = destack._generated.program.vm.table.decode_allocation_site_id(reader)
    access = destack._generated.program.vm.table.decode_slice_projection_id(reader)
    success = destack._generated.program.vm.table.decode_edge(reader)
    failure = destack._generated.program.vm.table.decode_edge(reader)

    return SliceAllocationBranch(
        destination=destination,
        length=length,
        element=element,
        access=access,
        success=success,
        failure=failure,
    )


def to_json_slice_allocation_branch(value: SliceAllocationBranch) -> Json:
    """Return one JSON value for one SliceAllocationBranch."""
    return {
        "destination": value.destination,
        "length": value.length,
        "element": destack._generated.program.vm.table.to_json_allocation_site_id(
            value.element
        ),
        "access": destack._generated.program.vm.table.to_json_slice_projection_id(
            value.access
        ),
        "success": destack._generated.program.vm.table.to_json_edge(value.success),
        "failure": destack._generated.program.vm.table.to_json_edge(value.failure),
    }


def from_json_slice_allocation_branch(value: Json) -> SliceAllocationBranch:
    """Return one SliceAllocationBranch from one JSON value."""
    object_ = json_object(value)

    return SliceAllocationBranch(
        destination=json_int(json_field(object_, "destination")),
        length=json_int(json_field(object_, "length")),
        element=destack._generated.program.vm.table.from_json_allocation_site_id(
            json_field(object_, "element")
        ),
        access=destack._generated.program.vm.table.from_json_slice_projection_id(
            json_field(object_, "access")
        ),
        success=destack._generated.program.vm.table.from_json_edge(
            json_field(object_, "success")
        ),
        failure=destack._generated.program.vm.table.from_json_edge(
            json_field(object_, "failure")
        ),
    )


@dataclass(frozen=True, slots=True)
class AllocationSite:
    """The allocation site consumed by heap allocation instructions."""

    # the heap-ready allocation site
    heap: destack._generated.heap.core.layout.AllocationSite
    # the exact heap trace map id
    trace_map: destack._generated.mir.metadata.trace.TraceId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_allocation_site(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AllocationSite:
        """Decode one AllocationSite."""
        return decode_allocation_site(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_allocation_site(self)

    @classmethod
    def from_json(cls, value: Json) -> AllocationSite:
        """Return one AllocationSite from one JSON value."""
        return from_json_allocation_site(value)


def encode_allocation_site(writer: BinaryWriter, value: AllocationSite) -> None:
    """Encode one AllocationSite."""
    destack._generated.heap.core.layout.encode_allocation_site(writer, value.heap)
    destack._generated.mir.metadata.trace.encode_trace_id(writer, value.trace_map)


def decode_allocation_site(reader: BinaryReader) -> AllocationSite:
    """Decode one AllocationSite."""
    heap = destack._generated.heap.core.layout.decode_allocation_site(reader)
    trace_map = destack._generated.mir.metadata.trace.decode_trace_id(reader)

    return AllocationSite(
        heap=heap,
        trace_map=trace_map,
    )


def to_json_allocation_site(value: AllocationSite) -> Json:
    """Return one JSON value for one AllocationSite."""
    return {
        "heap": destack._generated.heap.core.layout.to_json_allocation_site(value.heap),
        "traceMap": destack._generated.mir.metadata.trace.to_json_trace_id(
            value.trace_map
        ),
    }


def from_json_allocation_site(value: Json) -> AllocationSite:
    """Return one AllocationSite from one JSON value."""
    object_ = json_object(value)

    return AllocationSite(
        heap=destack._generated.heap.core.layout.from_json_allocation_site(
            json_field(object_, "heap")
        ),
        trace_map=destack._generated.mir.metadata.trace.from_json_trace_id(
            json_field(object_, "traceMap")
        ),
    )


@dataclass(frozen=True, slots=True)
class SmallAllocationSite:
    """The small allocation site consumed by small heap allocation instructions."""

    # the full heap-ready allocation site for cold allocation
    heap: destack._generated.heap.core.layout.AllocationSite
    # the heap-ready small allocation site for cursor reservation
    small: destack._generated.heap.core.layout.SmallAllocationSite
    # the exact heap trace map id
    trace_map: destack._generated.mir.metadata.trace.TraceId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_small_allocation_site(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SmallAllocationSite:
        """Decode one SmallAllocationSite."""
        return decode_small_allocation_site(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_small_allocation_site(self)

    @classmethod
    def from_json(cls, value: Json) -> SmallAllocationSite:
        """Return one SmallAllocationSite from one JSON value."""
        return from_json_small_allocation_site(value)


def encode_small_allocation_site(
    writer: BinaryWriter, value: SmallAllocationSite
) -> None:
    """Encode one SmallAllocationSite."""
    destack._generated.heap.core.layout.encode_allocation_site(writer, value.heap)
    destack._generated.heap.core.layout.encode_small_allocation_site(
        writer, value.small
    )
    destack._generated.mir.metadata.trace.encode_trace_id(writer, value.trace_map)


def decode_small_allocation_site(reader: BinaryReader) -> SmallAllocationSite:
    """Decode one SmallAllocationSite."""
    heap = destack._generated.heap.core.layout.decode_allocation_site(reader)
    small = destack._generated.heap.core.layout.decode_small_allocation_site(reader)
    trace_map = destack._generated.mir.metadata.trace.decode_trace_id(reader)

    return SmallAllocationSite(
        heap=heap,
        small=small,
        trace_map=trace_map,
    )


def to_json_small_allocation_site(value: SmallAllocationSite) -> Json:
    """Return one JSON value for one SmallAllocationSite."""
    return {
        "heap": destack._generated.heap.core.layout.to_json_allocation_site(value.heap),
        "small": destack._generated.heap.core.layout.to_json_small_allocation_site(
            value.small
        ),
        "traceMap": destack._generated.mir.metadata.trace.to_json_trace_id(
            value.trace_map
        ),
    }


def from_json_small_allocation_site(value: Json) -> SmallAllocationSite:
    """Return one SmallAllocationSite from one JSON value."""
    object_ = json_object(value)

    return SmallAllocationSite(
        heap=destack._generated.heap.core.layout.from_json_allocation_site(
            json_field(object_, "heap")
        ),
        small=destack._generated.heap.core.layout.from_json_small_allocation_site(
            json_field(object_, "small")
        ),
        trace_map=destack._generated.mir.metadata.trace.from_json_trace_id(
            json_field(object_, "traceMap")
        ),
    )


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
