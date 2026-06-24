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

import destack._generated.dir.symbol.export
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class ExportTable:
    """Resolved module exports for one module."""

    # the module id of the export table
    module_id: destack._generated.source.file.model.module.ModuleId
    # local and indirect exports keyed by exported name
    export_by_key: Mapping[
        destack._generated.dir.symbol.export.ExportKey,
        destack._generated.dir.symbol.export.ExportEntry,
    ]
    # star exports declared by the module
    star_exports: Sequence[destack._generated.dir.symbol.export.StarExportEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportTable:
        """Decode one ExportTable."""
        return decode_export_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_table(self)

    @classmethod
    def from_json(cls, value: Json) -> ExportTable:
        """Return one ExportTable from one JSON value."""
        return from_json_export_table(value)


def encode_export_table(writer: BinaryWriter, value: ExportTable) -> None:
    """Encode one ExportTable."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    entries_value_export_by_key_0 = []
    for (
        key_value_export_by_key_0,
        item_value_export_by_key_0,
    ) in value.export_by_key.items():

        def write_key_value_export_by_key_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.export.encode_export_key(
                writer, key_value_export_by_key_0
            )

        key_bytes = nested_bytes(write_key_value_export_by_key_0)
        entries_value_export_by_key_0.append(
            (key_value_export_by_key_0, item_value_export_by_key_0, key_bytes)
        )
    entries_value_export_by_key_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_export_by_key_0))
    for entry_value_export_by_key_0 in entries_value_export_by_key_0:
        destack._generated.dir.symbol.export.encode_export_key(
            writer, entry_value_export_by_key_0[0]
        )
        destack._generated.dir.symbol.export.encode_export_entry(
            writer, entry_value_export_by_key_0[1]
        )
    writer.write_unsigned(len(value.star_exports))
    for item_value_star_exports_0 in value.star_exports:
        destack._generated.dir.symbol.export.encode_star_export_entry(
            writer, item_value_star_exports_0
        )


def decode_export_table(reader: BinaryReader) -> ExportTable:
    """Decode one ExportTable."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    export_by_key = {
        destack._generated.dir.symbol.export.decode_export_key(
            reader
        ): destack._generated.dir.symbol.export.decode_export_entry(reader)
        for _ in range(reader.read_number())
    }
    star_exports = [
        destack._generated.dir.symbol.export.decode_star_export_entry(reader)
        for _ in range(reader.read_number())
    ]

    return ExportTable(
        module_id=module_id,
        export_by_key=export_by_key,
        star_exports=star_exports,
    )


def to_json_export_table(value: ExportTable) -> Json:
    """Return one JSON value for one ExportTable."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "exportByKey": [
            [
                destack._generated.dir.symbol.export.to_json_export_key(key_0),
                destack._generated.dir.symbol.export.to_json_export_entry(item_0),
            ]
            for key_0, item_0 in value.export_by_key.items()
        ],
        "starExports": [
            destack._generated.dir.symbol.export.to_json_star_export_entry(item_0)
            for item_0 in value.star_exports
        ],
    }


def from_json_export_table(value: Json) -> ExportTable:
    """Return one ExportTable from one JSON value."""
    object_ = json_object(value)

    return ExportTable(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        export_by_key={
            destack._generated.dir.symbol.export.from_json_export_key(
                key_0
            ): destack._generated.dir.symbol.export.from_json_export_entry(item_0)
            for key_0, item_0 in json_array(json_field(object_, "exportByKey"))
        },
        star_exports=[
            destack._generated.dir.symbol.export.from_json_star_export_entry(item_0)
            for item_0 in json_array(json_field(object_, "starExports"))
        ],
    )


__all__ = [
    "ExportTable",
    "encode_export_table",
    "decode_export_table",
    "to_json_export_table",
    "from_json_export_table",
]
