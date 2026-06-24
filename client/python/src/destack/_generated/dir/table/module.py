# generated client target, do not edit

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

import destack._generated.dir.symbol.module
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class ModuleSegment:
    """Module import edges added by one DIR phase."""

    # the module id of the segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # locally resolved module import edges
    edges: Sequence[destack._generated.dir.symbol.module.ModuleEdge]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_module_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleSegment:
        """Decode one ModuleSegment."""
        return decode_module_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_module_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> ModuleSegment:
        """Return one ModuleSegment from one JSON value."""
        return from_json_module_segment(value)


def encode_module_segment(writer: BinaryWriter, value: ModuleSegment) -> None:
    """Encode one ModuleSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(len(value.edges))
    for item_value_edges_0 in value.edges:
        destack._generated.dir.symbol.module.encode_module_edge(
            writer, item_value_edges_0
        )


def decode_module_segment(reader: BinaryReader) -> ModuleSegment:
    """Decode one ModuleSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    edges = [
        destack._generated.dir.symbol.module.decode_module_edge(reader)
        for _ in range(reader.read_number())
    ]

    return ModuleSegment(
        module_id=module_id,
        edges=edges,
    )


def to_json_module_segment(value: ModuleSegment) -> Json:
    """Return one JSON value for one ModuleSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "edges": [
            destack._generated.dir.symbol.module.to_json_module_edge(item_0)
            for item_0 in value.edges
        ],
    }


def from_json_module_segment(value: Json) -> ModuleSegment:
    """Return one ModuleSegment from one JSON value."""
    object_ = json_object(value)

    return ModuleSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        edges=[
            destack._generated.dir.symbol.module.from_json_module_edge(item_0)
            for item_0 in json_array(json_field(object_, "edges"))
        ],
    )


__all__ = [
    "ModuleSegment",
    "encode_module_segment",
    "decode_module_segment",
    "to_json_module_segment",
    "from_json_module_segment",
]
