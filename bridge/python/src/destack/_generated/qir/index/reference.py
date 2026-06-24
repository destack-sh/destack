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
)

import destack._generated.dir.symbol.symbol
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class ReferenceIndex:
    """Reference target membership index."""

    # the reference entries ordered by target symbol
    by_target: Sequence[ReferenceEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceIndex:
        """Decode one ReferenceIndex."""
        return decode_reference_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference_index(self)

    @classmethod
    def from_json(cls, value: Json) -> ReferenceIndex:
        """Return one ReferenceIndex from one JSON value."""
        return from_json_reference_index(value)


def encode_reference_index(writer: BinaryWriter, value: ReferenceIndex) -> None:
    """Encode one ReferenceIndex."""
    writer.write_unsigned(len(value.by_target))
    for item_value_by_target_0 in value.by_target:
        encode_reference_entry(writer, item_value_by_target_0)


def decode_reference_index(reader: BinaryReader) -> ReferenceIndex:
    """Decode one ReferenceIndex."""
    by_target = [decode_reference_entry(reader) for _ in range(reader.read_number())]

    return ReferenceIndex(
        by_target=by_target,
    )


def to_json_reference_index(value: ReferenceIndex) -> Json:
    """Return one JSON value for one ReferenceIndex."""
    return {
        "byTarget": [to_json_reference_entry(item_0) for item_0 in value.by_target],
    }


def from_json_reference_index(value: Json) -> ReferenceIndex:
    """Return one ReferenceIndex from one JSON value."""
    object_ = json_object(value)

    return ReferenceIndex(
        by_target=[
            from_json_reference_entry(item_0)
            for item_0 in json_array(json_field(object_, "byTarget"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ReferenceEntry:
    """One module's membership in one reference target set."""

    # the referenced symbol
    target_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the module that may reference the symbol
    module_id: destack._generated.source.file.model.module.ModuleId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceEntry:
        """Decode one ReferenceEntry."""
        return decode_reference_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> ReferenceEntry:
        """Return one ReferenceEntry from one JSON value."""
        return from_json_reference_entry(value)


def encode_reference_entry(writer: BinaryWriter, value: ReferenceEntry) -> None:
    """Encode one ReferenceEntry."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.target_symbol
    )
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )


def decode_reference_entry(reader: BinaryReader) -> ReferenceEntry:
    """Decode one ReferenceEntry."""
    target_symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)

    return ReferenceEntry(
        target_symbol=target_symbol,
        module_id=module_id,
    )


def to_json_reference_entry(value: ReferenceEntry) -> Json:
    """Return one JSON value for one ReferenceEntry."""
    return {
        "targetSymbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.target_symbol
        ),
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
    }


def from_json_reference_entry(value: Json) -> ReferenceEntry:
    """Return one ReferenceEntry from one JSON value."""
    object_ = json_object(value)

    return ReferenceEntry(
        target_symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "targetSymbol")
        ),
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
    )


__all__ = [
    "ReferenceIndex",
    "encode_reference_index",
    "decode_reference_index",
    "to_json_reference_index",
    "from_json_reference_index",
    "ReferenceEntry",
    "encode_reference_entry",
    "decode_reference_entry",
    "to_json_reference_entry",
    "from_json_reference_entry",
]
