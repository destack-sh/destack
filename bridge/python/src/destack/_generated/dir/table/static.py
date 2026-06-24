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
    json_int,
    json_object,
    nested_bytes,
)

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.static
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class StaticSegment:
    """Static values added by one DIR phase."""

    # the module id of the static segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first static id owned by this table segment
    first_static_id: int
    # interned static values
    statics: Sequence[destack._generated.dir.tree.static.StaticTerm]
    # checked static value keyed by symbol
    static_by_symbol_id: Mapping[
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
        destack._generated.dir.tree.static.GlobalStaticId,
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticSegment:
        """Decode one StaticSegment."""
        return decode_static_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> StaticSegment:
        """Return one StaticSegment from one JSON value."""
        return from_json_static_segment(value)


def encode_static_segment(writer: BinaryWriter, value: StaticSegment) -> None:
    """Encode one StaticSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(value.first_static_id)
    writer.write_unsigned(len(value.statics))
    for item_value_statics_0 in value.statics:
        destack._generated.dir.tree.static.encode_static_term(
            writer, item_value_statics_0
        )
    entries_value_static_by_symbol_id_0 = []
    for (
        key_value_static_by_symbol_id_0,
        item_value_static_by_symbol_id_0,
    ) in value.static_by_symbol_id.items():

        def write_key_value_static_by_symbol_id_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.symbol.encode_global_symbol_id(
                writer, key_value_static_by_symbol_id_0
            )

        key_bytes = nested_bytes(write_key_value_static_by_symbol_id_0)
        entries_value_static_by_symbol_id_0.append(
            (
                key_value_static_by_symbol_id_0,
                item_value_static_by_symbol_id_0,
                key_bytes,
            )
        )
    entries_value_static_by_symbol_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_static_by_symbol_id_0))
    for entry_value_static_by_symbol_id_0 in entries_value_static_by_symbol_id_0:
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_static_by_symbol_id_0[0]
        )
        destack._generated.dir.tree.static.encode_global_static_id(
            writer, entry_value_static_by_symbol_id_0[1]
        )


def decode_static_segment(reader: BinaryReader) -> StaticSegment:
    """Decode one StaticSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    first_static_id = reader.read_number()
    statics = [
        destack._generated.dir.tree.static.decode_static_term(reader)
        for _ in range(reader.read_number())
    ]
    static_by_symbol_id = {
        destack._generated.dir.symbol.symbol.decode_global_symbol_id(
            reader
        ): destack._generated.dir.tree.static.decode_global_static_id(reader)
        for _ in range(reader.read_number())
    }

    return StaticSegment(
        module_id=module_id,
        first_static_id=first_static_id,
        statics=statics,
        static_by_symbol_id=static_by_symbol_id,
    )


def to_json_static_segment(value: StaticSegment) -> Json:
    """Return one JSON value for one StaticSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "firstStaticId": value.first_static_id,
        "statics": [
            destack._generated.dir.tree.static.to_json_static_term(item_0)
            for item_0 in value.statics
        ],
        "staticBySymbolId": [
            [
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(key_0),
                destack._generated.dir.tree.static.to_json_global_static_id(item_0),
            ]
            for key_0, item_0 in value.static_by_symbol_id.items()
        ],
    }


def from_json_static_segment(value: Json) -> StaticSegment:
    """Return one StaticSegment from one JSON value."""
    object_ = json_object(value)

    return StaticSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        first_static_id=json_int(json_field(object_, "firstStaticId")),
        statics=[
            destack._generated.dir.tree.static.from_json_static_term(item_0)
            for item_0 in json_array(json_field(object_, "statics"))
        ],
        static_by_symbol_id={
            destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                key_0
            ): destack._generated.dir.tree.static.from_json_global_static_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "staticBySymbolId"))
        },
    )


__all__ = [
    "StaticSegment",
    "encode_static_segment",
    "decode_static_segment",
    "to_json_static_segment",
    "from_json_static_segment",
]
