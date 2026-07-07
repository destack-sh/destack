# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.index.postings
import destack._generated.dir.table.decorator
import destack._generated.dir.tree.node


@dataclass(frozen=True, slots=True)
class DecoratorIndex:
    """Indexed decorator applications."""

    # the decorator applications in stable order
    entries: Sequence[DecoratorEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorIndex:
        """Decode one DecoratorIndex."""
        return decode_decorator_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_index(self)

    @classmethod
    def from_json(cls, value: Json) -> DecoratorIndex:
        """Return one DecoratorIndex from one JSON value."""
        return from_json_decorator_index(value)


def encode_decorator_index(writer: BinaryWriter, value: DecoratorIndex) -> None:
    """Encode one DecoratorIndex."""
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_decorator_entry(writer, item_value_entries_0)


def decode_decorator_index(reader: BinaryReader) -> DecoratorIndex:
    """Decode one DecoratorIndex."""
    entries = [decode_decorator_entry(reader) for _ in range(reader.read_number())]

    return DecoratorIndex(
        entries=entries,
    )


def to_json_decorator_index(value: DecoratorIndex) -> Json:
    """Return one JSON value for one DecoratorIndex."""
    return {
        "entries": [to_json_decorator_entry(item_0) for item_0 in value.entries],
    }


def from_json_decorator_index(value: Json) -> DecoratorIndex:
    """Return one DecoratorIndex from one JSON value."""
    object_ = json_object(value)

    return DecoratorIndex(
        entries=[
            from_json_decorator_entry(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DecoratorEntry:
    """One indexed decorator application."""

    # the decorator name when syntactically known
    name: str | None
    # the local decorator application id
    application: destack._generated.dir.table.decorator.LocalDecoratorId
    # the decorator node
    decorator: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the decorator expression target
    expression: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the decorated target node
    target: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the resolved decorator target
    resolution: destack._generated.dir.table.decorator.DecoratorResolution

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorEntry:
        """Decode one DecoratorEntry."""
        return decode_decorator_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> DecoratorEntry:
        """Return one DecoratorEntry from one JSON value."""
        return from_json_decorator_entry(value)


def encode_decorator_entry(writer: BinaryWriter, value: DecoratorEntry) -> None:
    """Encode one DecoratorEntry."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
    destack._generated.dir.table.decorator.encode_local_decorator_id(
        writer, value.application
    )
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.decorator)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.expression)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.target)
    destack._generated.dir.table.decorator.encode_decorator_resolution(
        writer, value.resolution
    )


def decode_decorator_entry(reader: BinaryReader) -> DecoratorEntry:
    """Decode one DecoratorEntry."""
    name = reader.read_option(lambda: reader.read_string())
    application = destack._generated.dir.table.decorator.decode_local_decorator_id(
        reader
    )
    decorator = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    expression = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    target = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    resolution = destack._generated.dir.table.decorator.decode_decorator_resolution(
        reader
    )

    return DecoratorEntry(
        name=name,
        application=application,
        decorator=decorator,
        expression=expression,
        target=target,
        resolution=resolution,
    )


def to_json_decorator_entry(value: DecoratorEntry) -> Json:
    """Return one JSON value for one DecoratorEntry."""
    return {
        **({} if value.name is None else {"name": value.name}),
        "application": destack._generated.dir.table.decorator.to_json_local_decorator_id(
            value.application
        ),
        "decorator": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.decorator
        ),
        "expression": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.expression
        ),
        "target": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.target
        ),
        "resolution": destack._generated.dir.table.decorator.to_json_decorator_resolution(
            value.resolution
        ),
    }


def from_json_decorator_entry(value: Json) -> DecoratorEntry:
    """Return one DecoratorEntry from one JSON value."""
    object_ = json_object(value)

    return DecoratorEntry(
        name=json_optional(object_, "name", lambda value: json_string(value)),
        application=destack._generated.dir.table.decorator.from_json_local_decorator_id(
            json_field(object_, "application")
        ),
        decorator=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "decorator")
        ),
        expression=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "expression")
        ),
        target=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "target")
        ),
        resolution=destack._generated.dir.table.decorator.from_json_decorator_resolution(
            json_field(object_, "resolution")
        ),
    )


@dataclass(frozen=True, slots=True)
class DecoratorPostings:
    """Decorator postings by name."""

    # named decorator postings
    names: destack._generated.dir.index.postings.Postings
    # modules that contain unnamed decorators
    unnamed: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_postings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorPostings:
        """Decode one DecoratorPostings."""
        return decode_decorator_postings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_postings(self)

    @classmethod
    def from_json(cls, value: Json) -> DecoratorPostings:
        """Return one DecoratorPostings from one JSON value."""
        return from_json_decorator_postings(value)


def encode_decorator_postings(writer: BinaryWriter, value: DecoratorPostings) -> None:
    """Encode one DecoratorPostings."""
    destack._generated.dir.index.postings.encode_postings(writer, value.names)
    writer.write_unsigned(len(value.unnamed))
    for item_value_unnamed_0 in value.unnamed:
        writer.write_unsigned(item_value_unnamed_0)


def decode_decorator_postings(reader: BinaryReader) -> DecoratorPostings:
    """Decode one DecoratorPostings."""
    names = destack._generated.dir.index.postings.decode_postings(reader)
    unnamed = [reader.read_number() for _ in range(reader.read_number())]

    return DecoratorPostings(
        names=names,
        unnamed=unnamed,
    )


def to_json_decorator_postings(value: DecoratorPostings) -> Json:
    """Return one JSON value for one DecoratorPostings."""
    return {
        "names": destack._generated.dir.index.postings.to_json_postings(value.names),
        "unnamed": [item_0 for item_0 in value.unnamed],
    }


def from_json_decorator_postings(value: Json) -> DecoratorPostings:
    """Return one DecoratorPostings from one JSON value."""
    object_ = json_object(value)

    return DecoratorPostings(
        names=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "names")
        ),
        unnamed=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "unnamed"))
        ],
    )


__all__ = [
    "DecoratorIndex",
    "encode_decorator_index",
    "decode_decorator_index",
    "to_json_decorator_index",
    "from_json_decorator_index",
    "DecoratorEntry",
    "encode_decorator_entry",
    "decode_decorator_entry",
    "to_json_decorator_entry",
    "from_json_decorator_entry",
    "DecoratorPostings",
    "encode_decorator_postings",
    "decode_decorator_postings",
    "to_json_decorator_postings",
    "from_json_decorator_postings",
]
