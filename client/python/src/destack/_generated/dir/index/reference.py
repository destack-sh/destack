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
    json_object,
    json_string,
)

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class ReferenceIndex:
    """Reference occurrence index."""

    # the references ordered by target symbol
    by_target: Sequence[ReferenceEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceIndex:
        """Decode one ReferenceIndex."""
        return decode_reference_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference_index(self)

    @classmethod
    def from_json(cls, value: Json) -> ReferenceIndex:
        """Return one ReferenceIndex from one JSON value."""
        return from_json_reference_index(value)


def encode_reference_index(writer: BinaryWriter, value: ReferenceIndex) -> None:
    """Encode one ReferenceIndex."""
    writer.write_unsigned(len(value.by_target))
    for item_value_by_target_0 in value.by_target:
        encode_reference_entry(writer, item_value_by_target_0)


def decode_reference_index(reader: BinaryReader) -> ReferenceIndex:
    """Decode one ReferenceIndex."""
    by_target = [decode_reference_entry(reader) for _ in range(reader.read_number())]

    return ReferenceIndex(
        by_target=by_target,
    )


def to_json_reference_index(value: ReferenceIndex) -> Json:
    """Return one JSON value for one ReferenceIndex."""
    return {
        "byTarget": [to_json_reference_entry(item_0) for item_0 in value.by_target],
    }


def from_json_reference_index(value: Json) -> ReferenceIndex:
    """Return one ReferenceIndex from one JSON value."""
    object_ = json_object(value)

    return ReferenceIndex(
        by_target=[
            from_json_reference_entry(item_0)
            for item_0 in json_array(json_field(object_, "byTarget"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ReferenceEntry:
    """One indexed reference occurrence."""

    # the referenced symbol
    target: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source node that references the symbol
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the reference kind
    kind: ReferenceKind

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceEntry:
        """Decode one ReferenceEntry."""
        return decode_reference_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> ReferenceEntry:
        """Return one ReferenceEntry from one JSON value."""
        return from_json_reference_entry(value)


def encode_reference_entry(writer: BinaryWriter, value: ReferenceEntry) -> None:
    """Encode one ReferenceEntry."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.target)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    encode_reference_kind(writer, value.kind)


def decode_reference_entry(reader: BinaryReader) -> ReferenceEntry:
    """Decode one ReferenceEntry."""
    target = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)
    kind = decode_reference_kind(reader)

    return ReferenceEntry(
        target=target,
        source=source,
        span=span,
        kind=kind,
    )


def to_json_reference_entry(value: ReferenceEntry) -> Json:
    """Return one JSON value for one ReferenceEntry."""
    return {
        "target": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.target
        ),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        "kind": to_json_reference_kind(value.kind),
    }


def from_json_reference_entry(value: Json) -> ReferenceEntry:
    """Return one ReferenceEntry from one JSON value."""
    object_ = json_object(value)

    return ReferenceEntry(
        target=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "target")
        ),
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        kind=from_json_reference_kind(json_field(object_, "kind")),
    )


"""Kind of indexed reference occurrence."""
ReferenceKind: typing.TypeAlias = (
    typing.Literal["name"]
    | typing.Literal["member"]
    | typing.Literal["call"]
    | typing.Literal["construct"]
    | typing.Literal["dependency"]
    | typing.Literal["type"]
    | typing.Literal["storage"]
)


def encode_reference_kind(writer: BinaryWriter, value: ReferenceKind) -> None:
    """Encode one ReferenceKind."""
    if value == "name":
        writer.write_unsigned(0)
    elif value == "member":
        writer.write_unsigned(1)
    elif value == "call":
        writer.write_unsigned(2)
    elif value == "construct":
        writer.write_unsigned(3)
    elif value == "dependency":
        writer.write_unsigned(4)
    elif value == "type":
        writer.write_unsigned(5)
    elif value == "storage":
        writer.write_unsigned(6)
    else:
        raise SerdeError("unknown enum variant")


def decode_reference_kind(reader: BinaryReader) -> ReferenceKind:
    """Decode one ReferenceKind."""
    variant = reader.read_number()

    if variant == 0:
        return "name"
    elif variant == 1:
        return "member"
    elif variant == 2:
        return "call"
    elif variant == 3:
        return "construct"
    elif variant == 4:
        return "dependency"
    elif variant == 5:
        return "type"
    elif variant == 6:
        return "storage"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_reference_kind(value: ReferenceKind) -> Json:
    """Return one JSON value for one ReferenceKind."""
    return value


def from_json_reference_kind(value: Json) -> ReferenceKind:
    """Return one ReferenceKind from one JSON value."""
    variant = json_string(value)

    if variant == "name":
        return "name"
    elif variant == "member":
        return "member"
    elif variant == "call":
        return "call"
    elif variant == "construct":
        return "construct"
    elif variant == "dependency":
        return "dependency"
    elif variant == "type":
        return "type"
    elif variant == "storage":
        return "storage"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ReferencePostings:
    """Reference postings by target symbol."""

    # reference target postings
    targets: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference_postings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferencePostings:
        """Decode one ReferencePostings."""
        return decode_reference_postings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference_postings(self)

    @classmethod
    def from_json(cls, value: Json) -> ReferencePostings:
        """Return one ReferencePostings from one JSON value."""
        return from_json_reference_postings(value)


def encode_reference_postings(writer: BinaryWriter, value: ReferencePostings) -> None:
    """Encode one ReferencePostings."""
    destack._generated.dir.index.postings.encode_postings(writer, value.targets)


def decode_reference_postings(reader: BinaryReader) -> ReferencePostings:
    """Decode one ReferencePostings."""
    targets = destack._generated.dir.index.postings.decode_postings(reader)

    return ReferencePostings(
        targets=targets,
    )


def to_json_reference_postings(value: ReferencePostings) -> Json:
    """Return one JSON value for one ReferencePostings."""
    return {
        "targets": destack._generated.dir.index.postings.to_json_postings(
            value.targets
        ),
    }


def from_json_reference_postings(value: Json) -> ReferencePostings:
    """Return one ReferencePostings from one JSON value."""
    object_ = json_object(value)

    return ReferencePostings(
        targets=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "targets")
        ),
    )


__all__ = [
    "ReferenceIndex",
    "encode_reference_index",
    "decode_reference_index",
    "to_json_reference_index",
    "from_json_reference_index",
    "ReferenceEntry",
    "encode_reference_entry",
    "decode_reference_entry",
    "to_json_reference_entry",
    "from_json_reference_entry",
    "ReferenceKind",
    "encode_reference_kind",
    "decode_reference_kind",
    "to_json_reference_kind",
    "from_json_reference_kind",
    "ReferencePostings",
    "encode_reference_postings",
    "decode_reference_postings",
    "to_json_reference_postings",
    "from_json_reference_postings",
]
