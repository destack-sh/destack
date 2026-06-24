# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
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
    json_optional,
    json_string,
    nested_bytes,
)

import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class SourceIndexData:
    """Serialized source index shape."""

    enclosing_spans: Sequence[destack._generated.source.file.model.span.Span]
    main_spans: Sequence[destack._generated.source.file.model.span.Span | None]
    type_spans: Sequence[destack._generated.source.file.model.span.Span | None]
    side_spans: Mapping[NodeSpanKey, destack._generated.source.file.model.span.Span]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_source_index_data(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SourceIndexData:
        """Decode one SourceIndexData."""
        return decode_source_index_data(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_source_index_data(self)

    @classmethod
    def from_json(cls, value: Json) -> SourceIndexData:
        """Return one SourceIndexData from one JSON value."""
        return from_json_source_index_data(value)


def encode_source_index_data(writer: BinaryWriter, value: SourceIndexData) -> None:
    """Encode one SourceIndexData."""
    writer.write_unsigned(len(value.enclosing_spans))
    for item_value_enclosing_spans_0 in value.enclosing_spans:
        destack._generated.source.file.model.span.encode_span(
            writer, item_value_enclosing_spans_0
        )
    writer.write_unsigned(len(value.main_spans))
    for item_value_main_spans_0 in value.main_spans:
        if item_value_main_spans_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.source.file.model.span.encode_span(
                writer, item_value_main_spans_0
            )
    writer.write_unsigned(len(value.type_spans))
    for item_value_type_spans_0 in value.type_spans:
        if item_value_type_spans_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.source.file.model.span.encode_span(
                writer, item_value_type_spans_0
            )
    entries_value_side_spans_0 = []
    for key_value_side_spans_0, item_value_side_spans_0 in value.side_spans.items():

        def write_key_value_side_spans_0(writer: BinaryWriter) -> None:
            encode_node_span_key(writer, key_value_side_spans_0)

        key_bytes = nested_bytes(write_key_value_side_spans_0)
        entries_value_side_spans_0.append(
            (key_value_side_spans_0, item_value_side_spans_0, key_bytes)
        )
    entries_value_side_spans_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_side_spans_0))
    for entry_value_side_spans_0 in entries_value_side_spans_0:
        encode_node_span_key(writer, entry_value_side_spans_0[0])
        destack._generated.source.file.model.span.encode_span(
            writer, entry_value_side_spans_0[1]
        )


def decode_source_index_data(reader: BinaryReader) -> SourceIndexData:
    """Decode one SourceIndexData."""
    enclosing_spans = [
        destack._generated.source.file.model.span.decode_span(reader)
        for _ in range(reader.read_number())
    ]
    main_spans = [
        reader.read_option(
            lambda: destack._generated.source.file.model.span.decode_span(reader)
        )
        for _ in range(reader.read_number())
    ]
    type_spans = [
        reader.read_option(
            lambda: destack._generated.source.file.model.span.decode_span(reader)
        )
        for _ in range(reader.read_number())
    ]
    side_spans = {
        decode_node_span_key(
            reader
        ): destack._generated.source.file.model.span.decode_span(reader)
        for _ in range(reader.read_number())
    }

    return SourceIndexData(
        enclosing_spans=enclosing_spans,
        main_spans=main_spans,
        type_spans=type_spans,
        side_spans=side_spans,
    )


def to_json_source_index_data(value: SourceIndexData) -> Json:
    """Return one JSON value for one SourceIndexData."""
    return {
        "enclosingSpans": [
            destack._generated.source.file.model.span.to_json_span(item_0)
            for item_0 in value.enclosing_spans
        ],
        "mainSpans": [
            None
            if item_0 is None
            else destack._generated.source.file.model.span.to_json_span(item_0)
            for item_0 in value.main_spans
        ],
        "typeSpans": [
            None
            if item_0 is None
            else destack._generated.source.file.model.span.to_json_span(item_0)
            for item_0 in value.type_spans
        ],
        "sideSpans": [
            [
                to_json_node_span_key(key_0),
                destack._generated.source.file.model.span.to_json_span(item_0),
            ]
            for key_0, item_0 in value.side_spans.items()
        ],
    }


