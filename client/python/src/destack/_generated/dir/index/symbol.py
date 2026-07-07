# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.file
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class SymbolIndex:
    """Indexed declared symbols."""

    # the symbols in stable display order
    entries: Sequence[SymbolEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolIndex:
        """Decode one SymbolIndex."""
        return decode_symbol_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol_index(self)

    @classmethod
    def from_json(cls, value: Json) -> SymbolIndex:
        """Return one SymbolIndex from one JSON value."""
        return from_json_symbol_index(value)


def encode_symbol_index(writer: BinaryWriter, value: SymbolIndex) -> None:
    """Encode one SymbolIndex."""
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_symbol_entry(writer, item_value_entries_0)


def decode_symbol_index(reader: BinaryReader) -> SymbolIndex:
    """Decode one SymbolIndex."""
    entries = [decode_symbol_entry(reader) for _ in range(reader.read_number())]

    return SymbolIndex(
        entries=entries,
    )


def to_json_symbol_index(value: SymbolIndex) -> Json:
    """Return one JSON value for one SymbolIndex."""
    return {
        "entries": [to_json_symbol_entry(item_0) for item_0 in value.entries],
    }


def from_json_symbol_index(value: Json) -> SymbolIndex:
    """Return one SymbolIndex from one JSON value."""
    object_ = json_object(value)

    return SymbolIndex(
        entries=[
            from_json_symbol_entry(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
    )


@dataclass(frozen=True, slots=True)
class SymbolEntry:
    """One indexed declared symbol."""

    # the display name
    name: str
    # the checked symbol kind
    kind: destack._generated.dir.symbol.symbol.SymbolKind
    # the checked symbol role
    role: destack._generated.dir.symbol.symbol.SymbolRole
    # the symbol id
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source node that declares the symbol
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source file
    file: destack._generated.source.file.model.file.FileId
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the checked type of the symbol when known
    ty: destack._generated.dir.type.type.GlobalTypeId | None
    # the containing declaration display name
    container: str | None
    # the binding mutability when this is a value binding
    mutability: destack._generated.dir.tree.node.Mutability | None
    # whether this symbol is exported from its declaring module
    is_exported: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolEntry:
        """Decode one SymbolEntry."""
        return decode_symbol_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> SymbolEntry:
        """Return one SymbolEntry from one JSON value."""
        return from_json_symbol_entry(value)


def encode_symbol_entry(writer: BinaryWriter, value: SymbolEntry) -> None:
    """Encode one SymbolEntry."""
    writer.write_string(value.name)
    destack._generated.dir.symbol.symbol.encode_symbol_kind(writer, value.kind)
    destack._generated.dir.symbol.symbol.encode_symbol_role(writer, value.role)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.source.file.model.file.encode_file_id(writer, value.file)
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    if value.ty is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    if value.container is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.container)
    if value.mutability is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
    writer.write_bool(value.is_exported)


def decode_symbol_entry(reader: BinaryReader) -> SymbolEntry:
    """Decode one SymbolEntry."""
    name = reader.read_string()
    kind = destack._generated.dir.symbol.symbol.decode_symbol_kind(reader)
    role = destack._generated.dir.symbol.symbol.decode_symbol_role(reader)
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    file = destack._generated.source.file.model.file.decode_file_id(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)
    ty = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
    )
    container = reader.read_option(lambda: reader.read_string())
    mutability = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_mutability(reader)
    )
    is_exported = reader.read_bool()

    return SymbolEntry(
        name=name,
        kind=kind,
        role=role,
        symbol=symbol,
        source=source,
        file=file,
        span=span,
        ty=ty,
        container=container,
        mutability=mutability,
        is_exported=is_exported,
    )


def to_json_symbol_entry(value: SymbolEntry) -> Json:
    """Return one JSON value for one SymbolEntry."""
    return {
        "name": value.name,
        "kind": destack._generated.dir.symbol.symbol.to_json_symbol_kind(value.kind),
        "role": destack._generated.dir.symbol.symbol.to_json_symbol_role(value.role),
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "file": destack._generated.source.file.model.file.to_json_file_id(value.file),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        **(
            {}
            if value.ty is None
            else {
                "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty)
            }
        ),
        **({} if value.container is None else {"container": value.container}),
        **(
            {}
            if value.mutability is None
            else {
                "mutability": destack._generated.dir.tree.node.to_json_mutability(
                    value.mutability
                )
            }
        ),
        "isExported": value.is_exported,
    }


def from_json_symbol_entry(value: Json) -> SymbolEntry:
    """Return one SymbolEntry from one JSON value."""
    object_ = json_object(value)

    return SymbolEntry(
        name=json_string(json_field(object_, "name")),
        kind=destack._generated.dir.symbol.symbol.from_json_symbol_kind(
            json_field(object_, "kind")
        ),
        role=destack._generated.dir.symbol.symbol.from_json_symbol_role(
            json_field(object_, "role")
        ),
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        file=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "file")
        ),
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        ty=json_optional(
            object_,
            "ty",
            lambda value: destack._generated.dir.type.type.from_json_global_type_id(
                value
            ),
        ),
        container=json_optional(object_, "container", lambda value: json_string(value)),
        mutability=json_optional(
            object_,
            "mutability",
            lambda value: destack._generated.dir.tree.node.from_json_mutability(value),
        ),
        is_exported=json_bool(json_field(object_, "isExported")),
    )


@dataclass(frozen=True, slots=True)
class SymbolPostings:
    """Symbol postings by name."""

    # symbol name postings
    names: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol_postings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolPostings:
        """Decode one SymbolPostings."""
        return decode_symbol_postings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol_postings(self)

    @classmethod
    def from_json(cls, value: Json) -> SymbolPostings:
        """Return one SymbolPostings from one JSON value."""
        return from_json_symbol_postings(value)


def encode_symbol_postings(writer: BinaryWriter, value: SymbolPostings) -> None:
    """Encode one SymbolPostings."""
    destack._generated.dir.index.postings.encode_postings(writer, value.names)


def decode_symbol_postings(reader: BinaryReader) -> SymbolPostings:
    """Decode one SymbolPostings."""
    names = destack._generated.dir.index.postings.decode_postings(reader)

    return SymbolPostings(
        names=names,
    )


def to_json_symbol_postings(value: SymbolPostings) -> Json:
    """Return one JSON value for one SymbolPostings."""
    return {
        "names": destack._generated.dir.index.postings.to_json_postings(value.names),
    }


def from_json_symbol_postings(value: Json) -> SymbolPostings:
    """Return one SymbolPostings from one JSON value."""
    object_ = json_object(value)

    return SymbolPostings(
        names=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "names")
        ),
    )


__all__ = [
    "SymbolIndex",
    "encode_symbol_index",
    "decode_symbol_index",
    "to_json_symbol_index",
    "from_json_symbol_index",
    "SymbolEntry",
    "encode_symbol_entry",
    "decode_symbol_entry",
    "to_json_symbol_entry",
    "from_json_symbol_entry",
    "SymbolPostings",
    "encode_symbol_postings",
    "decode_symbol_postings",
    "to_json_symbol_postings",
    "from_json_symbol_postings",
]
