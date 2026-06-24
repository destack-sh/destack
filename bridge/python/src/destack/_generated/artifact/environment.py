# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_object,
    json_string,
    nested_bytes,
)

import destack._generated.core.string
import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.language
import destack._generated.dir.symbol.symbol
import destack._generated.dir.table.import_
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class LanguageEnvironment:
    """Compiler-known language environment for one profile."""

    # language item symbols by item id
    symbol_by_item: Mapping[
        destack._generated.dir.symbol.language.LanguageItem,
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
    ]
    # language items by symbol id
    items_by_symbol: Mapping[
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
        destack._generated.dir.symbol.language.LanguageItem,
    ]
    # builtin symbols by export name
    symbols: Mapping[
        destack._generated.core.string.StringId,
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_language_environment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LanguageEnvironment:
        """Decode one LanguageEnvironment."""
        return decode_language_environment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_language_environment(self)

    @classmethod
    def from_json(cls, value: Json) -> LanguageEnvironment:
        """Return one LanguageEnvironment from one JSON value."""
        return from_json_language_environment(value)


def encode_language_environment(
    writer: BinaryWriter, value: LanguageEnvironment
) -> None:
    """Encode one LanguageEnvironment."""
    entries_value_symbol_by_item_0 = []
    for (
        key_value_symbol_by_item_0,
        item_value_symbol_by_item_0,
    ) in value.symbol_by_item.items():

        def write_key_value_symbol_by_item_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.language.encode_language_item(
                writer, key_value_symbol_by_item_0
            )

        key_bytes = nested_bytes(write_key_value_symbol_by_item_0)
        entries_value_symbol_by_item_0.append(
            (key_value_symbol_by_item_0, item_value_symbol_by_item_0, key_bytes)
        )
    entries_value_symbol_by_item_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_symbol_by_item_0))
    for entry_value_symbol_by_item_0 in entries_value_symbol_by_item_0:
        destack._generated.dir.symbol.language.encode_language_item(
            writer, entry_value_symbol_by_item_0[0]
        )
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_symbol_by_item_0[1]
        )
    entries_value_items_by_symbol_0 = []
    for (
        key_value_items_by_symbol_0,
        item_value_items_by_symbol_0,
    ) in value.items_by_symbol.items():

        def write_key_value_items_by_symbol_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.symbol.encode_global_symbol_id(
                writer, key_value_items_by_symbol_0
            )

        key_bytes = nested_bytes(write_key_value_items_by_symbol_0)
        entries_value_items_by_symbol_0.append(
            (key_value_items_by_symbol_0, item_value_items_by_symbol_0, key_bytes)
        )
    entries_value_items_by_symbol_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_items_by_symbol_0))
    for entry_value_items_by_symbol_0 in entries_value_items_by_symbol_0:
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_items_by_symbol_0[0]
        )
        destack._generated.dir.symbol.language.encode_language_item(
            writer, entry_value_items_by_symbol_0[1]
        )
    entries_value_symbols_0 = []
    for key_value_symbols_0, item_value_symbols_0 in value.symbols.items():

        def write_key_value_symbols_0(writer: BinaryWriter) -> None:
            destack._generated.core.string.encode_string_id(writer, key_value_symbols_0)

        key_bytes = nested_bytes(write_key_value_symbols_0)
        entries_value_symbols_0.append(
            (key_value_symbols_0, item_value_symbols_0, key_bytes)
        )
    entries_value_symbols_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_symbols_0))
    for entry_value_symbols_0 in entries_value_symbols_0:
        destack._generated.core.string.encode_string_id(
            writer, entry_value_symbols_0[0]
        )
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_symbols_0[1]
        )


def decode_language_environment(reader: BinaryReader) -> LanguageEnvironment:
    """Decode one LanguageEnvironment."""
    symbol_by_item = {
        destack._generated.dir.symbol.language.decode_language_item(
            reader
        ): destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        for _ in range(reader.read_number())
    }
    items_by_symbol = {
        destack._generated.dir.symbol.symbol.decode_global_symbol_id(
            reader
        ): destack._generated.dir.symbol.language.decode_language_item(reader)
        for _ in range(reader.read_number())
    }
    symbols = {
        destack._generated.core.string.decode_string_id(
            reader
        ): destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        for _ in range(reader.read_number())
    }

    return LanguageEnvironment(
        symbol_by_item=symbol_by_item,
        items_by_symbol=items_by_symbol,
        symbols=symbols,
    )


def to_json_language_environment(value: LanguageEnvironment) -> Json:
    """Return one JSON value for one LanguageEnvironment."""
    return {
        "symbolByItem": [
            [
                destack._generated.dir.symbol.language.to_json_language_item(key_0),
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(item_0),
            ]
            for key_0, item_0 in value.symbol_by_item.items()
        ],
        "itemsBySymbol": [
            [
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(key_0),
                destack._generated.dir.symbol.language.to_json_language_item(item_0),
            ]
            for key_0, item_0 in value.items_by_symbol.items()
        ],
        "symbols": [
            [
                destack._generated.core.string.to_json_string_id(key_0),
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(item_0),
            ]
            for key_0, item_0 in value.symbols.items()
        ],
    }