def from_json_source_index_data(value: Json) -> SourceIndexData:
    """Return one SourceIndexData from one JSON value."""
    object_ = json_object(value)

    return SourceIndexData(
        enclosing_spans=[
            destack._generated.source.file.model.span.from_json_span(item_0)
            for item_0 in json_array(json_field(object_, "enclosingSpans"))
        ],
        main_spans=[
            None
            if item_0 is None
            else destack._generated.source.file.model.span.from_json_span(item_0)
            for item_0 in json_array(json_field(object_, "mainSpans"))
        ],
        type_spans=[
            None
            if item_0 is None
            else destack._generated.source.file.model.span.from_json_span(item_0)
            for item_0 in json_array(json_field(object_, "typeSpans"))
        ],
        side_spans={
            from_json_node_span_key(
                key_0
            ): destack._generated.source.file.model.span.from_json_span(item_0)
            for key_0, item_0 in json_array(json_field(object_, "sideSpans"))
        },
    )


@dataclass(frozen=True, slots=True)
class NodeSpanKey:
    """One typed node span keyed by source node id and span kind."""

    # the source node id that owns this node span
    source_id: int
    # the span kind within that source node
    span_type: NodeSpanType

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_node_span_key(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NodeSpanKey:
        """Decode one NodeSpanKey."""
        return decode_node_span_key(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_node_span_key(self)

    @classmethod
    def from_json(cls, value: Json) -> NodeSpanKey:
        """Return one NodeSpanKey from one JSON value."""
        return from_json_node_span_key(value)


def encode_node_span_key(writer: BinaryWriter, value: NodeSpanKey) -> None:
    """Encode one NodeSpanKey."""
    writer.write_unsigned(value.source_id)
    encode_node_span_type(writer, value.span_type)


def decode_node_span_key(reader: BinaryReader) -> NodeSpanKey:
    """Decode one NodeSpanKey."""
    source_id = reader.read_number()
    span_type = decode_node_span_type(reader)

    return NodeSpanKey(
        source_id=source_id,
        span_type=span_type,
    )


def to_json_node_span_key(value: NodeSpanKey) -> Json:
    """Return one JSON value for one NodeSpanKey."""
    return {
        "sourceId": value.source_id,
        "spanType": to_json_node_span_type(value.span_type),
    }


def from_json_node_span_key(value: Json) -> NodeSpanKey:
    """Return one NodeSpanKey from one JSON value."""
    object_ = json_object(value)

    return NodeSpanKey(
        source_id=json_int(json_field(object_, "sourceId")),
        span_type=from_json_node_span_type(json_field(object_, "spanType")),
    )


@dataclass(frozen=True, slots=True)
class NodeSpanTypeEnclosing:
    """The enclosing span of a node."""

    kind: typing.Literal["enclosing"] = "enclosing"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_node_span_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_node_span_type(self)


@dataclass(frozen=True, slots=True)
class NodeSpanTypeMain:
    """The main span of a node (usually its identifier)."""

    kind: typing.Literal["main"] = "main"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_node_span_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_node_span_type(self)


@dataclass(frozen=True, slots=True)
class NodeSpanTypeHead:
    """The head span of a node."""

    kind: typing.Literal["head"] = "head"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_node_span_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_node_span_type(self)


@dataclass(frozen=True, slots=True)
class NodeSpanTypeBoundary:
    """One boundary span owned by a node."""

    boundary: NodeSpanBoundary
    kind: typing.Literal["boundary"] = "boundary"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_node_span_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_node_span_type(self)


@dataclass(frozen=True, slots=True)
class NodeSpanTypeRegion:
    """One named region span within a node."""

    region: NodeSpanRegion
    kind: typing.Literal["region"] = "region"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_node_span_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_node_span_type(self)


@dataclass(frozen=True, slots=True)
class NodeSpanTypeListItem:
    """One indexed item span within a node."""

    field_0: NodeSpanList
    field_1: int
    kind: typing.Literal["listItem"] = "listItem"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_node_span_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_node_span_type(self)


"""The type of span for a node."""
NodeSpanType: typing.TypeAlias = (
    NodeSpanTypeEnclosing
    | NodeSpanTypeMain
    | NodeSpanTypeHead
    | NodeSpanTypeBoundary
    | NodeSpanTypeRegion
    | NodeSpanTypeListItem
)


def encode_node_span_type(writer: BinaryWriter, value: NodeSpanType) -> None:
    """Encode one NodeSpanType."""
    if value.kind == "enclosing":
        writer.write_unsigned(0)
    elif value.kind == "main":
        writer.write_unsigned(1)
    elif value.kind == "head":
        writer.write_unsigned(2)
    elif value.kind == "boundary":
        writer.write_unsigned(3)
        encode_node_span_boundary(writer, value.boundary)
    elif value.kind == "region":
        writer.write_unsigned(4)
        encode_node_span_region(writer, value.region)
    elif value.kind == "listItem":
        writer.write_unsigned(5)
        encode_node_span_list(writer, value.field_0)
        writer.write_unsigned(value.field_1)
    else:
        raise SerdeError("unknown enum variant")


def decode_node_span_type(reader: BinaryReader) -> NodeSpanType:
    """Decode one NodeSpanType."""
    variant = reader.read_number()

    if variant == 0:
        return NodeSpanTypeEnclosing()
    elif variant == 1:
        return NodeSpanTypeMain()
    elif variant == 2:
        return NodeSpanTypeHead()
    elif variant == 3:
        boundary = decode_node_span_boundary(reader)

        return NodeSpanTypeBoundary(boundary=boundary)
    elif variant == 4:
        region = decode_node_span_region(reader)

        return NodeSpanTypeRegion(region=region)
    elif variant == 5:
        field_0 = decode_node_span_list(reader)
        field_1 = reader.read_number()

        return NodeSpanTypeListItem(
            field_0=field_0,
            field_1=field_1,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_node_span_type(value: NodeSpanType) -> Json:
    """Return one JSON value for one NodeSpanType."""
    if value.kind == "enclosing":
        return {
            "kind": "enclosing",
        }
    elif value.kind == "main":
        return {
            "kind": "main",
        }
    elif value.kind == "head":
        return {
            "kind": "head",
        }
    elif value.kind == "boundary":
        return {
            "kind": "boundary",
            "boundary": to_json_node_span_boundary(value.boundary),
        }
    elif value.kind == "region":
        return {
            "kind": "region",
            "region": to_json_node_span_region(value.region),
        }
    elif value.kind == "listItem":
        return {
            "kind": "listItem",
            "0": to_json_node_span_list(value.field_0),
            "1": value.field_1,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_node_span_type(value: Json) -> NodeSpanType:
    """Return one NodeSpanType from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "enclosing":
        return NodeSpanTypeEnclosing()
    elif kind == "main":
        return NodeSpanTypeMain()
    elif kind == "head":
        return NodeSpanTypeHead()
    elif kind == "boundary":
        return NodeSpanTypeBoundary(
            boundary=from_json_node_span_boundary(json_field(object_, "boundary"))
        )
    elif kind == "region":
        return NodeSpanTypeRegion(
            region=from_json_node_span_region(json_field(object_, "region"))
        )
    elif kind == "listItem":
        return NodeSpanTypeListItem(
            field_0=from_json_node_span_list(json_field(object_, "0")),
            field_1=json_int(json_field(object_, "1")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""A source boundary owned by one node."""
NodeSpanBoundary: typing.TypeAlias = (
    typing.Literal["leading"]
    | typing.Literal["leadingOperator"]
    | typing.Literal["trailing"]
)


def encode_node_span_boundary(writer: BinaryWriter, value: NodeSpanBoundary) -> None:
    """Encode one NodeSpanBoundary."""
    if value == "leading":
        writer.write_unsigned(0)
    elif value == "leadingOperator":
        writer.write_unsigned(1)
    elif value == "trailing":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_node_span_boundary(reader: BinaryReader) -> NodeSpanBoundary:
    """Decode one NodeSpanBoundary."""
    variant = reader.read_number()

    if variant == 0:
        return "leading"
    elif variant == 1:
        return "leadingOperator"
    elif variant == 2:
        return "trailing"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_node_span_boundary(value: NodeSpanBoundary) -> Json:
    """Return one JSON value for one NodeSpanBoundary."""
    return value


def from_json_node_span_boundary(value: Json) -> NodeSpanBoundary:
    """Return one NodeSpanBoundary from one JSON value."""
    variant = json_string(value)

    if variant == "leading":
        return "leading"
    elif variant == "leadingOperator":
        return "leadingOperator"
    elif variant == "trailing":
        return "trailing"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""A named source region within one node."""
NodeSpanRegion: typing.TypeAlias = (
    typing.Literal["opening"]
    | typing.Literal["genericParameters"]
    | typing.Literal["parameters"]
    | typing.Literal["body"]
    | typing.Literal["statement"]
    | typing.Literal["clause"]
    | typing.Literal["prelude"]
    | typing.Literal["name"]
    | typing.Literal["value"]
    | typing.Literal["type"]
    | typing.Literal["wrapper"]
)


def encode_node_span_region(writer: BinaryWriter, value: NodeSpanRegion) -> None:
    """Encode one NodeSpanRegion."""
    if value == "opening":
        writer.write_unsigned(0)
    elif value == "genericParameters":
        writer.write_unsigned(1)
    elif value == "parameters":
        writer.write_unsigned(2)
    elif value == "body":
        writer.write_unsigned(3)
    elif value == "statement":
        writer.write_unsigned(4)
    elif value == "clause":
        writer.write_unsigned(5)
    elif value == "prelude":
        writer.write_unsigned(6)
    elif value == "name":
        writer.write_unsigned(7)
    elif value == "value":
        writer.write_unsigned(8)
    elif value == "type":
        writer.write_unsigned(9)
    elif value == "wrapper":
        writer.write_unsigned(10)
    else:
        raise SerdeError("unknown enum variant")


def decode_node_span_region(reader: BinaryReader) -> NodeSpanRegion:
    """Decode one NodeSpanRegion."""
    variant = reader.read_number()

    if variant == 0:
        return "opening"
    elif variant == 1:
        return "genericParameters"
    elif variant == 2:
        return "parameters"
    elif variant == 3:
        return "body"
    elif variant == 4:
        return "statement"
    elif variant == 5:
        return "clause"
    elif variant == 6:
        return "prelude"
    elif variant == 7:
        return "name"
    elif variant == 8:
        return "value"
    elif variant == 9:
        return "type"
    elif variant == 10:
        return "wrapper"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_node_span_region(value: NodeSpanRegion) -> Json:
    """Return one JSON value for one NodeSpanRegion."""
    return value


def from_json_node_span_region(value: Json) -> NodeSpanRegion:
    """Return one NodeSpanRegion from one JSON value."""
    variant = json_string(value)

    if variant == "opening":
        return "opening"
    elif variant == "genericParameters":
        return "genericParameters"
    elif variant == "parameters":
        return "parameters"
    elif variant == "body":
        return "body"
    elif variant == "statement":
        return "statement"
    elif variant == "clause":
        return "clause"
    elif variant == "prelude":
        return "prelude"
    elif variant == "name":
        return "name"
    elif variant == "value":
        return "value"
    elif variant == "type":
        return "type"
    elif variant == "wrapper":
        return "wrapper"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""An indexed source list within one node."""
NodeSpanList: typing.TypeAlias = typing.Literal["segment"] | typing.Literal["entry"]


def encode_node_span_list(writer: BinaryWriter, value: NodeSpanList) -> None:
    """Encode one NodeSpanList."""
    if value == "segment":
        writer.write_unsigned(0)
    elif value == "entry":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_node_span_list(reader: BinaryReader) -> NodeSpanList:
    """Decode one NodeSpanList."""
    variant = reader.read_number()

    if variant == 0:
        return "segment"
    elif variant == 1:
        return "entry"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_node_span_list(value: NodeSpanList) -> Json:
    """Return one JSON value for one NodeSpanList."""
    return value


def from_json_node_span_list(value: Json) -> NodeSpanList:
    """Return one NodeSpanList from one JSON value."""
    variant = json_string(value)

    if variant == "segment":
        return "segment"
    elif variant == "entry":
        return "entry"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "SourceIndexData",
    "encode_source_index_data",
    "decode_source_index_data",
    "to_json_source_index_data",
    "from_json_source_index_data",
    "NodeSpanKey",
    "encode_node_span_key",
    "decode_node_span_key",
    "to_json_node_span_key",
    "from_json_node_span_key",
    "NodeSpanType",
    "encode_node_span_type",
    "decode_node_span_type",
    "to_json_node_span_type",
    "from_json_node_span_type",
    "NodeSpanTypeEnclosing",
    "NodeSpanTypeMain",
    "NodeSpanTypeHead",
    "NodeSpanTypeBoundary",
    "NodeSpanTypeRegion",
    "NodeSpanTypeListItem",
    "NodeSpanBoundary",
    "encode_node_span_boundary",
    "decode_node_span_boundary",
    "to_json_node_span_boundary",
    "from_json_node_span_boundary",
    "NodeSpanRegion",
    "encode_node_span_region",
    "decode_node_span_region",
    "to_json_node_span_region",
    "from_json_node_span_region",
    "NodeSpanList",
    "encode_node_span_list",
    "decode_node_span_list",
    "to_json_node_span_list",
    "from_json_node_span_list",
]
