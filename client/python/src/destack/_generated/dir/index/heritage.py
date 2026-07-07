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
import destack._generated.dir.type.type


@dataclass(frozen=True, slots=True)
class HeritageIndex:
    """Indexed nominal heritage edges."""

    # heritage edges ordered by base symbol
    by_base: Sequence[HeritageEntry]
    # heritage edges ordered by derived symbol
    by_derived: Sequence[HeritageEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_heritage_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HeritageIndex:
        """Decode one HeritageIndex."""
        return decode_heritage_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_heritage_index(self)

    @classmethod
    def from_json(cls, value: Json) -> HeritageIndex:
        """Return one HeritageIndex from one JSON value."""
        return from_json_heritage_index(value)


def encode_heritage_index(writer: BinaryWriter, value: HeritageIndex) -> None:
    """Encode one HeritageIndex."""
    writer.write_unsigned(len(value.by_base))
    for item_value_by_base_0 in value.by_base:
        encode_heritage_entry(writer, item_value_by_base_0)
    writer.write_unsigned(len(value.by_derived))
    for item_value_by_derived_0 in value.by_derived:
        encode_heritage_entry(writer, item_value_by_derived_0)


def decode_heritage_index(reader: BinaryReader) -> HeritageIndex:
    """Decode one HeritageIndex."""
    by_base = [decode_heritage_entry(reader) for _ in range(reader.read_number())]
    by_derived = [decode_heritage_entry(reader) for _ in range(reader.read_number())]

    return HeritageIndex(
        by_base=by_base,
        by_derived=by_derived,
    )


def to_json_heritage_index(value: HeritageIndex) -> Json:
    """Return one JSON value for one HeritageIndex."""
    return {
        "byBase": [to_json_heritage_entry(item_0) for item_0 in value.by_base],
        "byDerived": [to_json_heritage_entry(item_0) for item_0 in value.by_derived],
    }


def from_json_heritage_index(value: Json) -> HeritageIndex:
    """Return one HeritageIndex from one JSON value."""
    object_ = json_object(value)

    return HeritageIndex(
        by_base=[
            from_json_heritage_entry(item_0)
            for item_0 in json_array(json_field(object_, "byBase"))
        ],
        by_derived=[
            from_json_heritage_entry(item_0)
            for item_0 in json_array(json_field(object_, "byDerived"))
        ],
    )


@dataclass(frozen=True, slots=True)
class HeritageEntry:
    """One nominal heritage edge."""

    # the derived nominal symbol
    derived: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the inherited or implemented nominal symbol
    base: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source heritage node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the generic arguments used at the heritage site
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the heritage kind
    kind: HeritageKind

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_heritage_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HeritageEntry:
        """Decode one HeritageEntry."""
        return decode_heritage_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_heritage_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> HeritageEntry:
        """Return one HeritageEntry from one JSON value."""
        return from_json_heritage_entry(value)


def encode_heritage_entry(writer: BinaryWriter, value: HeritageEntry) -> None:
    """Encode one HeritageEntry."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.derived)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.base)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_arguments_0
        )
    encode_heritage_kind(writer, value.kind)


def decode_heritage_entry(reader: BinaryReader) -> HeritageEntry:
    """Decode one HeritageEntry."""
    derived = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    base = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    arguments = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]
    kind = decode_heritage_kind(reader)

    return HeritageEntry(
        derived=derived,
        base=base,
        source=source,
        arguments=arguments,
        kind=kind,
    )


def to_json_heritage_entry(value: HeritageEntry) -> Json:
    """Return one JSON value for one HeritageEntry."""
    return {
        "derived": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.derived
        ),
        "base": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.base
        ),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "arguments": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.arguments
        ],
        "kind": to_json_heritage_kind(value.kind),
    }


def from_json_heritage_entry(value: Json) -> HeritageEntry:
    """Return one HeritageEntry from one JSON value."""
    object_ = json_object(value)

    return HeritageEntry(
        derived=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "derived")
        ),
        base=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "base")
        ),
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        arguments=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
        kind=from_json_heritage_kind(json_field(object_, "kind")),
    )


"""Nominal heritage kind."""
HeritageKind: typing.TypeAlias = (
    typing.Literal["extends"] | typing.Literal["implements"]
)


def encode_heritage_kind(writer: BinaryWriter, value: HeritageKind) -> None:
    """Encode one HeritageKind."""
    if value == "extends":
        writer.write_unsigned(0)
    elif value == "implements":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_heritage_kind(reader: BinaryReader) -> HeritageKind:
    """Decode one HeritageKind."""
    variant = reader.read_number()

    if variant == 0:
        return "extends"
    elif variant == 1:
        return "implements"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_heritage_kind(value: HeritageKind) -> Json:
    """Return one JSON value for one HeritageKind."""
    return value


def from_json_heritage_kind(value: Json) -> HeritageKind:
    """Return one HeritageKind from one JSON value."""
    variant = json_string(value)

    if variant == "extends":
        return "extends"
    elif variant == "implements":
        return "implements"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class HeritagePostings:
    """Heritage postings by base symbol."""

    # heritage base postings
    bases: destack._generated.dir.index.postings.Postings
    # heritage derived postings
    derived: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_heritage_postings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HeritagePostings:
        """Decode one HeritagePostings."""
        return decode_heritage_postings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_heritage_postings(self)

    @classmethod
    def from_json(cls, value: Json) -> HeritagePostings:
        """Return one HeritagePostings from one JSON value."""
        return from_json_heritage_postings(value)


def encode_heritage_postings(writer: BinaryWriter, value: HeritagePostings) -> None:
    """Encode one HeritagePostings."""
    destack._generated.dir.index.postings.encode_postings(writer, value.bases)
    destack._generated.dir.index.postings.encode_postings(writer, value.derived)


def decode_heritage_postings(reader: BinaryReader) -> HeritagePostings:
    """Decode one HeritagePostings."""
    bases = destack._generated.dir.index.postings.decode_postings(reader)
    derived = destack._generated.dir.index.postings.decode_postings(reader)

    return HeritagePostings(
        bases=bases,
        derived=derived,
    )


def to_json_heritage_postings(value: HeritagePostings) -> Json:
    """Return one JSON value for one HeritagePostings."""
    return {
        "bases": destack._generated.dir.index.postings.to_json_postings(value.bases),
        "derived": destack._generated.dir.index.postings.to_json_postings(
            value.derived
        ),
    }


def from_json_heritage_postings(value: Json) -> HeritagePostings:
    """Return one HeritagePostings from one JSON value."""
    object_ = json_object(value)

    return HeritagePostings(
        bases=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "bases")
        ),
        derived=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "derived")
        ),
    )


__all__ = [
    "HeritageIndex",
    "encode_heritage_index",
    "decode_heritage_index",
    "to_json_heritage_index",
    "from_json_heritage_index",
    "HeritageEntry",
    "encode_heritage_entry",
    "decode_heritage_entry",
    "to_json_heritage_entry",
    "from_json_heritage_entry",
    "HeritageKind",
    "encode_heritage_kind",
    "decode_heritage_kind",
    "to_json_heritage_kind",
    "from_json_heritage_kind",
    "HeritagePostings",
    "encode_heritage_postings",
    "decode_heritage_postings",
    "to_json_heritage_postings",
    "from_json_heritage_postings",
]
