# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class SourceIndexData:
    """Serialized source index shape."""

    enclosing_spans: Sequence[destack._generated.source.file.model.span.Span]
    main_spans: Sequence[destack._generated.source.file.model.span.Span | None]
    type_spans: Sequence[destack._generated.source.file.model.span.Span | None]
    side_spans: Mapping[NodeSpanKey, destack._generated.source.file.model.span.Span]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SourceIndexData: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SourceIndexData: ...

def encode_source_index_data(writer: BinaryWriter, value: SourceIndexData) -> None: ...
def decode_source_index_data(reader: BinaryReader) -> SourceIndexData: ...
def to_json_source_index_data(value: SourceIndexData) -> Json: ...
def from_json_source_index_data(value: Json) -> SourceIndexData: ...

@dataclass(frozen=True, slots=True)
class NodeSpanKey:
    """One typed node span keyed by source node id and span kind."""

    # the source node id that owns this node span
    source_id: int
    # the span kind within that source node
    span_type: NodeSpanType

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NodeSpanKey: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NodeSpanKey: ...

def encode_node_span_key(writer: BinaryWriter, value: NodeSpanKey) -> None: ...
def decode_node_span_key(reader: BinaryReader) -> NodeSpanKey: ...
def to_json_node_span_key(value: NodeSpanKey) -> Json: ...
def from_json_node_span_key(value: Json) -> NodeSpanKey: ...

@dataclass(frozen=True, slots=True)
class NodeSpanTypeEnclosing:
    """The enclosing span of a node."""

    kind: typing.Literal["enclosing"] = "enclosing"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class NodeSpanTypeMain:
    """The main span of a node (usually its identifier)."""

    kind: typing.Literal["main"] = "main"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class NodeSpanTypeHead:
    """The head span of a node."""

    kind: typing.Literal["head"] = "head"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class NodeSpanTypeBoundary:
    """One boundary span owned by a node."""

    boundary: NodeSpanBoundary
    kind: typing.Literal["boundary"] = "boundary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class NodeSpanTypeRegion:
    """One named region span within a node."""

    region: NodeSpanRegion
    kind: typing.Literal["region"] = "region"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class NodeSpanTypeListItem:
    """One indexed item span within a node."""

    field_0: NodeSpanList
    field_1: int
    kind: typing.Literal["listItem"] = "listItem"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""The type of span for a node."""
NodeSpanType: typing.TypeAlias = (
    NodeSpanTypeEnclosing
    | NodeSpanTypeMain
    | NodeSpanTypeHead
    | NodeSpanTypeBoundary
    | NodeSpanTypeRegion
    | NodeSpanTypeListItem
)

def encode_node_span_type(writer: BinaryWriter, value: NodeSpanType) -> None: ...
def decode_node_span_type(reader: BinaryReader) -> NodeSpanType: ...
def to_json_node_span_type(value: NodeSpanType) -> Json: ...
def from_json_node_span_type(value: Json) -> NodeSpanType: ...

"""A source boundary owned by one node."""
NodeSpanBoundary: typing.TypeAlias = (
    typing.Literal["leading"]
    | typing.Literal["leadingOperator"]
    | typing.Literal["trailing"]
)

def encode_node_span_boundary(
    writer: BinaryWriter, value: NodeSpanBoundary
) -> None: ...
def decode_node_span_boundary(reader: BinaryReader) -> NodeSpanBoundary: ...
def to_json_node_span_boundary(value: NodeSpanBoundary) -> Json: ...
def from_json_node_span_boundary(value: Json) -> NodeSpanBoundary: ...

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

def encode_node_span_region(writer: BinaryWriter, value: NodeSpanRegion) -> None: ...
def decode_node_span_region(reader: BinaryReader) -> NodeSpanRegion: ...
def to_json_node_span_region(value: NodeSpanRegion) -> Json: ...
def from_json_node_span_region(value: Json) -> NodeSpanRegion: ...

"""An indexed source list within one node."""
NodeSpanList: typing.TypeAlias = typing.Literal["segment"] | typing.Literal["entry"]

def encode_node_span_list(writer: BinaryWriter, value: NodeSpanList) -> None: ...
def decode_node_span_list(reader: BinaryReader) -> NodeSpanList: ...
def to_json_node_span_list(value: NodeSpanList) -> Json: ...
def from_json_node_span_list(value: Json) -> NodeSpanList: ...

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
