# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_int,
    json_object,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_map(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_map(self)


@dataclass(frozen=True, slots=True)
class TraceMapFixed(TraceMapImpl):
    """Payload stores heap-reference words at fixed byte offsets."""

    # byte offsets of encoded local heap references
    local_offsets: Sequence[int]
    # byte offsets of encoded shared heap references
    shared_offsets: Sequence[int]
    kind: typing.Literal["fixed"] = "fixed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_map(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_map(self)


@dataclass(frozen=True, slots=True)
class TraceMapNested(TraceMapImpl):
    """Payload stores one nested map at a byte offset."""

    # the byte offset of the nested payload
    byte_offset: int
    # the nested trace map
    map: TraceMap
    kind: typing.Literal["nested"] = "nested"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_map(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_map(self)


@dataclass(frozen=True, slots=True)
class TraceMapComposite(TraceMapImpl):
    """Payload stores multiple nested maps."""

    # the nested trace maps
    maps: Sequence[TraceMap]
    kind: typing.Literal["composite"] = "composite"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_map(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_map(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_map(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_map(self)


@dataclass(frozen=True, slots=True)
class TraceMapTagged(TraceMapImpl):
    """Payload stores a tagged variant with variant-specific trace maps."""

    # the byte width of the variant tag
    tag_bytes: int
    # variant trace maps keyed by normalized tag value
    variants: Sequence[TraceVariant]
    kind: typing.Literal["tagged"] = "tagged"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_map(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_map(self)


"""Heap trace metadata for one runtime payload."""
TraceMap: typing.TypeAlias = (
    TraceMapEmpty
    | TraceMapFixed
    | TraceMapNested
    | TraceMapComposite
    | TraceMapRepeated
    | TraceMapTagged
)


def encode_trace_map(writer: BinaryWriter, value: TraceMap) -> None:
    """Encode one TraceMap."""
    if value.kind == "empty":
        writer.write_unsigned(0)
    elif value.kind == "fixed":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.local_offsets))
        for item_value_local_offsets_0 in value.local_offsets:
            writer.write_unsigned(item_value_local_offsets_0)
        writer.write_unsigned(len(value.shared_offsets))
        for item_value_shared_offsets_0 in value.shared_offsets:
            writer.write_unsigned(item_value_shared_offsets_0)
    elif value.kind == "nested":
        writer.write_unsigned(2)
        writer.write_unsigned(value.byte_offset)
        encode_trace_map(writer, value.map)
    elif value.kind == "composite":
        writer.write_unsigned(3)
        writer.write_unsigned(len(value.maps))
        for item_value_maps_0 in value.maps:
            encode_trace_map(writer, item_value_maps_0)
    elif value.kind == "repeated":
        writer.write_unsigned(4)
        writer.write_unsigned(value.count)
        writer.write_unsigned(value.stride)
        encode_trace_map(writer, value.element)
    elif value.kind == "tagged":
        writer.write_unsigned(5)
        writer.write_byte(value.tag_bytes)
        writer.write_unsigned(len(value.variants))
        for item_value_variants_0 in value.variants:
            encode_trace_variant(writer, item_value_variants_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_trace_map(reader: BinaryReader) -> TraceMap:
    """Decode one TraceMap."""
    variant = reader.read_number()

    if variant == 0:
        return TraceMapEmpty()
    elif variant == 1:
        local_offsets = [reader.read_number() for _ in range(reader.read_number())]
        shared_offsets = [reader.read_number() for _ in range(reader.read_number())]

        return TraceMapFixed(
            local_offsets=local_offsets,
            shared_offsets=shared_offsets,
        )
    elif variant == 2:
        byte_offset = reader.read_number()
        map = decode_trace_map(reader)

        return TraceMapNested(
            byte_offset=byte_offset,
            map=map,
        )
    elif variant == 3:
        maps = [decode_trace_map(reader) for _ in range(reader.read_number())]

        return TraceMapComposite(
            maps=maps,
        )
    elif variant == 4:
        count = reader.read_number()
        stride = reader.read_number()
        element = decode_trace_map(reader)

        return TraceMapRepeated(
            count=count,
            stride=stride,
            element=element,
        )
    elif variant == 5:
        tag_bytes = reader.read_byte()
        variants = [decode_trace_variant(reader) for _ in range(reader.read_number())]

        return TraceMapTagged(
            tag_bytes=tag_bytes,
            variants=variants,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_trace_map(value: TraceMap) -> Json:
    """Return one JSON value for one TraceMap."""
    if value.kind == "empty":
        return {
            "kind": "empty",
        }
    elif value.kind == "fixed":
        return {
            "kind": "fixed",
            "localOffsets": [item_0 for item_0 in value.local_offsets],
            "sharedOffsets": [item_0 for item_0 in value.shared_offsets],
        }
    elif value.kind == "nested":
        return {
            "kind": "nested",
            "byteOffset": value.byte_offset,
            "map": to_json_trace_map(value.map),
        }
    elif value.kind == "composite":
        return {
            "kind": "composite",
            "maps": [to_json_trace_map(item_0) for item_0 in value.maps],
        }
    elif value.kind == "repeated":
        return {
            "kind": "repeated",
            "count": value.count,
            "stride": value.stride,
            "element": to_json_trace_map(value.element),
        }
    elif value.kind == "tagged":
        return {
            "kind": "tagged",
            "tagBytes": value.tag_bytes,
            "variants": [to_json_trace_variant(item_0) for item_0 in value.variants],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_trace_map(value: Json) -> TraceMap:
    """Return one TraceMap from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "empty":
        return TraceMapEmpty()
    elif kind == "fixed":
        return TraceMapFixed(
            local_offsets=[
                json_int(item_0)
                for item_0 in json_array(json_field(object_, "localOffsets"))
            ],
            shared_offsets=[
                json_int(item_0)
                for item_0 in json_array(json_field(object_, "sharedOffsets"))
            ],
        )
    elif kind == "nested":
        return TraceMapNested(
            byte_offset=json_int(json_field(object_, "byteOffset")),
            map=from_json_trace_map(json_field(object_, "map")),
        )
    elif kind == "composite":
        return TraceMapComposite(
            maps=[
                from_json_trace_map(item_0)
                for item_0 in json_array(json_field(object_, "maps"))
            ],
        )
    elif kind == "repeated":
        return TraceMapRepeated(
            count=json_int(json_field(object_, "count")),
            stride=json_int(json_field(object_, "stride")),
            element=from_json_trace_map(json_field(object_, "element")),
        )
    elif kind == "tagged":
        return TraceMapTagged(
            tag_bytes=json_int(json_field(object_, "tagBytes")),
            variants=[
                from_json_trace_variant(item_0)
                for item_0 in json_array(json_field(object_, "variants"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TraceVariant:
    """One tag-selected trace variant."""

    # the normalized numeric tag value selecting this variant
    tag: int
    # the byte offset of the variant payload
    payload_offset: int
    # the payload trace map for this variant
    map: TraceMap

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_variant(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceVariant:
        """Decode one TraceVariant."""
        return decode_trace_variant(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_variant(self)

    @classmethod
    def from_json(cls, value: Json) -> TraceVariant:
        """Return one TraceVariant from one JSON value."""
        return from_json_trace_variant(value)


def encode_trace_variant(writer: BinaryWriter, value: TraceVariant) -> None:
    """Encode one TraceVariant."""
    writer.write_unsigned(value.tag)
    writer.write_unsigned(value.payload_offset)
    encode_trace_map(writer, value.map)


def decode_trace_variant(reader: BinaryReader) -> TraceVariant:
    """Decode one TraceVariant."""
    tag = reader.read_number()
    payload_offset = reader.read_number()
    map = decode_trace_map(reader)

    return TraceVariant(
        tag=tag,
        payload_offset=payload_offset,
        map=map,
    )


def to_json_trace_variant(value: TraceVariant) -> Json:
    """Return one JSON value for one TraceVariant."""
    return {
        "tag": value.tag,
        "payloadOffset": value.payload_offset,
        "map": to_json_trace_map(value.map),
    }


def from_json_trace_variant(value: Json) -> TraceVariant:
    """Return one TraceVariant from one JSON value."""
    object_ = json_object(value)

    return TraceVariant(
        tag=json_int(json_field(object_, "tag")),
        payload_offset=json_int(json_field(object_, "payloadOffset")),
        map=from_json_trace_map(json_field(object_, "map")),
    )


@dataclass(frozen=True, slots=True)
class TraceTable(TraceTableImpl):
    """Shared table of heap trace maps for one lowered program."""

    # trace maps indexed by TraceId
    traces: Sequence[TraceMap]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceTable:
        """Decode one TraceTable."""
        return decode_trace_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_table(self)

    @classmethod
    def from_json(cls, value: Json) -> TraceTable:
        """Return one TraceTable from one JSON value."""
        return from_json_trace_table(value)


def encode_trace_table(writer: BinaryWriter, value: TraceTable) -> None:
    """Encode one TraceTable."""
    writer.write_unsigned(len(value.traces))
    for item_value_traces_0 in value.traces:
        encode_trace_map(writer, item_value_traces_0)


def decode_trace_table(reader: BinaryReader) -> TraceTable:
    """Decode one TraceTable."""
    traces = [decode_trace_map(reader) for _ in range(reader.read_number())]

    return TraceTable(
        traces=traces,
    )


def to_json_trace_table(value: TraceTable) -> Json:
    """Return one JSON value for one TraceTable."""
    return {
        "traces": [to_json_trace_map(item_0) for item_0 in value.traces],
    }


def from_json_trace_table(value: Json) -> TraceTable:
    """Return one TraceTable from one JSON value."""
    object_ = json_object(value)

    return TraceTable(
        traces=[
            from_json_trace_map(item_0)
            for item_0 in json_array(json_field(object_, "traces"))
        ],
    )


"""Stable non-zero identifier for one heap trace map."""
TraceId: typing.TypeAlias = int


def encode_trace_id(writer: BinaryWriter, value: TraceId) -> None:
    """Encode one TraceId."""
    writer.write_unsigned(value)


def decode_trace_id(reader: BinaryReader) -> TraceId:
    """Decode one TraceId."""
    return reader.read_number()


def to_json_trace_id(value: TraceId) -> Json:
    """Return one JSON value for one TraceId."""
    return value


def from_json_trace_id(value: Json) -> TraceId:
    """Return one TraceId from one JSON value."""
    return json_int(value)


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
