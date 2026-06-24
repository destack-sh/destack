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
    json_string,
)

import destack._generated.dir.symbol.symbol


@dataclass(frozen=True, slots=True)
class DefinitionIndex:
    """Query index entries read from checked definitions."""

    # nominal relation entries ordered by target symbol
    relations_by_target: Sequence[NominalEntry]
    # extension entries ordered by target symbol
    extensions_by_target: Sequence[ExtensionEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DefinitionIndex:
        """Decode one DefinitionIndex."""
        return decode_definition_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition_index(self)

    @classmethod
    def from_json(cls, value: Json) -> DefinitionIndex:
        """Return one DefinitionIndex from one JSON value."""
        return from_json_definition_index(value)


def encode_definition_index(writer: BinaryWriter, value: DefinitionIndex) -> None:
    """Encode one DefinitionIndex."""
    writer.write_unsigned(len(value.relations_by_target))
    for item_value_relations_by_target_0 in value.relations_by_target:
        encode_nominal_entry(writer, item_value_relations_by_target_0)
    writer.write_unsigned(len(value.extensions_by_target))
    for item_value_extensions_by_target_0 in value.extensions_by_target:
        encode_extension_entry(writer, item_value_extensions_by_target_0)


def decode_definition_index(reader: BinaryReader) -> DefinitionIndex:
    """Decode one DefinitionIndex."""
    relations_by_target = [
        decode_nominal_entry(reader) for _ in range(reader.read_number())
    ]
    extensions_by_target = [
        decode_extension_entry(reader) for _ in range(reader.read_number())
    ]

    return DefinitionIndex(
        relations_by_target=relations_by_target,
        extensions_by_target=extensions_by_target,
    )


def to_json_definition_index(value: DefinitionIndex) -> Json:
    """Return one JSON value for one DefinitionIndex."""
    return {
        "relationsByTarget": [
            to_json_nominal_entry(item_0) for item_0 in value.relations_by_target
        ],
        "extensionsByTarget": [
            to_json_extension_entry(item_0) for item_0 in value.extensions_by_target
        ],
    }


def from_json_definition_index(value: Json) -> DefinitionIndex:
    """Return one DefinitionIndex from one JSON value."""
    object_ = json_object(value)

    return DefinitionIndex(
        relations_by_target=[
            from_json_nominal_entry(item_0)
            for item_0 in json_array(json_field(object_, "relationsByTarget"))
        ],
        extensions_by_target=[
            from_json_extension_entry(item_0)
            for item_0 in json_array(json_field(object_, "extensionsByTarget"))
        ],
    )


@dataclass(frozen=True, slots=True)
class NominalEntry:
    """Nominal relation entry."""

    # the source nominal symbol
    source_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the target nominal symbol
    target_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the relation kind
    relation: NominalRelation

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_nominal_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NominalEntry:
        """Decode one NominalEntry."""
        return decode_nominal_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_nominal_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> NominalEntry:
        """Return one NominalEntry from one JSON value."""
        return from_json_nominal_entry(value)


def encode_nominal_entry(writer: BinaryWriter, value: NominalEntry) -> None:
    """Encode one NominalEntry."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.source_symbol
    )
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.target_symbol
    )
    encode_nominal_relation(writer, value.relation)


def decode_nominal_entry(reader: BinaryReader) -> NominalEntry:
    """Decode one NominalEntry."""
    source_symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    target_symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    relation = decode_nominal_relation(reader)

    return NominalEntry(
        source_symbol=source_symbol,
        target_symbol=target_symbol,
        relation=relation,
    )


def to_json_nominal_entry(value: NominalEntry) -> Json:
    """Return one JSON value for one NominalEntry."""
    return {
        "sourceSymbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.source_symbol
        ),
        "targetSymbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.target_symbol
        ),
        "relation": to_json_nominal_relation(value.relation),
    }


def from_json_nominal_entry(value: Json) -> NominalEntry:
    """Return one NominalEntry from one JSON value."""
    object_ = json_object(value)

    return NominalEntry(
        source_symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "sourceSymbol")
        ),
        target_symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "targetSymbol")
        ),
        relation=from_json_nominal_relation(json_field(object_, "relation")),
    )


"""Nominal relation kind."""
NominalRelation: typing.TypeAlias = (
    typing.Literal["extends"] | typing.Literal["implements"]
)


def encode_nominal_relation(writer: BinaryWriter, value: NominalRelation) -> None:
    """Encode one NominalRelation."""
    if value == "extends":
        writer.write_unsigned(0)
    elif value == "implements":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_nominal_relation(reader: BinaryReader) -> NominalRelation:
    """Decode one NominalRelation."""
    variant = reader.read_number()

    if variant == 0:
        return "extends"
    elif variant == 1:
        return "implements"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_nominal_relation(value: NominalRelation) -> Json:
    """Return one JSON value for one NominalRelation."""
    return value


def from_json_nominal_relation(value: Json) -> NominalRelation:
    """Return one NominalRelation from one JSON value."""
    variant = json_string(value)

    if variant == "extends":
        return "extends"
    elif variant == "implements":
        return "implements"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ExtensionEntry:
    """Extension declaration entry."""

    # the extension declaration symbol
    extension_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the canonical target symbol
    target_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId

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
        writer, value.extension_symbol
    )
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.target_symbol
    )


def decode_extension_entry(reader: BinaryReader) -> ExtensionEntry:
    """Decode one ExtensionEntry."""
    extension_symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(
        reader
    )
    target_symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

    return ExtensionEntry(
        extension_symbol=extension_symbol,
        target_symbol=target_symbol,
    )


def to_json_extension_entry(value: ExtensionEntry) -> Json:
    """Return one JSON value for one ExtensionEntry."""
    return {
        "extensionSymbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.extension_symbol
        ),
        "targetSymbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.target_symbol
        ),
    }


def from_json_extension_entry(value: Json) -> ExtensionEntry:
    """Return one ExtensionEntry from one JSON value."""
    object_ = json_object(value)

    return ExtensionEntry(
        extension_symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "extensionSymbol")
        ),
        target_symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "targetSymbol")
        ),
    )


__all__ = [
    "DefinitionIndex",
    "encode_definition_index",
    "decode_definition_index",
    "to_json_definition_index",
    "from_json_definition_index",
    "NominalEntry",
    "encode_nominal_entry",
    "decode_nominal_entry",
    "to_json_nominal_entry",
    "from_json_nominal_entry",
    "NominalRelation",
    "encode_nominal_relation",
    "decode_nominal_relation",
    "to_json_nominal_relation",
    "from_json_nominal_relation",
    "ExtensionEntry",
    "encode_extension_entry",
    "decode_extension_entry",
    "to_json_extension_entry",
    "from_json_extension_entry",
]
