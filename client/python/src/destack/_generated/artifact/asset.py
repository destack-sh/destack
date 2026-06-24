# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
    json_optional,
)

import destack._generated.artifact.map
import destack._generated.source.file.model.file
import destack._generated.source.file.model.type
import destack._generated.source.file.path.uri


@dataclass(frozen=True, slots=True)
class Asset:
    """One opaque linker input for a target."""

    # the asset file type
    file_type: destack._generated.source.file.model.type.FileType
    # the asset content identity
    content: destack._generated.source.file.model.file.ContentId
    # the source module URI when one exists
    source: destack._generated.source.file.path.uri.Uri | None
    # the source map when one exists
    map: destack._generated.artifact.map.SourceMap | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_asset(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Asset:
        """Decode one Asset."""
        return decode_asset(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_asset(self)

    @classmethod
    def from_json(cls, value: Json) -> Asset:
        """Return one Asset from one JSON value."""
        return from_json_asset(value)


def encode_asset(writer: BinaryWriter, value: Asset) -> None:
    """Encode one Asset."""
    destack._generated.source.file.model.type.encode_file_type(writer, value.file_type)
    destack._generated.source.file.model.file.encode_content_id(writer, value.content)
    if value.source is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.path.uri.encode_uri(writer, value.source)
    if value.map is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.artifact.map.encode_source_map(writer, value.map)


def decode_asset(reader: BinaryReader) -> Asset:
    """Decode one Asset."""
    file_type = destack._generated.source.file.model.type.decode_file_type(reader)
    content = destack._generated.source.file.model.file.decode_content_id(reader)
    source = reader.read_option(
        lambda: destack._generated.source.file.path.uri.decode_uri(reader)
    )
    map = reader.read_option(
        lambda: destack._generated.artifact.map.decode_source_map(reader)
    )

    return Asset(
        file_type=file_type,
        content=content,
        source=source,
        map=map,
    )


def to_json_asset(value: Asset) -> Json:
    """Return one JSON value for one Asset."""
    return {
        "fileType": destack._generated.source.file.model.type.to_json_file_type(
            value.file_type
        ),
        "content": destack._generated.source.file.model.file.to_json_content_id(
            value.content
        ),
        **(
            {}
            if value.source is None
            else {
                "source": destack._generated.source.file.path.uri.to_json_uri(
                    value.source
                )
            }
        ),
        **(
            {}
            if value.map is None
            else {"map": destack._generated.artifact.map.to_json_source_map(value.map)}
        ),
    }


def from_json_asset(value: Json) -> Asset:
    """Return one Asset from one JSON value."""
    object_ = json_object(value)

    return Asset(
        file_type=destack._generated.source.file.model.type.from_json_file_type(
            json_field(object_, "fileType")
        ),
        content=destack._generated.source.file.model.file.from_json_content_id(
            json_field(object_, "content")
        ),
        source=json_optional(
            object_,
            "source",
            lambda value: destack._generated.source.file.path.uri.from_json_uri(value),
        ),
        map=json_optional(
            object_,
            "map",
            lambda value: destack._generated.artifact.map.from_json_source_map(value),
        ),
    )


__all__ = [
    "Asset",
    "encode_asset",
    "decode_asset",
    "to_json_asset",
    "from_json_asset",
]
