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
    json_optional,
    json_string,
)

import destack._generated.dir.symbol.symbol
import destack._generated.qir.index.name
import destack._generated.source.file.model.file
import destack._generated.source.file.model.module
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class SymbolIndex:
    """Searchable symbol declaration index."""

    # the symbol entries in stable display order
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
    """Searchable workspace symbol entry."""

    # the display name
    name: destack._generated.qir.index.name.Name
    # the symbol kind
    kind: SymbolKind
    # the owning module
    module_id: destack._generated.source.file.model.module.ModuleId
    # the source file
    file_id: destack._generated.source.file.model.file.FileId
    # the source range
    range: destack._generated.source.file.model.span.Span
    # the indexed symbol when known
    symbol_id: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the containing symbol display name
    container_name: str | None

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
    destack._generated.qir.index.name.encode_name(writer, value.name)
    encode_symbol_kind(writer, value.kind)
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    destack._generated.source.file.model.file.encode_file_id(writer, value.file_id)
    destack._generated.source.file.model.span.encode_span(writer, value.range)
    if value.symbol_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol_id
        )
    if value.container_name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.container_name)


def decode_symbol_entry(reader: BinaryReader) -> SymbolEntry:
    """Decode one SymbolEntry."""
    name = destack._generated.qir.index.name.decode_name(reader)
    kind = decode_symbol_kind(reader)
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    file_id = destack._generated.source.file.model.file.decode_file_id(reader)
    range_ = destack._generated.source.file.model.span.decode_span(reader)
    symbol_id = reader.read_option(
        lambda: destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    )
    container_name = reader.read_option(lambda: reader.read_string())

    return SymbolEntry(
        name=name,
        kind=kind,
        module_id=module_id,
        file_id=file_id,
        range=range_,
        symbol_id=symbol_id,
        container_name=container_name,
    )


def to_json_symbol_entry(value: SymbolEntry) -> Json:
    """Return one JSON value for one SymbolEntry."""
    return {
        "name": destack._generated.qir.index.name.to_json_name(value.name),
        "kind": to_json_symbol_kind(value.kind),
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "fileId": destack._generated.source.file.model.file.to_json_file_id(
            value.file_id
        ),
        "range": destack._generated.source.file.model.span.to_json_span(value.range),
        **(
            {}
            if value.symbol_id is None
            else {
                "symbolId": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                    value.symbol_id
                )
            }
        ),
        **(
            {}
            if value.container_name is None
            else {"containerName": value.container_name}
        ),
    }


def from_json_symbol_entry(value: Json) -> SymbolEntry:
    """Return one SymbolEntry from one JSON value."""
    object_ = json_object(value)

    return SymbolEntry(
        name=destack._generated.qir.index.name.from_json_name(
            json_field(object_, "name")
        ),
        kind=from_json_symbol_kind(json_field(object_, "kind")),
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        file_id=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "fileId")
        ),
        range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "range")
        ),
        symbol_id=json_optional(
            object_,
            "symbolId",
            lambda value: (
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(value)
            ),
        ),
        container_name=json_optional(
            object_, "containerName", lambda value: json_string(value)
        ),
    )


"""Searchable workspace symbol kind."""
SymbolKind: typing.TypeAlias = (
    typing.Literal["namespace"]
    | typing.Literal["class"]
    | typing.Literal["enum"]
    | typing.Literal["interface"]
    | typing.Literal["function"]
    | typing.Literal["variable"]
    | typing.Literal["constant"]
    | typing.Literal["struct"]
    | typing.Literal["typeParameter"]
)


def encode_symbol_kind(writer: BinaryWriter, value: SymbolKind) -> None:
    """Encode one SymbolKind."""
    if value == "namespace":
        writer.write_unsigned(0)
    elif value == "class":
        writer.write_unsigned(1)
    elif value == "enum":
        writer.write_unsigned(2)
    elif value == "interface":
        writer.write_unsigned(3)
    elif value == "function":
        writer.write_unsigned(4)
    elif value == "variable":
        writer.write_unsigned(5)
    elif value == "constant":
        writer.write_unsigned(6)
    elif value == "struct":
        writer.write_unsigned(7)
    elif value == "typeParameter":
        writer.write_unsigned(8)
    else:
        raise SerdeError("unknown enum variant")


def decode_symbol_kind(reader: BinaryReader) -> SymbolKind:
    """Decode one SymbolKind."""
    variant = reader.read_number()

    if variant == 0:
        return "namespace"
    elif variant == 1:
        return "class"
    elif variant == 2:
        return "enum"
    elif variant == 3:
        return "interface"
    elif variant == 4:
        return "function"
    elif variant == 5:
        return "variable"
    elif variant == 6:
        return "constant"
    elif variant == 7:
        return "struct"
    elif variant == 8:
        return "typeParameter"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_symbol_kind(value: SymbolKind) -> Json:
    """Return one JSON value for one SymbolKind."""
    return value


def from_json_symbol_kind(value: Json) -> SymbolKind:
    """Return one SymbolKind from one JSON value."""
    variant = json_string(value)

    if variant == "namespace":
        return "namespace"
    elif variant == "class":
        return "class"
    elif variant == "enum":
        return "enum"
    elif variant == "interface":
        return "interface"
    elif variant == "function":
        return "function"
    elif variant == "variable":
        return "variable"
    elif variant == "constant":
        return "constant"
    elif variant == "struct":
        return "struct"
    elif variant == "typeParameter":
        return "typeParameter"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
    "SymbolKind",
    "encode_symbol_kind",
    "decode_symbol_kind",
    "to_json_symbol_kind",
    "from_json_symbol_kind",
]
