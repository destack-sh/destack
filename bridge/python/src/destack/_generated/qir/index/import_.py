# generated bridge target, do not edit

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

import destack._generated.dir.symbol.symbol
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class ImportIndex:
    """Importable export index."""

    # the import entries in stable display order
    entries: Sequence[ImportEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportIndex:
        """Decode one ImportIndex."""
        return decode_import_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_index(self)

    @classmethod
    def from_json(cls, value: Json) -> ImportIndex:
        """Return one ImportIndex from one JSON value."""
        return from_json_import_index(value)


def encode_import_index(writer: BinaryWriter, value: ImportIndex) -> None:
    """Encode one ImportIndex."""
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_import_entry(writer, item_value_entries_0)


def decode_import_index(reader: BinaryReader) -> ImportIndex:
    """Decode one ImportIndex."""
    entries = [decode_import_entry(reader) for _ in range(reader.read_number())]

    return ImportIndex(
        entries=entries,
    )


def to_json_import_index(value: ImportIndex) -> Json:
    """Return one JSON value for one ImportIndex."""
    return {
        "entries": [to_json_import_entry(item_0) for item_0 in value.entries],
    }


def from_json_import_index(value: Json) -> ImportIndex:
    """Return one ImportIndex from one JSON value."""
    object_ = json_object(value)

    return ImportIndex(
        entries=[
            from_json_import_entry(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ImportEntry:
    """Importable exported symbol entry."""

    # the exported symbol name
    name: str
    # the exported symbol kind
    kind: destack._generated.dir.symbol.symbol.SymbolKind
    # the exported symbol space
    space: destack._generated.dir.symbol.symbol.SymbolSpace
    # the module that exports this symbol
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local exported symbol id
    local_id: destack._generated.dir.symbol.symbol.LocalSymbolId
    # the module path to use in imports
    module_path: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportEntry:
        """Decode one ImportEntry."""
        return decode_import_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> ImportEntry:
        """Return one ImportEntry from one JSON value."""
        return from_json_import_entry(value)


def encode_import_entry(writer: BinaryWriter, value: ImportEntry) -> None:
    """Encode one ImportEntry."""
    writer.write_string(value.name)
    destack._generated.dir.symbol.symbol.encode_symbol_kind(writer, value.kind)
    destack._generated.dir.symbol.symbol.encode_symbol_space(writer, value.space)
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    destack._generated.dir.symbol.symbol.encode_local_symbol_id(writer, value.local_id)
    if value.module_path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.module_path)


def decode_import_entry(reader: BinaryReader) -> ImportEntry:
    """Decode one ImportEntry."""
    name = reader.read_string()
    kind = destack._generated.dir.symbol.symbol.decode_symbol_kind(reader)
    space = destack._generated.dir.symbol.symbol.decode_symbol_space(reader)
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    local_id = destack._generated.dir.symbol.symbol.decode_local_symbol_id(reader)
    module_path = reader.read_option(lambda: reader.read_string())

    return ImportEntry(
        name=name,
        kind=kind,
        space=space,
        module_id=module_id,
        local_id=local_id,
        module_path=module_path,
    )


def to_json_import_entry(value: ImportEntry) -> Json:
    """Return one JSON value for one ImportEntry."""
    return {
        "name": value.name,
        "kind": destack._generated.dir.symbol.symbol.to_json_symbol_kind(value.kind),
        "space": destack._generated.dir.symbol.symbol.to_json_symbol_space(value.space),
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "localId": destack._generated.dir.symbol.symbol.to_json_local_symbol_id(
            value.local_id
        ),
        **({} if value.module_path is None else {"modulePath": value.module_path}),
    }


def from_json_import_entry(value: Json) -> ImportEntry:
    """Return one ImportEntry from one JSON value."""
    object_ = json_object(value)

    return ImportEntry(
        name=json_string(json_field(object_, "name")),
        kind=destack._generated.dir.symbol.symbol.from_json_symbol_kind(
            json_field(object_, "kind")
        ),
        space=destack._generated.dir.symbol.symbol.from_json_symbol_space(
            json_field(object_, "space")
        ),
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        local_id=destack._generated.dir.symbol.symbol.from_json_local_symbol_id(
            json_field(object_, "localId")
        ),
        module_path=json_optional(
            object_, "modulePath", lambda value: json_string(value)
        ),
    )


__all__ = [
    "ImportIndex",
    "encode_import_index",
    "decode_import_index",
    "to_json_import_index",
    "from_json_import_index",
    "ImportEntry",
    "encode_import_entry",
    "decode_import_entry",
    "to_json_import_entry",
    "from_json_import_entry",
]
