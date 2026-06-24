# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
)


@dataclass(frozen=True, slots=True)
class TargetOutputOptions:
    """Target output paths and metadata options."""

    # output directory for this target
    directory: str
    # output file for single-file targets
    file: str | None
    # whether to emit declaration files
    declaration: bool
    # separate directory for declaration files
    declaration_directory: str | None
    # source map emission mode
    source_map: SourceMapMode | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_output_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetOutputOptions:
        """Decode one TargetOutputOptions."""
        return decode_target_output_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_output_options(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetOutputOptions:
        """Return one TargetOutputOptions from one JSON value."""
        return from_json_target_output_options(value)


def encode_target_output_options(
    writer: BinaryWriter, value: TargetOutputOptions
) -> None:
    """Encode one TargetOutputOptions."""
    writer.write_string(value.directory)
    if value.file is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.file)
    writer.write_bool(value.declaration)
    if value.declaration_directory is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.declaration_directory)
    if value.source_map is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_source_map_mode(writer, value.source_map)


def decode_target_output_options(reader: BinaryReader) -> TargetOutputOptions:
    """Decode one TargetOutputOptions."""
    directory = reader.read_string()
    file = reader.read_option(lambda: reader.read_string())
    declaration = reader.read_bool()
    declaration_directory = reader.read_option(lambda: reader.read_string())
    source_map = reader.read_option(lambda: decode_source_map_mode(reader))

    return TargetOutputOptions(
        directory=directory,
        file=file,
        declaration=declaration,
        declaration_directory=declaration_directory,
        source_map=source_map,
    )


def to_json_target_output_options(value: TargetOutputOptions) -> Json:
    """Return one JSON value for one TargetOutputOptions."""
    return {
        "directory": value.directory,
        **({} if value.file is None else {"file": value.file}),
        "declaration": value.declaration,
        **(
            {}
            if value.declaration_directory is None
            else {"declarationDirectory": value.declaration_directory}
        ),
        **(
            {}
            if value.source_map is None
            else {"sourceMap": to_json_source_map_mode(value.source_map)}
        ),
    }


def from_json_target_output_options(value: Json) -> TargetOutputOptions:
    """Return one TargetOutputOptions from one JSON value."""
    object_ = json_object(value)

    return TargetOutputOptions(
        directory=json_string(json_field(object_, "directory")),
        file=json_optional(object_, "file", lambda value: json_string(value)),
        declaration=json_bool(json_field(object_, "declaration")),
        declaration_directory=json_optional(
            object_, "declarationDirectory", lambda value: json_string(value)
        ),
        source_map=json_optional(
            object_, "sourceMap", lambda value: from_json_source_map_mode(value)
        ),
    )


"""Source map emission mode for one target."""
SourceMapMode: typing.TypeAlias = (
    typing.Literal["external"] | typing.Literal["inline"] | typing.Literal["hidden"]
)


def encode_source_map_mode(writer: BinaryWriter, value: SourceMapMode) -> None:
    """Encode one SourceMapMode."""
    if value == "external":
        writer.write_unsigned(0)
    elif value == "inline":
        writer.write_unsigned(1)
    elif value == "hidden":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_source_map_mode(reader: BinaryReader) -> SourceMapMode:
    """Decode one SourceMapMode."""
    variant = reader.read_number()

    if variant == 0:
        return "external"
    elif variant == 1:
        return "inline"
    elif variant == 2:
        return "hidden"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_source_map_mode(value: SourceMapMode) -> Json:
    """Return one JSON value for one SourceMapMode."""
    return value


def from_json_source_map_mode(value: Json) -> SourceMapMode:
    """Return one SourceMapMode from one JSON value."""
    variant = json_string(value)

    if variant == "external":
        return "external"
    elif variant == "inline":
        return "inline"
    elif variant == "hidden":
        return "hidden"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "TargetOutputOptions",
    "encode_target_output_options",
    "decode_target_output_options",
    "to_json_target_output_options",
    "from_json_target_output_options",
    "SourceMapMode",
    "encode_source_map_mode",
    "decode_source_map_mode",
    "to_json_source_map_mode",
    "from_json_source_map_mode",
]
