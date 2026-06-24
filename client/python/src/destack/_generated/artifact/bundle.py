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
    json_optional,
    json_string,
)

import destack._generated.artifact.emit
import destack._generated.source.file.model.file
import destack._generated.source.file.model.type
import destack._generated.source.file.path.uri

"""One section of a linked bundle."""
BundleSection: typing.TypeAlias = (
    typing.Literal["module"]
    | typing.Literal["entry"]
    | typing.Literal["declaration"]
    | typing.Literal["asset"]
    | typing.Literal["manifest"]
    | typing.Literal["sourceMap"]
    | typing.Literal["native"]
)


def encode_bundle_section(writer: BinaryWriter, value: BundleSection) -> None:
    """Encode one BundleSection."""
    if value == "module":
        writer.write_unsigned(0)
    elif value == "entry":
        writer.write_unsigned(1)
    elif value == "declaration":
        writer.write_unsigned(2)
    elif value == "asset":
        writer.write_unsigned(3)
    elif value == "manifest":
        writer.write_unsigned(4)
    elif value == "sourceMap":
        writer.write_unsigned(5)
    elif value == "native":
        writer.write_unsigned(6)
    else:
        raise SerdeError("unknown enum variant")


def decode_bundle_section(reader: BinaryReader) -> BundleSection:
    """Decode one BundleSection."""
    variant = reader.read_number()

    if variant == 0:
        return "module"
    elif variant == 1:
        return "entry"
    elif variant == 2:
        return "declaration"
    elif variant == 3:
        return "asset"
    elif variant == 4:
        return "manifest"
    elif variant == 5:
        return "sourceMap"
    elif variant == 6:
        return "native"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_bundle_section(value: BundleSection) -> Json:
    """Return one JSON value for one BundleSection."""
    return value


