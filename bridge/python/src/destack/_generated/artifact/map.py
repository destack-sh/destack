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
    json_int,
    json_object,
    json_optional,
    json_string,
)


@dataclass(frozen=True, slots=True)
class SourceMap:
    """One emitted or linked source map."""

    # the source map version
    version: int
    # the emitted file name when one exists
    file: str | None
    # the source root when one exists
    source_root: str | None
    # the mapped source names
    sources: Sequence[str]
    # the embedded source contents when they exist
    sources_content: Sequence[str | None] | None
    # the recorded symbol names
    names: Sequence[str]
    # the VLQ mapping payload
    mappings: str
    # the debug id when one exists
    debug_id: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_source_map(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SourceMap:
        """Decode one SourceMap."""
        return decode_source_map(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_source_map(self)

    @classmethod
    def from_json(cls, value: Json) -> SourceMap:
        """Return one SourceMap from one JSON value."""
        return from_json_source_map(value)


def encode_source_map(writer: BinaryWriter, value: SourceMap) -> None:
    """Encode one SourceMap."""
    writer.write_unsigned(value.version)
    if value.file is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.file)
    if value.source_root is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.source_root)
    writer.write_unsigned(len(value.sources))
    for item_value_sources_0 in value.sources:
        writer.write_string(item_value_sources_0)
    if value.sources_content is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.sources_content))
        for item_value_sources_content_1 in value.sources_content:
            if item_value_sources_content_1 is None:
                writer.write_byte(0)
            else:
                writer.write_byte(1)
                writer.write_string(item_value_sources_content_1)
    writer.write_unsigned(len(value.names))
    for item_value_names_0 in value.names:
        writer.write_string(item_value_names_0)
    writer.write_string(value.mappings)
    if value.debug_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.debug_id)


def decode_source_map(reader: BinaryReader) -> SourceMap:
    """Decode one SourceMap."""
    version = reader.read_number()
    file = reader.read_option(lambda: reader.read_string())
    source_root = reader.read_option(lambda: reader.read_string())
    sources = [reader.read_string() for _ in range(reader.read_number())]
    sources_content = reader.read_option(
        lambda: [
            reader.read_option(lambda: reader.read_string())
            for _ in range(reader.read_number())
        ]
    )
    names = [reader.read_string() for _ in range(reader.read_number())]
    mappings = reader.read_string()
    debug_id = reader.read_option(lambda: reader.read_string())

    return SourceMap(
        version=version,
        file=file,
        source_root=source_root,
        sources=sources,
        sources_content=sources_content,
        names=names,
        mappings=mappings,
        debug_id=debug_id,
    )


def to_json_source_map(value: SourceMap) -> Json:
    """Return one JSON value for one SourceMap."""
    return {
        "version": value.version,
        **({} if value.file is None else {"file": value.file}),
        **({} if value.source_root is None else {"sourceRoot": value.source_root}),
        "sources": [item_0 for item_0 in value.sources],
        **(
            {}
            if value.sources_content is None
            else {
                "sourcesContent": [
                    None if item_0 is None else item_0
                    for item_0 in value.sources_content
                ]
            }
        ),
        "names": [item_0 for item_0 in value.names],
        "mappings": value.mappings,
        **({} if value.debug_id is None else {"debugId": value.debug_id}),
    }


def from_json_source_map(value: Json) -> SourceMap:
    """Return one SourceMap from one JSON value."""
    object_ = json_object(value)

    return SourceMap(
        version=json_int(json_field(object_, "version")),
        file=json_optional(object_, "file", lambda value: json_string(value)),
        source_root=json_optional(
            object_, "sourceRoot", lambda value: json_string(value)
        ),
        sources=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "sources"))
        ],
        sources_content=json_optional(
            object_,
            "sourcesContent",
            lambda value: [
                None if item_0 is None else json_string(item_0)
                for item_0 in json_array(value)
            ],
        ),
        names=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "names"))
        ],
        mappings=json_string(json_field(object_, "mappings")),
        debug_id=json_optional(object_, "debugId", lambda value: json_string(value)),
    )


__all__ = [
    "SourceMap",
    "encode_source_map",
    "decode_source_map",
    "to_json_source_map",
    "from_json_source_map",
]
