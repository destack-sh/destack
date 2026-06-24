# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
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
    nested_bytes,
)

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.language
import destack._generated.dir.symbol.symbol
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class ImportTargetSymbol:
    """A symbol exported by a target module."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_target(self)


@dataclass(frozen=True, slots=True)
class ImportTargetNamespace:
    """A namespace object for a target module."""

    namespace: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["namespace"] = "namespace"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_target(self)


"""Target selected by one import binding."""
ImportTarget: typing.TypeAlias = ImportTargetSymbol | ImportTargetNamespace


def encode_import_target(writer: BinaryWriter, value: ImportTarget) -> None:
    """Encode one ImportTarget."""
    if value.kind == "symbol":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
    elif value.kind == "namespace":
        writer.write_unsigned(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.namespace
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_import_target(reader: BinaryReader) -> ImportTarget:
    """Decode one ImportTarget."""
    variant = reader.read_number()

    if variant == 0:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return ImportTargetSymbol(symbol=symbol)
    elif variant == 1:
        namespace = destack._generated.source.file.model.module.decode_module_id(reader)

        return ImportTargetNamespace(namespace=namespace)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_import_target(value: ImportTarget) -> Json:
    """Return one JSON value for one ImportTarget."""
    if value.kind == "symbol":
        return {
            "kind": "symbol",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
        }
    elif value.kind == "namespace":
        return {
            "kind": "namespace",
            "namespace": destack._generated.source.file.model.module.to_json_module_id(
                value.namespace
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_import_target(value: Json) -> ImportTarget:
    """Return one ImportTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "symbol":
        return ImportTargetSymbol(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            )
        )
    elif kind == "namespace":
        return ImportTargetNamespace(
            namespace=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "namespace")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ImportTable:
    """Resolved import targets for one module."""

    # the module id of the import table
    module_id: destack._generated.source.file.model.module.ModuleId
    # modules reached by resolved imports and active globals
    modules: Sequence[destack._generated.source.file.model.module.ModuleId]
    # imported local symbols keyed to their resolved target
    target_by_symbol: Mapping[
        destack._generated.dir.symbol.symbol.LocalSymbolId, ImportTarget
    ]
    # global targets made visible by the active profile
    global_target_by_key: Mapping[
        destack._generated.dir.symbol.key.StaticKey, Sequence[ImportTarget]
    ]
    # language item symbols required by compiler syntax
    language_symbol_by_item: Mapping[
        destack._generated.dir.symbol.language.LanguageItem,
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportTable:
        """Decode one ImportTable."""
        return decode_import_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_table(self)

    @classmethod
    def from_json(cls, value: Json) -> ImportTable:
        """Return one ImportTable from one JSON value."""
        return from_json_import_table(value)


def encode_import_table(writer: BinaryWriter, value: ImportTable) -> None:
    """Encode one ImportTable."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(len(value.modules))
    for item_value_modules_0 in value.modules:
        destack._generated.source.file.model.module.encode_module_id(
            writer, item_value_modules_0
        )
    entries_value_target_by_symbol_0 = []
    for (
        key_value_target_by_symbol_0,
        item_value_target_by_symbol_0,
    ) in value.target_by_symbol.items():

        def write_key_value_target_by_symbol_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.symbol.encode_local_symbol_id(
                writer, key_value_target_by_symbol_0
            )

        key_bytes = nested_bytes(write_key_value_target_by_symbol_0)
        entries_value_target_by_symbol_0.append(
            (key_value_target_by_symbol_0, item_value_target_by_symbol_0, key_bytes)
        )
    entries_value_target_by_symbol_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_target_by_symbol_0))
    for entry_value_target_by_symbol_0 in entries_value_target_by_symbol_0:
        destack._generated.dir.symbol.symbol.encode_local_symbol_id(
            writer, entry_value_target_by_symbol_0[0]
        )
        encode_import_target(writer, entry_value_target_by_symbol_0[1])
    entries_value_global_target_by_key_0 = []
    for (
        key_value_global_target_by_key_0,
        item_value_global_target_by_key_0,
    ) in value.global_target_by_key.items():

        def write_key_value_global_target_by_key_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.key.encode_static_key(
                writer, key_value_global_target_by_key_0
            )

        key_bytes = nested_bytes(write_key_value_global_target_by_key_0)
        entries_value_global_target_by_key_0.append(
            (
                key_value_global_target_by_key_0,
                item_value_global_target_by_key_0,
                key_bytes,
            )
        )
    entries_value_global_target_by_key_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_global_target_by_key_0))
    for entry_value_global_target_by_key_0 in entries_value_global_target_by_key_0:
        destack._generated.dir.symbol.key.encode_static_key(
            writer, entry_value_global_target_by_key_0[0]
        )
        writer.write_unsigned(len(entry_value_global_target_by_key_0[1]))
        for (
            item_entry_value_global_target_by_key_0_1_1
        ) in entry_value_global_target_by_key_0[1]:
            encode_import_target(writer, item_entry_value_global_target_by_key_0_1_1)
    entries_value_language_symbol_by_item_0 = []
    for (
        key_value_language_symbol_by_item_0,
        item_value_language_symbol_by_item_0,
    ) in value.language_symbol_by_item.items():

        def write_key_value_language_symbol_by_item_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.language.encode_language_item(
                writer, key_value_language_symbol_by_item_0
            )

        key_bytes = nested_bytes(write_key_value_language_symbol_by_item_0)
        entries_value_language_symbol_by_item_0.append(
            (
                key_value_language_symbol_by_item_0,
                item_value_language_symbol_by_item_0,
                key_bytes,
            )
        )
    entries_value_language_symbol_by_item_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_language_symbol_by_item_0))
    for (
        entry_value_language_symbol_by_item_0
    ) in entries_value_language_symbol_by_item_0:
        destack._generated.dir.symbol.language.encode_language_item(
            writer, entry_value_language_symbol_by_item_0[0]
        )
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_language_symbol_by_item_0[1]
        )


def decode_import_table(reader: BinaryReader) -> ImportTable:
    """Decode one ImportTable."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    modules = [
        destack._generated.source.file.model.module.decode_module_id(reader)
        for _ in range(reader.read_number())
    ]
    target_by_symbol = {
        destack._generated.dir.symbol.symbol.decode_local_symbol_id(
            reader
        ): decode_import_target(reader)
        for _ in range(reader.read_number())
    }
    global_target_by_key = {
        destack._generated.dir.symbol.key.decode_static_key(reader): [
            decode_import_target(reader) for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    language_symbol_by_item = {
        destack._generated.dir.symbol.language.decode_language_item(
            reader
        ): destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        for _ in range(reader.read_number())
    }

    return ImportTable(
        module_id=module_id,
        modules=modules,
        target_by_symbol=target_by_symbol,
        global_target_by_key=global_target_by_key,
        language_symbol_by_item=language_symbol_by_item,
    )


def to_json_import_table(value: ImportTable) -> Json:
    """Return one JSON value for one ImportTable."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "modules": [
            destack._generated.source.file.model.module.to_json_module_id(item_0)
            for item_0 in value.modules
        ],
        "targetBySymbol": [
            [
                destack._generated.dir.symbol.symbol.to_json_local_symbol_id(key_0),
                to_json_import_target(item_0),
            ]
            for key_0, item_0 in value.target_by_symbol.items()
        ],
        "globalTargetByKey": [
            [
                destack._generated.dir.symbol.key.to_json_static_key(key_0),
                [to_json_import_target(item_1) for item_1 in item_0],
            ]
            for key_0, item_0 in value.global_target_by_key.items()
        ],
        "languageSymbolByItem": [
            [
                destack._generated.dir.symbol.language.to_json_language_item(key_0),
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(item_0),
            ]
            for key_0, item_0 in value.language_symbol_by_item.items()
        ],
    }


def from_json_import_table(value: Json) -> ImportTable:
    """Return one ImportTable from one JSON value."""
    object_ = json_object(value)

    return ImportTable(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        modules=[
            destack._generated.source.file.model.module.from_json_module_id(item_0)
            for item_0 in json_array(json_field(object_, "modules"))
        ],
        target_by_symbol={
            destack._generated.dir.symbol.symbol.from_json_local_symbol_id(
                key_0
            ): from_json_import_target(item_0)
            for key_0, item_0 in json_array(json_field(object_, "targetBySymbol"))
        },
        global_target_by_key={
            destack._generated.dir.symbol.key.from_json_static_key(key_0): [
                from_json_import_target(item_1) for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "globalTargetByKey"))
        },
        language_symbol_by_item={
            destack._generated.dir.symbol.language.from_json_language_item(
                key_0
            ): destack._generated.dir.symbol.symbol.from_json_global_symbol_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "languageSymbolByItem"))
        },
    )


__all__ = [
    "ImportTarget",
    "encode_import_target",
    "decode_import_target",
    "to_json_import_target",
    "from_json_import_target",
    "ImportTargetSymbol",
    "ImportTargetNamespace",
    "ImportTable",
    "encode_import_table",
    "decode_import_table",
    "to_json_import_table",
    "from_json_import_table",
]
