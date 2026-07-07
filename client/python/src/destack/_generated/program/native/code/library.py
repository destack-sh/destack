# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.source.file.model.file


@dataclass(frozen=True, slots=True)
class Library:
    """Loadable native library image."""

    # the native library location
    source: LibrarySource
    # native unwind tables bytes
    unwind: destack._generated.source.file.model.file.ContentId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_library(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Library:
        """Decode one Library."""
        return decode_library(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_library(self)

    @classmethod
    def from_json(cls, value: Json) -> Library:
        """Return one Library from one JSON value."""
        return from_json_library(value)


def encode_library(writer: BinaryWriter, value: Library) -> None:
    """Encode one Library."""
    encode_library_source(writer, value.source)
    if value.unwind is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.file.encode_content_id(
            writer, value.unwind
        )


def decode_library(reader: BinaryReader) -> Library:
    """Decode one Library."""
    source = decode_library_source(reader)
    unwind = reader.read_option(
        lambda: destack._generated.source.file.model.file.decode_content_id(reader)
    )

    return Library(
        source=source,
        unwind=unwind,
    )


def to_json_library(value: Library) -> Json:
    """Return one JSON value for one Library."""
    return {
        "source": to_json_library_source(value.source),
        **(
            {}
            if value.unwind is None
            else {
                "unwind": destack._generated.source.file.model.file.to_json_content_id(
                    value.unwind
                )
            }
        ),
    }


def from_json_library(value: Json) -> Library:
    """Return one Library from one JSON value."""
    object_ = json_object(value)

    return Library(
        source=from_json_library_source(json_field(object_, "source")),
        unwind=json_optional(
            object_,
            "unwind",
            lambda value: (
                destack._generated.source.file.model.file.from_json_content_id(value)
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class LibrarySourceName:
    """Library is loaded from a process or platform search path."""

    name: destack._generated.core.string.StringId
    kind: typing.Literal["name"] = "name"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_library_source(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_library_source(self)


@dataclass(frozen=True, slots=True)
class LibrarySourceArtifact:
    """Library is packaged as a program artifact."""

    artifact: destack._generated.source.file.model.file.ContentId
    kind: typing.Literal["artifact"] = "artifact"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_library_source(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_library_source(self)


"""Native library source."""
LibrarySource: typing.TypeAlias = LibrarySourceName | LibrarySourceArtifact


def encode_library_source(writer: BinaryWriter, value: LibrarySource) -> None:
    """Encode one LibrarySource."""
    if value.kind == "name":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "artifact":
        writer.write_unsigned(1)
        destack._generated.source.file.model.file.encode_content_id(
            writer, value.artifact
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_library_source(reader: BinaryReader) -> LibrarySource:
    """Decode one LibrarySource."""
    variant = reader.read_number()

    if variant == 0:
        name = destack._generated.core.string.decode_string_id(reader)

        return LibrarySourceName(name=name)
    elif variant == 1:
        artifact = destack._generated.source.file.model.file.decode_content_id(reader)

        return LibrarySourceArtifact(artifact=artifact)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_library_source(value: LibrarySource) -> Json:
    """Return one JSON value for one LibrarySource."""
    if value.kind == "name":
        return {
            "kind": "name",
            "name": destack._generated.core.string.to_json_string_id(value.name),
        }
    elif value.kind == "artifact":
        return {
            "kind": "artifact",
            "artifact": destack._generated.source.file.model.file.to_json_content_id(
                value.artifact
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_library_source(value: Json) -> LibrarySource:
    """Return one LibrarySource from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "name":
        return LibrarySourceName(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            )
        )
    elif kind == "artifact":
        return LibrarySourceArtifact(
            artifact=destack._generated.source.file.model.file.from_json_content_id(
                json_field(object_, "artifact")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Library",
    "encode_library",
    "decode_library",
    "to_json_library",
    "from_json_library",
    "LibrarySource",
    "encode_library_source",
    "decode_library_source",
    "to_json_library_source",
    "from_json_library_source",
    "LibrarySourceName",
    "LibrarySourceArtifact",
]
