# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

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
    writer.write_unsigned(value.byte_len)
    writer.write_unsigned(value.alignment)
    if value.trace_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.metadata.trace.encode_trace_id(writer, value.trace_id)
    writer.write_bool(value.is_noscan)
    writer.write_bool(value.has_shared_reference)
    encode_allocation_class(writer, value.class_)


def decode_allocation_site(reader: BinaryReader) -> AllocationSite:
    """Decode one AllocationSite."""
    byte_len = reader.read_number()
    alignment = reader.read_number()
    trace_id = reader.read_option(
        lambda: destack._generated.mir.metadata.trace.decode_trace_id(reader)
    )
    is_noscan = reader.read_bool()
    has_shared_reference = reader.read_bool()
    class_ = decode_allocation_class(reader)

    return AllocationSite(
        byte_len=byte_len,
        alignment=alignment,
        trace_id=trace_id,
        is_noscan=is_noscan,
        has_shared_reference=has_shared_reference,
        class_=class_,
    )


def to_json_allocation_site(value: AllocationSite) -> Json:
    """Return one JSON value for one AllocationSite."""
    return {
        "byteLen": value.byte_len,
        "alignment": value.alignment,
        **(
            {}
            if value.trace_id is None
            else {
                "traceId": destack._generated.mir.metadata.trace.to_json_trace_id(
                    value.trace_id
                )
            }
        ),
        "isNoscan": value.is_noscan,
        "hasSharedReference": value.has_shared_reference,
        "class": to_json_allocation_class(value.class_),
    }


def from_json_allocation_site(value: Json) -> AllocationSite:
    """Return one AllocationSite from one JSON value."""
    object_ = json_object(value)

    return AllocationSite(
        byte_len=json_int(json_field(object_, "byteLen")),
        alignment=json_int(json_field(object_, "alignment")),
        trace_id=json_optional(
            object_,
            "traceId",
            lambda value: destack._generated.mir.metadata.trace.from_json_trace_id(
                value
            ),
        ),
        is_noscan=json_bool(json_field(object_, "isNoscan")),
        has_shared_reference=json_bool(json_field(object_, "hasSharedReference")),
        class_=from_json_allocation_class(json_field(object_, "class")),
    )


