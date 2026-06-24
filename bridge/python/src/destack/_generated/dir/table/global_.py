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
    nested_bytes,
)

import destack._generated.dir.symbol.global_
import destack._generated.dir.symbol.key
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class GlobalTable:
    """Global names contributed by one module."""

    # the module id of the global table
    module_id: destack._generated.source.file.model.module.ModuleId
    # global entries keyed by visible global name
    entries_by_key: Mapping[
        destack._generated.dir.symbol.key.StaticKey,
        Sequence[destack._generated.dir.symbol.global_.GlobalEntry],
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalTable:
        """Decode one GlobalTable."""
        return decode_global_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_table(self)

    @classmethod
    def from_json(cls, value: Json) -> GlobalTable:
        """Return one GlobalTable from one JSON value."""
        return from_json_global_table(value)


def encode_global_table(writer: BinaryWriter, value: GlobalTable) -> None:
    """Encode one GlobalTable."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    entries_value_entries_by_key_0 = []
    for (
        key_value_entries_by_key_0,
        item_value_entries_by_key_0,
    ) in value.entries_by_key.items():

        def write_key_value_entries_by_key_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.key.encode_static_key(
                writer, key_value_entries_by_key_0
            )

        key_bytes = nested_bytes(write_key_value_entries_by_key_0)
        entries_value_entries_by_key_0.append(
            (key_value_entries_by_key_0, item_value_entries_by_key_0, key_bytes)
        )
    entries_value_entries_by_key_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_entries_by_key_0))
    for entry_value_entries_by_key_0 in entries_value_entries_by_key_0:
        destack._generated.dir.symbol.key.encode_static_key(
            writer, entry_value_entries_by_key_0[0]
        )
        writer.write_unsigned(len(entry_value_entries_by_key_0[1]))
        for item_entry_value_entries_by_key_0_1_1 in entry_value_entries_by_key_0[1]:
            destack._generated.dir.symbol.global_.encode_global_entry(
                writer, item_entry_value_entries_by_key_0_1_1
            )


def decode_global_table(reader: BinaryReader) -> GlobalTable:
    """Decode one GlobalTable."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    entries_by_key = {
        destack._generated.dir.symbol.key.decode_static_key(reader): [
            destack._generated.dir.symbol.global_.decode_global_entry(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }

    return GlobalTable(
        module_id=module_id,
        entries_by_key=entries_by_key,
    )


def to_json_global_table(value: GlobalTable) -> Json:
    """Return one JSON value for one GlobalTable."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "entriesByKey": [
            [
                destack._generated.dir.symbol.key.to_json_static_key(key_0),
                [
                    destack._generated.dir.symbol.global_.to_json_global_entry(item_1)
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.entries_by_key.items()
        ],
    }


def from_json_global_table(value: Json) -> GlobalTable:
    """Return one GlobalTable from one JSON value."""
    object_ = json_object(value)

    return GlobalTable(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        entries_by_key={
            destack._generated.dir.symbol.key.from_json_static_key(key_0): [
                destack._generated.dir.symbol.global_.from_json_global_entry(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "entriesByKey"))
        },
    )


__all__ = [
    "GlobalTable",
    "encode_global_table",
    "decode_global_table",
    "to_json_global_table",
    "from_json_global_table",
]
