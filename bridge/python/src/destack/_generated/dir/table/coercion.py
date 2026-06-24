# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping
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

import destack._generated.dir.tree.cast
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class CoercionSegment:
    """Coercions added by one DIR phase."""

    # the module id of the coercion segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # coercions keyed by the value node being coerced
    coercions: Mapping[destack._generated.dir.tree.node.GlobalNodeIdAny, Coercion]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_coercion_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CoercionSegment:
        """Decode one CoercionSegment."""
        return decode_coercion_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_coercion_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> CoercionSegment:
        """Return one CoercionSegment from one JSON value."""
        return from_json_coercion_segment(value)


def encode_coercion_segment(writer: BinaryWriter, value: CoercionSegment) -> None:
    """Encode one CoercionSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    entries_value_coercions_0 = []
    for key_value_coercions_0, item_value_coercions_0 in value.coercions.items():

        def write_key_value_coercions_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_coercions_0
            )

        key_bytes = nested_bytes(write_key_value_coercions_0)
        entries_value_coercions_0.append(
            (key_value_coercions_0, item_value_coercions_0, key_bytes)
        )
    entries_value_coercions_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_coercions_0))
    for entry_value_coercions_0 in entries_value_coercions_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_coercions_0[0]
        )
        encode_coercion(writer, entry_value_coercions_0[1])


def decode_coercion_segment(reader: BinaryReader) -> CoercionSegment:
    """Decode one CoercionSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    coercions = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): decode_coercion(reader)
        for _ in range(reader.read_number())
    }

    return CoercionSegment(
        module_id=module_id,
        coercions=coercions,
    )


def to_json_coercion_segment(value: CoercionSegment) -> Json:
    """Return one JSON value for one CoercionSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "coercions": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                to_json_coercion(item_0),
            ]
            for key_0, item_0 in value.coercions.items()
        ],
    }


def from_json_coercion_segment(value: Json) -> CoercionSegment:
    """Return one CoercionSegment from one JSON value."""
    object_ = json_object(value)

    return CoercionSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        coercions={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): from_json_coercion(item_0)
            for key_0, item_0 in json_array(json_field(object_, "coercions"))
        },
    )


@dataclass(frozen=True, slots=True)
class Coercion:
    """One type coercion attached to a value node."""

    # the source type before coercion
    source: destack._generated.dir.type.type.GlobalTypeId
    # the target type after coercion
    target: destack._generated.dir.type.type.GlobalTypeId
    # how the coercion entered DIR
    origin: destack._generated.dir.tree.cast.CastOrigin

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_coercion(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Coercion:
        """Decode one Coercion."""
        return decode_coercion(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_coercion(self)

    @classmethod
    def from_json(cls, value: Json) -> Coercion:
        """Return one Coercion from one JSON value."""
        return from_json_coercion(value)


def encode_coercion(writer: BinaryWriter, value: Coercion) -> None:
    """Encode one Coercion."""
    destack._generated.dir.type.type.encode_global_type_id(writer, value.source)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.target)
    destack._generated.dir.tree.cast.encode_cast_origin(writer, value.origin)


def decode_coercion(reader: BinaryReader) -> Coercion:
    """Decode one Coercion."""
    source = destack._generated.dir.type.type.decode_global_type_id(reader)
    target = destack._generated.dir.type.type.decode_global_type_id(reader)
    origin = destack._generated.dir.tree.cast.decode_cast_origin(reader)

    return Coercion(
        source=source,
        target=target,
        origin=origin,
    )


def to_json_coercion(value: Coercion) -> Json:
    """Return one JSON value for one Coercion."""
    return {
        "source": destack._generated.dir.type.type.to_json_global_type_id(value.source),
        "target": destack._generated.dir.type.type.to_json_global_type_id(value.target),
        "origin": destack._generated.dir.tree.cast.to_json_cast_origin(value.origin),
    }


def from_json_coercion(value: Json) -> Coercion:
    """Return one Coercion from one JSON value."""
    object_ = json_object(value)

    return Coercion(
        source=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "source")
        ),
        target=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "target")
        ),
        origin=destack._generated.dir.tree.cast.from_json_cast_origin(
            json_field(object_, "origin")
        ),
    )


__all__ = [
    "CoercionSegment",
    "encode_coercion_segment",
    "decode_coercion_segment",
    "to_json_coercion_segment",
    "from_json_coercion_segment",
    "Coercion",
    "encode_coercion",
    "decode_coercion",
    "to_json_coercion",
    "from_json_coercion",
]
