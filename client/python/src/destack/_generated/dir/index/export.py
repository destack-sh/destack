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
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.file
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class ExportIndex:
    """Indexed exported symbols."""

    # the exports in stable display order
    entries: Sequence[ExportEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportIndex:
        """Decode one ExportIndex."""
        return decode_export_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_index(self)

    @classmethod
    def from_json(cls, value: Json) -> ExportIndex:
        """Return one ExportIndex from one JSON value."""
        return from_json_export_index(value)


def encode_export_index(writer: BinaryWriter, value: ExportIndex) -> None:
    """Encode one ExportIndex."""
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_export_entry(writer, item_value_entries_0)


def decode_export_index(reader: BinaryReader) -> ExportIndex:
    """Decode one ExportIndex."""
    entries = [decode_export_entry(reader) for _ in range(reader.read_number())]

    return ExportIndex(
        entries=entries,
    )


def to_json_export_index(value: ExportIndex) -> Json:
    """Return one JSON value for one ExportIndex."""
    return {
        "entries": [to_json_export_entry(item_0) for item_0 in value.entries],
    }


def from_json_export_index(value: Json) -> ExportIndex:
    """Return one ExportIndex from one JSON value."""
    object_ = json_object(value)

    return ExportIndex(
        entries=[
            from_json_export_entry(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ExportEntry:
    """One indexed exported symbol."""

    # the exported symbol name
    name: str
    # the exported symbol kind
    kind: destack._generated.dir.symbol.symbol.SymbolKind
    # the resolved exported symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source node that exposes this export
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source file
    file: destack._generated.source.file.model.file.FileId
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the module path to use in imports
    module_path: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportEntry:
        """Decode one ExportEntry."""
        return decode_export_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> ExportEntry:
        """Return one ExportEntry from one JSON value."""
        return from_json_export_entry(value)


def encode_export_entry(writer: BinaryWriter, value: ExportEntry) -> None:
    """Encode one ExportEntry."""
    writer.write_string(value.name)
    destack._generated.dir.symbol.symbol.encode_symbol_kind(writer, value.kind)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.source.file.model.file.encode_file_id(writer, value.file)
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    if value.module_path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.module_path)


def decode_export_entry(reader: BinaryReader) -> ExportEntry:
    """Decode one ExportEntry."""
    name = reader.read_string()
    kind = destack._generated.dir.symbol.symbol.decode_symbol_kind(reader)
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    file = destack._generated.source.file.model.file.decode_file_id(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)
    module_path = reader.read_option(lambda: reader.read_string())

    return ExportEntry(
        name=name,
        kind=kind,
        symbol=symbol,
        source=source,
        file=file,
        span=span,
        module_path=module_path,
    )


def to_json_export_entry(value: ExportEntry) -> Json:
    """Return one JSON value for one ExportEntry."""
    return {
        "name": value.name,
        "kind": destack._generated.dir.symbol.symbol.to_json_symbol_kind(value.kind),
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "file": destack._generated.source.file.model.file.to_json_file_id(value.file),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        **({} if value.module_path is None else {"modulePath": value.module_path}),
    }


def from_json_export_entry(value: Json) -> ExportEntry:
    """Return one ExportEntry from one JSON value."""
    object_ = json_object(value)

    return ExportEntry(
        name=json_string(json_field(object_, "name")),
        kind=destack._generated.dir.symbol.symbol.from_json_symbol_kind(
            json_field(object_, "kind")
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
        module_path=json_optional(
            object_, "modulePath", lambda value: json_string(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class ExportPostings:
    """Export postings by name."""

    # export name postings
    names: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_postings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportPostings:
        """Decode one ExportPostings."""
        return decode_export_postings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_postings(self)

    @classmethod
    def from_json(cls, value: Json) -> ExportPostings:
        """Return one ExportPostings from one JSON value."""
        return from_json_export_postings(value)


def encode_export_postings(writer: BinaryWriter, value: ExportPostings) -> None:
    """Encode one ExportPostings."""
    destack._generated.dir.index.postings.encode_postings(writer, value.names)


def decode_export_postings(reader: BinaryReader) -> ExportPostings:
    """Decode one ExportPostings."""
    names = destack._generated.dir.index.postings.decode_postings(reader)

    return ExportPostings(
        names=names,
    )


def to_json_export_postings(value: ExportPostings) -> Json:
    """Return one JSON value for one ExportPostings."""
    return {
        "names": destack._generated.dir.index.postings.to_json_postings(value.names),
    }


def from_json_export_postings(value: Json) -> ExportPostings:
    """Return one ExportPostings from one JSON value."""
    object_ = json_object(value)

    return ExportPostings(
        names=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "names")
        ),
    )


__all__ = [
    "ExportIndex",
    "encode_export_index",
    "decode_export_index",
    "to_json_export_index",
    "from_json_export_index",
    "ExportEntry",
    "encode_export_entry",
    "decode_export_entry",
    "to_json_export_entry",
    "from_json_export_entry",
    "ExportPostings",
    "encode_export_postings",
    "decode_export_postings",
    "to_json_export_postings",
    "from_json_export_postings",
]