@dataclass(frozen=True, slots=True)
class AllocationClassSmall:
    """One block backed by a small-span slot."""

    small: SmallAllocationPlan
    kind: typing.Literal["small"] = "small"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_allocation_class(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_allocation_class(self)


@dataclass(frozen=True, slots=True)
class AllocationClassLarge:
    """One block backed by a dedicated page span."""

    kind: typing.Literal["large"] = "large"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_allocation_class(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_allocation_class(self)


"""One allocator-ready allocation class."""
AllocationClass: typing.TypeAlias = AllocationClassSmall | AllocationClassLarge


def encode_allocation_class(writer: BinaryWriter, value: AllocationClass) -> None:
    """Encode one AllocationClass."""
    if value.kind == "small":
        writer.write_unsigned(0)
        encode_small_allocation_plan(writer, value.small)
    elif value.kind == "large":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_allocation_class(reader: BinaryReader) -> AllocationClass:
    """Decode one AllocationClass."""
    variant = reader.read_number()

    if variant == 0:
        small = decode_small_allocation_plan(reader)

        return AllocationClassSmall(small=small)
    elif variant == 1:
        return AllocationClassLarge()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_allocation_class(value: AllocationClass) -> Json:
    """Return one JSON value for one AllocationClass."""
    if value.kind == "small":
        return {
            "kind": "small",
            "small": to_json_small_allocation_plan(value.small),
        }
    elif value.kind == "large":
        return {
            "kind": "large",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_allocation_class(value: Json) -> AllocationClass:
    """Return one AllocationClass from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "small":
        return AllocationClassSmall(
            small=from_json_small_allocation_plan(json_field(object_, "small"))
        )
    elif kind == "large":
        return AllocationClassLarge()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class SmallAllocationPlan:
    """One allocator-ready small allocation plan."""

    # the exact mutator-cache index for this class
    cache_index: SmallCacheIndex
    # the smallest payload byte length routed to this class
    minimum_byte_len: int
    # the small-span class used by local and shared spaces
    class_: SmallSpanClass

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_small_allocation_plan(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SmallAllocationPlan:
        """Decode one SmallAllocationPlan."""
        return decode_small_allocation_plan(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_small_allocation_plan(self)

    @classmethod
    def from_json(cls, value: Json) -> SmallAllocationPlan:
        """Return one SmallAllocationPlan from one JSON value."""
        return from_json_small_allocation_plan(value)


def encode_small_allocation_plan(
    writer: BinaryWriter, value: SmallAllocationPlan
) -> None:
    """Encode one SmallAllocationPlan."""
    encode_small_cache_index(writer, value.cache_index)
    writer.write_unsigned(value.minimum_byte_len)
    encode_small_span_class(writer, value.class_)


def decode_small_allocation_plan(reader: BinaryReader) -> SmallAllocationPlan:
    """Decode one SmallAllocationPlan."""
    cache_index = decode_small_cache_index(reader)
    minimum_byte_len = reader.read_number()
    class_ = decode_small_span_class(reader)

    return SmallAllocationPlan(
        cache_index=cache_index,
        minimum_byte_len=minimum_byte_len,
        class_=class_,
    )


def to_json_small_allocation_plan(value: SmallAllocationPlan) -> Json:
    """Return one JSON value for one SmallAllocationPlan."""
    return {
        "cacheIndex": to_json_small_cache_index(value.cache_index),
        "minimumByteLen": value.minimum_byte_len,
        "class": to_json_small_span_class(value.class_),
    }


def from_json_small_allocation_plan(value: Json) -> SmallAllocationPlan:
    """Return one SmallAllocationPlan from one JSON value."""
    object_ = json_object(value)

    return SmallAllocationPlan(
        cache_index=from_json_small_cache_index(json_field(object_, "cacheIndex")),
        minimum_byte_len=json_int(json_field(object_, "minimumByteLen")),
        class_=from_json_small_span_class(json_field(object_, "class")),
    )


"""Dense mutator cache index for one small allocation site."""
SmallCacheIndex: typing.TypeAlias = int


def encode_small_cache_index(writer: BinaryWriter, value: SmallCacheIndex) -> None:
    """Encode one SmallCacheIndex."""
    writer.write_unsigned(value)


def decode_small_cache_index(reader: BinaryReader) -> SmallCacheIndex:
    """Decode one SmallCacheIndex."""
    return reader.read_number()


def to_json_small_cache_index(value: SmallCacheIndex) -> Json:
    """Return one JSON value for one SmallCacheIndex."""
    return value


def from_json_small_cache_index(value: Json) -> SmallCacheIndex:
    """Return one SmallCacheIndex from one JSON value."""
    return json_int(value)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_small_span_class(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SmallSpanClass:
        """Decode one SmallSpanClass."""
        return decode_small_span_class(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_small_span_class(self)

    @classmethod
    def from_json(cls, value: Json) -> SmallSpanClass:
        """Return one SmallSpanClass from one JSON value."""
        return from_json_small_span_class(value)


def encode_small_span_class(writer: BinaryWriter, value: SmallSpanClass) -> None:
    """Encode one SmallSpanClass."""
    writer.write_unsigned(value.size_class)
    writer.write_unsigned(value.span_size_bytes)
    if value.trace_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.metadata.trace.encode_trace_id(writer, value.trace_id)
    writer.write_bool(value.is_noscan)


def decode_small_span_class(reader: BinaryReader) -> SmallSpanClass:
    """Decode one SmallSpanClass."""
    size_class = reader.read_number()
    span_size_bytes = reader.read_number()
    trace_id = reader.read_option(
        lambda: destack._generated.mir.metadata.trace.decode_trace_id(reader)
    )
    is_noscan = reader.read_bool()

    return SmallSpanClass(
        size_class=size_class,
        span_size_bytes=span_size_bytes,
        trace_id=trace_id,
        is_noscan=is_noscan,
    )


def to_json_small_span_class(value: SmallSpanClass) -> Json:
    """Return one JSON value for one SmallSpanClass."""
    return {
        "sizeClass": value.size_class,
        "spanSizeBytes": value.span_size_bytes,
        **(
            {}
            if value.trace_id is None
            else {
                "traceId": destack._generated.mir.metadata.trace.to_json_trace_id(
                    value.trace_id
                )
            }
        ),
        "isNoscan": value.is_noscan,
    }


def from_json_small_span_class(value: Json) -> SmallSpanClass:
    """Return one SmallSpanClass from one JSON value."""
    object_ = json_object(value)

    return SmallSpanClass(
        size_class=json_int(json_field(object_, "sizeClass")),
        span_size_bytes=json_int(json_field(object_, "spanSizeBytes")),
        trace_id=json_optional(
            object_,
            "traceId",
            lambda value: destack._generated.mir.metadata.trace.from_json_trace_id(
                value
            ),
        ),
        is_noscan=json_bool(json_field(object_, "isNoscan")),
    )


@dataclass(frozen=True, slots=True)
class SmallAllocationSite:
    """One explicit small allocation site."""

    # the exact payload byte length
    byte_len: int
    # the resolved small allocation plan
    small: SmallAllocationPlan

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
    writer.write_unsigned(value.byte_len)
    encode_small_allocation_plan(writer, value.small)


def decode_small_allocation_site(reader: BinaryReader) -> SmallAllocationSite:
    """Decode one SmallAllocationSite."""
    byte_len = reader.read_number()
    small = decode_small_allocation_plan(reader)

    return SmallAllocationSite(
        byte_len=byte_len,
        small=small,
    )


def to_json_small_allocation_site(value: SmallAllocationSite) -> Json:
    """Return one JSON value for one SmallAllocationSite."""
    return {
        "byteLen": value.byte_len,
        "small": to_json_small_allocation_plan(value.small),
    }


def from_json_small_allocation_site(value: Json) -> SmallAllocationSite:
    """Return one SmallAllocationSite from one JSON value."""
    object_ = json_object(value)

    return SmallAllocationSite(
        byte_len=json_int(json_field(object_, "byteLen")),
        small=from_json_small_allocation_plan(json_field(object_, "small")),
    )


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