def from_json_bundle_section(value: Json) -> BundleSection:
    """Return one BundleSection from one JSON value."""
    variant = json_string(value)

    if variant == "module":
        return "module"
    elif variant == "entry":
        return "entry"
    elif variant == "declaration":
        return "declaration"
    elif variant == "asset":
        return "asset"
    elif variant == "manifest":
        return "manifest"
    elif variant == "sourceMap":
        return "sourceMap"
    elif variant == "native":
        return "native"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The assembly mode for one bundle."""
BundleMode: typing.TypeAlias = (
    typing.Literal["preserveModules"]
    | typing.Literal["singleFile"]
    | typing.Literal["chunked"]
)


def encode_bundle_mode(writer: BinaryWriter, value: BundleMode) -> None:
    """Encode one BundleMode."""
    if value == "preserveModules":
        writer.write_unsigned(0)
    elif value == "singleFile":
        writer.write_unsigned(1)
    elif value == "chunked":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_bundle_mode(reader: BinaryReader) -> BundleMode:
    """Decode one BundleMode."""
    variant = reader.read_number()

    if variant == 0:
        return "preserveModules"
    elif variant == 1:
        return "singleFile"
    elif variant == 2:
        return "chunked"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_bundle_mode(value: BundleMode) -> Json:
    """Return one JSON value for one BundleMode."""
    return value


def from_json_bundle_mode(value: Json) -> BundleMode:
    """Return one BundleMode from one JSON value."""
    variant = json_string(value)

    if variant == "preserveModules":
        return "preserveModules"
    elif variant == "singleFile":
        return "singleFile"
    elif variant == "chunked":
        return "chunked"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class BundleFile:
    """One derived output file."""

    # the bundle section this file belongs to
    section: BundleSection
    # the output URI
    uri: destack._generated.source.file.path.uri.Uri
    # the emitted file type
    file_type: destack._generated.source.file.model.type.FileType
    # the output content identity
    content: destack._generated.source.file.model.file.ContentId
    # the related source URI when one exists
    source: destack._generated.source.file.path.uri.Uri | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_bundle_file(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BundleFile:
        """Decode one BundleFile."""
        return decode_bundle_file(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_bundle_file(self)

    @classmethod
    def from_json(cls, value: Json) -> BundleFile:
        """Return one BundleFile from one JSON value."""
        return from_json_bundle_file(value)


def encode_bundle_file(writer: BinaryWriter, value: BundleFile) -> None:
    """Encode one BundleFile."""
    encode_bundle_section(writer, value.section)
    destack._generated.source.file.path.uri.encode_uri(writer, value.uri)
    destack._generated.source.file.model.type.encode_file_type(writer, value.file_type)
    destack._generated.source.file.model.file.encode_content_id(writer, value.content)
    if value.source is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.path.uri.encode_uri(writer, value.source)


def decode_bundle_file(reader: BinaryReader) -> BundleFile:
    """Decode one BundleFile."""
    section = decode_bundle_section(reader)
    uri = destack._generated.source.file.path.uri.decode_uri(reader)
    file_type = destack._generated.source.file.model.type.decode_file_type(reader)
    content = destack._generated.source.file.model.file.decode_content_id(reader)
    source = reader.read_option(
        lambda: destack._generated.source.file.path.uri.decode_uri(reader)
    )

    return BundleFile(
        section=section,
        uri=uri,
        file_type=file_type,
        content=content,
        source=source,
    )


def to_json_bundle_file(value: BundleFile) -> Json:
    """Return one JSON value for one BundleFile."""
    return {
        "section": to_json_bundle_section(value.section),
        "uri": destack._generated.source.file.path.uri.to_json_uri(value.uri),
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
    }


def from_json_bundle_file(value: Json) -> BundleFile:
    """Return one BundleFile from one JSON value."""
    object_ = json_object(value)

    return BundleFile(
        section=from_json_bundle_section(json_field(object_, "section")),
        uri=destack._generated.source.file.path.uri.from_json_uri(
            json_field(object_, "uri")
        ),
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
    )


@dataclass(frozen=True, slots=True)
class Bundle:
    """One linked file graph for one target."""

    # the emitted artifact family
    emit: destack._generated.artifact.emit.EmitFormat
    # the target-level assembly mode
    assembly: BundleMode
    # the files in this bundle
    files: Sequence[BundleFile]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_bundle(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Bundle:
        """Decode one Bundle."""
        return decode_bundle(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_bundle(self)

    @classmethod
    def from_json(cls, value: Json) -> Bundle:
        """Return one Bundle from one JSON value."""
        return from_json_bundle(value)


def encode_bundle(writer: BinaryWriter, value: Bundle) -> None:
    """Encode one Bundle."""
    destack._generated.artifact.emit.encode_emit_format(writer, value.emit)
    encode_bundle_mode(writer, value.assembly)
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        encode_bundle_file(writer, item_value_files_0)


def decode_bundle(reader: BinaryReader) -> Bundle:
    """Decode one Bundle."""
    emit = destack._generated.artifact.emit.decode_emit_format(reader)
    assembly = decode_bundle_mode(reader)
    files = [decode_bundle_file(reader) for _ in range(reader.read_number())]

    return Bundle(
        emit=emit,
        assembly=assembly,
        files=files,
    )


def to_json_bundle(value: Bundle) -> Json:
    """Return one JSON value for one Bundle."""
    return {
        "emit": destack._generated.artifact.emit.to_json_emit_format(value.emit),
        "assembly": to_json_bundle_mode(value.assembly),
        "files": [to_json_bundle_file(item_0) for item_0 in value.files],
    }


def from_json_bundle(value: Json) -> Bundle:
    """Return one Bundle from one JSON value."""
    object_ = json_object(value)

    return Bundle(
        emit=destack._generated.artifact.emit.from_json_emit_format(
            json_field(object_, "emit")
        ),
        assembly=from_json_bundle_mode(json_field(object_, "assembly")),
        files=[
            from_json_bundle_file(item_0)
            for item_0 in json_array(json_field(object_, "files"))
        ],
    )


__all__ = [
    "BundleSection",
    "encode_bundle_section",
    "decode_bundle_section",
    "to_json_bundle_section",
    "from_json_bundle_section",
    "BundleMode",
    "encode_bundle_mode",
    "decode_bundle_mode",
    "to_json_bundle_mode",
    "from_json_bundle_mode",
    "BundleFile",
    "encode_bundle_file",
    "decode_bundle_file",
    "to_json_bundle_file",
    "from_json_bundle_file",
    "Bundle",
    "encode_bundle",
    "decode_bundle",
    "to_json_bundle",
    "from_json_bundle",
]