def from_json_language_environment(value: Json) -> LanguageEnvironment:
    """Return one LanguageEnvironment from one JSON value."""
    object_ = json_object(value)

    return LanguageEnvironment(
        symbol_by_item={
            destack._generated.dir.symbol.language.from_json_language_item(
                key_0
            ): destack._generated.dir.symbol.symbol.from_json_global_symbol_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "symbolByItem"))
        },
        items_by_symbol={
            destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                key_0
            ): destack._generated.dir.symbol.language.from_json_language_item(item_0)
            for key_0, item_0 in json_array(json_field(object_, "itemsBySymbol"))
        },
        symbols={
            destack._generated.core.string.from_json_string_id(
                key_0
            ): destack._generated.dir.symbol.symbol.from_json_global_symbol_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "symbols"))
        },
    )


@dataclass(frozen=True, slots=True)
class GlobalEnvironment:
    """Explicit global environment selected for one profile."""

    # compiler-known language environment
    language: LanguageEnvironment
    # explicit global modules in load order
    globals: Sequence[destack._generated.source.file.model.module.ModuleId]
    # resolved global bindings by key across the global modules
    global_targets_by_key: Mapping[
        destack._generated.dir.symbol.key.StaticKey,
        Sequence[destack._generated.dir.table.import_.ImportTarget],
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_environment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalEnvironment:
        """Decode one GlobalEnvironment."""
        return decode_global_environment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_environment(self)

    @classmethod
    def from_json(cls, value: Json) -> GlobalEnvironment:
        """Return one GlobalEnvironment from one JSON value."""
        return from_json_global_environment(value)


def encode_global_environment(writer: BinaryWriter, value: GlobalEnvironment) -> None:
    """Encode one GlobalEnvironment."""
    encode_language_environment(writer, value.language)
    writer.write_unsigned(len(value.globals))
    for item_value_globals_0 in value.globals:
        destack._generated.source.file.model.module.encode_module_id(
            writer, item_value_globals_0
        )
    entries_value_global_targets_by_key_0 = []
    for (
        key_value_global_targets_by_key_0,
        item_value_global_targets_by_key_0,
    ) in value.global_targets_by_key.items():

        def write_key_value_global_targets_by_key_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.key.encode_static_key(
                writer, key_value_global_targets_by_key_0
            )

        key_bytes = nested_bytes(write_key_value_global_targets_by_key_0)
        entries_value_global_targets_by_key_0.append(
            (
                key_value_global_targets_by_key_0,
                item_value_global_targets_by_key_0,
                key_bytes,
            )
        )
    entries_value_global_targets_by_key_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_global_targets_by_key_0))
    for entry_value_global_targets_by_key_0 in entries_value_global_targets_by_key_0:
        destack._generated.dir.symbol.key.encode_static_key(
            writer, entry_value_global_targets_by_key_0[0]
        )
        writer.write_unsigned(len(entry_value_global_targets_by_key_0[1]))
        for (
            item_entry_value_global_targets_by_key_0_1_1
        ) in entry_value_global_targets_by_key_0[1]:
            destack._generated.dir.table.import_.encode_import_target(
                writer, item_entry_value_global_targets_by_key_0_1_1
            )


