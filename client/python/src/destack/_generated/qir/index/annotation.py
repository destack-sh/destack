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
    json_optional,
    json_string,
)

import destack._generated.dir.tree.node
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class AnnotationIndex:
    """Searchable annotation and decorator index."""

    # the annotation entries in stable order
    entries: Sequence[AnnotationEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationIndex:
        """Decode one AnnotationIndex."""
        return decode_annotation_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation_index(self)

    @classmethod
    def from_json(cls, value: Json) -> AnnotationIndex:
        """Return one AnnotationIndex from one JSON value."""
        return from_json_annotation_index(value)


def encode_annotation_index(writer: BinaryWriter, value: AnnotationIndex) -> None:
    """Encode one AnnotationIndex."""
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_annotation_entry(writer, item_value_entries_0)


def decode_annotation_index(reader: BinaryReader) -> AnnotationIndex:
    """Decode one AnnotationIndex."""
    entries = [decode_annotation_entry(reader) for _ in range(reader.read_number())]

    return AnnotationIndex(
        entries=entries,
    )


def to_json_annotation_index(value: AnnotationIndex) -> Json:
    """Return one JSON value for one AnnotationIndex."""
    return {
        "entries": [to_json_annotation_entry(item_0) for item_0 in value.entries],
    }


def from_json_annotation_index(value: Json) -> AnnotationIndex:
    """Return one AnnotationIndex from one JSON value."""
    object_ = json_object(value)

    return AnnotationIndex(
        entries=[
            from_json_annotation_entry(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
    )


@dataclass(frozen=True, slots=True)
class AnnotationEntry:
    """Searchable annotation or decorator entry."""

    # the annotation name when syntactically known
    name: str | None
    # the module that owns the annotation
    module_id: destack._generated.source.file.model.module.ModuleId
    # the decorator node
    decorator_id: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the annotated target node
    target_id: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationEntry:
        """Decode one AnnotationEntry."""
        return decode_annotation_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> AnnotationEntry:
        """Return one AnnotationEntry from one JSON value."""
        return from_json_annotation_entry(value)


def encode_annotation_entry(writer: BinaryWriter, value: AnnotationEntry) -> None:
    """Encode one AnnotationEntry."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    destack._generated.dir.tree.node.encode_global_node_id_any(
        writer, value.decorator_id
    )
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.target_id)


def decode_annotation_entry(reader: BinaryReader) -> AnnotationEntry:
    """Decode one AnnotationEntry."""
    name = reader.read_option(lambda: reader.read_string())
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    decorator_id = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    target_id = destack._generated.dir.tree.node.decode_global_node_id_any(reader)

    return AnnotationEntry(
        name=name,
        module_id=module_id,
        decorator_id=decorator_id,
        target_id=target_id,
    )


def to_json_annotation_entry(value: AnnotationEntry) -> Json:
    """Return one JSON value for one AnnotationEntry."""
    return {
        **({} if value.name is None else {"name": value.name}),
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "decoratorId": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.decorator_id
        ),
        "targetId": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.target_id
        ),
    }


def from_json_annotation_entry(value: Json) -> AnnotationEntry:
    """Return one AnnotationEntry from one JSON value."""
    object_ = json_object(value)

    return AnnotationEntry(
        name=json_optional(object_, "name", lambda value: json_string(value)),
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        decorator_id=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "decoratorId")
        ),
        target_id=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "targetId")
        ),
    )


__all__ = [
    "AnnotationIndex",
    "encode_annotation_index",
    "decode_annotation_index",
    "to_json_annotation_index",
    "from_json_annotation_index",
    "AnnotationEntry",
    "encode_annotation_entry",
    "decode_annotation_entry",
    "to_json_annotation_entry",
    "from_json_annotation_entry",
]
