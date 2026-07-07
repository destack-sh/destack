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
)

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.table.definition
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.file
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class ExtensionIndex:
    """Indexed checked extensions."""

    # extensions ordered by root symbol
    by_root: Sequence[ExtensionEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extension_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionIndex:
        """Decode one ExtensionIndex."""
        return decode_extension_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extension_index(self)

    @classmethod
    def from_json(cls, value: Json) -> ExtensionIndex:
        """Return one ExtensionIndex from one JSON value."""
        return from_json_extension_index(value)


def encode_extension_index(writer: BinaryWriter, value: ExtensionIndex) -> None:
    """Encode one ExtensionIndex."""
    writer.write_unsigned(len(value.by_root))
    for item_value_by_root_0 in value.by_root:
        encode_extension_entry(writer, item_value_by_root_0)


def decode_extension_index(reader: BinaryReader) -> ExtensionIndex:
    """Decode one ExtensionIndex."""
    by_root = [decode_extension_entry(reader) for _ in range(reader.read_number())]

    return ExtensionIndex(
        by_root=by_root,
    )


def to_json_extension_index(value: ExtensionIndex) -> Json:
    """Return one JSON value for one ExtensionIndex."""
    return {
        "byRoot": [to_json_extension_entry(item_0) for item_0 in value.by_root],
    }


def from_json_extension_index(value: Json) -> ExtensionIndex:
    """Return one ExtensionIndex from one JSON value."""
    object_ = json_object(value)

    return ExtensionIndex(
        by_root=[
            from_json_extension_entry(item_0)
            for item_0 in json_array(json_field(object_, "byRoot"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ExtensionEntry:
    """One indexed extension."""

    # the extension declaration symbol
    declaration: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source extension declaration node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source file
    file: destack._generated.source.file.model.file.FileId
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the canonical lookup root when this is a rooted extension
    root: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the checked receiver type
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the extension form
    form: destack._generated.dir.table.definition.ExtensionForm

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extension_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionEntry:
        """Decode one ExtensionEntry."""
        return decode_extension_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extension_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> ExtensionEntry:
        """Return one ExtensionEntry from one JSON value."""
        return from_json_extension_entry(value)


def encode_extension_entry(writer: BinaryWriter, value: ExtensionEntry) -> None:
    """Encode one ExtensionEntry."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.declaration
    )
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.source.file.model.file.encode_file_id(writer, value.file)
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    if value.root is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.root)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    destack._generated.dir.table.definition.encode_extension_form(writer, value.form)


def decode_extension_entry(reader: BinaryReader) -> ExtensionEntry:
    """Decode one ExtensionEntry."""
    declaration = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    file = destack._generated.source.file.model.file.decode_file_id(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)
    root = reader.read_option(
        lambda: destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    )
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)
    form = destack._generated.dir.table.definition.decode_extension_form(reader)

    return ExtensionEntry(
        declaration=declaration,
        source=source,
        file=file,
        span=span,
        root=root,
        ty=ty,
        form=form,
    )


def to_json_extension_entry(value: ExtensionEntry) -> Json:
    """Return one JSON value for one ExtensionEntry."""
    return {
        "declaration": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.declaration
        ),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "file": destack._generated.source.file.model.file.to_json_file_id(value.file),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        **(
            {}
            if value.root is None
            else {
                "root": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                    value.root
                )
            }
        ),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        "form": destack._generated.dir.table.definition.to_json_extension_form(
            value.form
        ),
    }


def from_json_extension_entry(value: Json) -> ExtensionEntry:
    """Return one ExtensionEntry from one JSON value."""
    object_ = json_object(value)

    return ExtensionEntry(
        declaration=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "declaration")
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
        root=json_optional(
            object_,
            "root",
            lambda value: (
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(value)
            ),
        ),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
        form=destack._generated.dir.table.definition.from_json_extension_form(
            json_field(object_, "form")
        ),
    )


@dataclass(frozen=True, slots=True)
class ExtensionPostings:
    """Extension postings by root symbol."""

    # rooted extension postings
    roots: destack._generated.dir.index.postings.Postings
    # modules that contain blanket extensions
    blankets: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extension_postings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionPostings:
        """Decode one ExtensionPostings."""
        return decode_extension_postings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extension_postings(self)

    @classmethod
    def from_json(cls, value: Json) -> ExtensionPostings:
        """Return one ExtensionPostings from one JSON value."""
        return from_json_extension_postings(value)


def encode_extension_postings(writer: BinaryWriter, value: ExtensionPostings) -> None:
    """Encode one ExtensionPostings."""
    destack._generated.dir.index.postings.encode_postings(writer, value.roots)
    writer.write_unsigned(len(value.blankets))
    for item_value_blankets_0 in value.blankets:
        writer.write_unsigned(item_value_blankets_0)


def decode_extension_postings(reader: BinaryReader) -> ExtensionPostings:
    """Decode one ExtensionPostings."""
    roots = destack._generated.dir.index.postings.decode_postings(reader)
    blankets = [reader.read_number() for _ in range(reader.read_number())]

    return ExtensionPostings(
        roots=roots,
        blankets=blankets,
    )


def to_json_extension_postings(value: ExtensionPostings) -> Json:
    """Return one JSON value for one ExtensionPostings."""
    return {
        "roots": destack._generated.dir.index.postings.to_json_postings(value.roots),
        "blankets": [item_0 for item_0 in value.blankets],
    }


def from_json_extension_postings(value: Json) -> ExtensionPostings:
    """Return one ExtensionPostings from one JSON value."""
    object_ = json_object(value)

    return ExtensionPostings(
        roots=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "roots")
        ),
        blankets=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "blankets"))
        ],
    )


__all__ = [
    "ExtensionIndex",
    "encode_extension_index",
    "decode_extension_index",
    "to_json_extension_index",
    "from_json_extension_index",
    "ExtensionEntry",
    "encode_extension_entry",
    "decode_extension_entry",
    "to_json_extension_entry",
    "from_json_extension_entry",
    "ExtensionPostings",
    "encode_extension_postings",
    "decode_extension_postings",
    "to_json_extension_postings",
    "from_json_extension_postings",
]