def decode_global_environment(reader: BinaryReader) -> GlobalEnvironment:
    """Decode one GlobalEnvironment."""
    language = decode_language_environment(reader)
    globals = [
        destack._generated.source.file.model.module.decode_module_id(reader)
        for _ in range(reader.read_number())
    ]
    global_targets_by_key = {
        destack._generated.dir.symbol.key.decode_static_key(reader): [
            destack._generated.dir.table.import_.decode_import_target(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }

    return GlobalEnvironment(
        language=language,
        globals=globals,
        global_targets_by_key=global_targets_by_key,
    )


def to_json_global_environment(value: GlobalEnvironment) -> Json:
    """Return one JSON value for one GlobalEnvironment."""
    return {
        "language": to_json_language_environment(value.language),
        "globals": [
            destack._generated.source.file.model.module.to_json_module_id(item_0)
            for item_0 in value.globals
        ],
        "globalTargetsByKey": [
            [
                destack._generated.dir.symbol.key.to_json_static_key(key_0),
                [
                    destack._generated.dir.table.import_.to_json_import_target(item_1)
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.global_targets_by_key.items()
        ],
    }


def from_json_global_environment(value: Json) -> GlobalEnvironment:
    """Return one GlobalEnvironment from one JSON value."""
    object_ = json_object(value)

    return GlobalEnvironment(
        language=from_json_language_environment(json_field(object_, "language")),
        globals=[
            destack._generated.source.file.model.module.from_json_module_id(item_0)
            for item_0 in json_array(json_field(object_, "globals"))
        ],
        global_targets_by_key={
            destack._generated.dir.symbol.key.from_json_static_key(key_0): [
                destack._generated.dir.table.import_.from_json_import_target(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "globalTargetsByKey"))
        },
    )


@dataclass(frozen=True, slots=True)
class LanguageIntrinsics:
    """Resolved compiler-known intrinsic bindings for a profile."""

    # intrinsic names keyed by symbol id
    names_by_symbol: Mapping[destack._generated.dir.symbol.symbol.GlobalSymbolId, str]
    # intrinsic symbols keyed by name
    symbols_by_name: Mapping[str, destack._generated.dir.symbol.symbol.GlobalSymbolId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_language_intrinsics(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LanguageIntrinsics:
        """Decode one LanguageIntrinsics."""
        return decode_language_intrinsics(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_language_intrinsics(self)

    @classmethod
    def from_json(cls, value: Json) -> LanguageIntrinsics:
        """Return one LanguageIntrinsics from one JSON value."""
        return from_json_language_intrinsics(value)


def encode_language_intrinsics(writer: BinaryWriter, value: LanguageIntrinsics) -> None:
    """Encode one LanguageIntrinsics."""
    entries_value_names_by_symbol_0 = []
    for (
        key_value_names_by_symbol_0,
        item_value_names_by_symbol_0,
    ) in value.names_by_symbol.items():

        def write_key_value_names_by_symbol_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.symbol.encode_global_symbol_id(
                writer, key_value_names_by_symbol_0
            )

        key_bytes = nested_bytes(write_key_value_names_by_symbol_0)
        entries_value_names_by_symbol_0.append(
            (key_value_names_by_symbol_0, item_value_names_by_symbol_0, key_bytes)
        )
    entries_value_names_by_symbol_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_names_by_symbol_0))
    for entry_value_names_by_symbol_0 in entries_value_names_by_symbol_0:
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_names_by_symbol_0[0]
        )
        writer.write_string(entry_value_names_by_symbol_0[1])
    entries_value_symbols_by_name_0 = []
    for (
        key_value_symbols_by_name_0,
        item_value_symbols_by_name_0,
    ) in value.symbols_by_name.items():

        def write_key_value_symbols_by_name_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_symbols_by_name_0)

        key_bytes = nested_bytes(write_key_value_symbols_by_name_0)
        entries_value_symbols_by_name_0.append(
            (key_value_symbols_by_name_0, item_value_symbols_by_name_0, key_bytes)
        )
    entries_value_symbols_by_name_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_symbols_by_name_0))
    for entry_value_symbols_by_name_0 in entries_value_symbols_by_name_0:
        writer.write_string(entry_value_symbols_by_name_0[0])
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_symbols_by_name_0[1]
        )


def decode_language_intrinsics(reader: BinaryReader) -> LanguageIntrinsics:
    """Decode one LanguageIntrinsics."""
    names_by_symbol = {
        destack._generated.dir.symbol.symbol.decode_global_symbol_id(
            reader
        ): reader.read_string()
        for _ in range(reader.read_number())
    }
    symbols_by_name = {
        reader.read_string(): destack._generated.dir.symbol.symbol.decode_global_symbol_id(
            reader
        )
        for _ in range(reader.read_number())
    }

    return LanguageIntrinsics(
        names_by_symbol=names_by_symbol,
        symbols_by_name=symbols_by_name,
    )


def to_json_language_intrinsics(value: LanguageIntrinsics) -> Json:
    """Return one JSON value for one LanguageIntrinsics."""
    return {
        "namesBySymbol": [
            [
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(key_0),
                item_0,
            ]
            for key_0, item_0 in value.names_by_symbol.items()
        ],
        "symbolsByName": {
            key_0: destack._generated.dir.symbol.symbol.to_json_global_symbol_id(item_0)
            for key_0, item_0 in value.symbols_by_name.items()
        },
    }


def from_json_language_intrinsics(value: Json) -> LanguageIntrinsics:
    """Return one LanguageIntrinsics from one JSON value."""
    object_ = json_object(value)

    return LanguageIntrinsics(
        names_by_symbol={
            destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                key_0
            ): json_string(item_0)
            for key_0, item_0 in json_array(json_field(object_, "namesBySymbol"))
        },
        symbols_by_name={
            key_0: destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                item_0
            )
            for key_0, item_0 in json_object(
                json_field(object_, "symbolsByName")
            ).items()
        },
    )


__all__ = [
    "LanguageEnvironment",
    "encode_language_environment",
    "decode_language_environment",
    "to_json_language_environment",
    "from_json_language_environment",
    "GlobalEnvironment",
    "encode_global_environment",
    "decode_global_environment",
    "to_json_global_environment",
    "from_json_global_environment",
    "LanguageIntrinsics",
    "encode_language_intrinsics",
    "decode_language_intrinsics",
    "to_json_language_intrinsics",
    "from_json_language_intrinsics",
]
