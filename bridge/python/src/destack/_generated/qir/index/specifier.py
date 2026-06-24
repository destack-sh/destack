# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_array_length,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.source.file.model.file
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class SpecifierIndex:
    """Import specifier rewrite index."""

    # the resolved specifier entries ordered by target path
    by_target_path: Sequence[tuple[str, SpecifierEntry]]
    # the specifier entries without a resolved target path
    unresolved: Sequence[SpecifierEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_specifier_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SpecifierIndex:
        """Decode one SpecifierIndex."""
        return decode_specifier_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_specifier_index(self)

    @classmethod
    def from_json(cls, value: Json) -> SpecifierIndex:
        """Return one SpecifierIndex from one JSON value."""
        return from_json_specifier_index(value)


def encode_specifier_index(writer: BinaryWriter, value: SpecifierIndex) -> None:
    """Encode one SpecifierIndex."""
    writer.write_unsigned(len(value.by_target_path))
    for item_value_by_target_path_0 in value.by_target_path:
        writer.write_string(item_value_by_target_path_0[0])
        encode_specifier_entry(writer, item_value_by_target_path_0[1])
    writer.write_unsigned(len(value.unresolved))
    for item_value_unresolved_0 in value.unresolved:
        encode_specifier_entry(writer, item_value_unresolved_0)


def decode_specifier_index(reader: BinaryReader) -> SpecifierIndex:
    """Decode one SpecifierIndex."""
    by_target_path = [
        (
            reader.read_string(),
            decode_specifier_entry(reader),
        )
        for _ in range(reader.read_number())
    ]
    unresolved = [decode_specifier_entry(reader) for _ in range(reader.read_number())]

    return SpecifierIndex(
        by_target_path=by_target_path,
        unresolved=unresolved,
    )


def to_json_specifier_index(value: SpecifierIndex) -> Json:
    """Return one JSON value for one SpecifierIndex."""
    return {
        "byTargetPath": [
            [item_0[0], to_json_specifier_entry(item_0[1])]
            for item_0 in value.by_target_path
        ],
        "unresolved": [to_json_specifier_entry(item_0) for item_0 in value.unresolved],
    }


def from_json_specifier_index(value: Json) -> SpecifierIndex:
    """Return one SpecifierIndex from one JSON value."""
    object_ = json_object(value)

    return SpecifierIndex(
        by_target_path=[
            (
                lambda items: (
                    json_string(items[0]),
                    from_json_specifier_entry(items[1]),
                )
            )(json_array_length(item_0, 2))
            for item_0 in json_array(json_field(object_, "byTargetPath"))
        ],
        unresolved=[
            from_json_specifier_entry(item_0)
            for item_0 in json_array(json_field(object_, "unresolved"))
        ],
    )


@dataclass(frozen=True, slots=True)
class SpecifierEntry:
    """Import specifier rewrite entry."""

    # the module containing the specifier
    module_id: destack._generated.source.file.model.module.ModuleId
    # the source file
    file_id: destack._generated.source.file.model.file.FileId
    # the source node that owns the specifier
    source_node_id: int
    # the specifier text
    specifier: str
    # the semantic target module when resolved
    target_module_id: destack._generated.source.file.model.module.ModuleId | None
    # the semantic target path when known
    target_path: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_specifier_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SpecifierEntry:
        """Decode one SpecifierEntry."""
        return decode_specifier_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_specifier_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> SpecifierEntry:
        """Return one SpecifierEntry from one JSON value."""
        return from_json_specifier_entry(value)


def encode_specifier_entry(writer: BinaryWriter, value: SpecifierEntry) -> None:
    """Encode one SpecifierEntry."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    destack._generated.source.file.model.file.encode_file_id(writer, value.file_id)
    writer.write_unsigned(value.source_node_id)
    writer.write_string(value.specifier)
    if value.target_module_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.target_module_id
        )
    if value.target_path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.target_path)


def decode_specifier_entry(reader: BinaryReader) -> SpecifierEntry:
    """Decode one SpecifierEntry."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    file_id = destack._generated.source.file.model.file.decode_file_id(reader)
    source_node_id = reader.read_number()
    specifier = reader.read_string()
    target_module_id = reader.read_option(
        lambda: destack._generated.source.file.model.module.decode_module_id(reader)
    )
    target_path = reader.read_option(lambda: reader.read_string())

    return SpecifierEntry(
        module_id=module_id,
        file_id=file_id,
        source_node_id=source_node_id,
        specifier=specifier,
        target_module_id=target_module_id,
        target_path=target_path,
    )


def to_json_specifier_entry(value: SpecifierEntry) -> Json:
    """Return one JSON value for one SpecifierEntry."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "fileId": destack._generated.source.file.model.file.to_json_file_id(
            value.file_id
        ),
        "sourceNodeId": value.source_node_id,
        "specifier": value.specifier,
        **(
            {}
            if value.target_module_id is None
            else {
                "targetModuleId": destack._generated.source.file.model.module.to_json_module_id(
                    value.target_module_id
                )
            }
        ),
        **({} if value.target_path is None else {"targetPath": value.target_path}),
    }


def from_json_specifier_entry(value: Json) -> SpecifierEntry:
    """Return one SpecifierEntry from one JSON value."""
    object_ = json_object(value)

    return SpecifierEntry(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        file_id=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "fileId")
        ),
        source_node_id=json_int(json_field(object_, "sourceNodeId")),
        specifier=json_string(json_field(object_, "specifier")),
        target_module_id=json_optional(
            object_,
            "targetModuleId",
            lambda value: (
                destack._generated.source.file.model.module.from_json_module_id(value)
            ),
        ),
        target_path=json_optional(
            object_, "targetPath", lambda value: json_string(value)
        ),
    )


__all__ = [
    "SpecifierIndex",
    "encode_specifier_index",
    "decode_specifier_index",
    "to_json_specifier_index",
    "from_json_specifier_index",
    "SpecifierEntry",
    "encode_specifier_entry",
    "decode_specifier_entry",
    "to_json_specifier_entry",
    "from_json_specifier_entry",
]
