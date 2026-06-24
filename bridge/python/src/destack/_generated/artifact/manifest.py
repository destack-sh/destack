# generated bridge target, do not edit

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
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
)


@dataclass(frozen=True, slots=True)
class BuildManifestFile:
    """One public build manifest file record."""

    # the emitted output path, relative to the output root when possible
    path: str
    # the public file kind
    type: BuildManifestFileType
    # the emitted file loader
    loader: BuildManifestLoader
    # the logical chunk name when one exists
    name: str | None
    # the source input that produced this file when one exists
    input: str | None
    # whether this file is one entry output
    is_entry: bool | None
    # whether this file is one dynamic entry output
    is_dynamic_entry: bool | None
    # imported chunks or external specifiers
    imports: Sequence[str]
    # dynamically imported chunks or external specifiers
    dynamic_imports: Sequence[str]
    # associated emitted stylesheets
    stylesheets: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_build_manifest_file(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BuildManifestFile:
        """Decode one BuildManifestFile."""
        return decode_build_manifest_file(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_build_manifest_file(self)

    @classmethod
    def from_json(cls, value: Json) -> BuildManifestFile:
        """Return one BuildManifestFile from one JSON value."""
        return from_json_build_manifest_file(value)


def encode_build_manifest_file(writer: BinaryWriter, value: BuildManifestFile) -> None:
    """Encode one BuildManifestFile."""
    writer.write_string(value.path)
    encode_build_manifest_file_type(writer, value.type)
    encode_build_manifest_loader(writer, value.loader)
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
    if value.input is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.input)
    if value.is_entry is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_bool(value.is_entry)
    if value.is_dynamic_entry is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_bool(value.is_dynamic_entry)
    writer.write_unsigned(len(value.imports))
    for item_value_imports_0 in value.imports:
        writer.write_string(item_value_imports_0)
    writer.write_unsigned(len(value.dynamic_imports))
    for item_value_dynamic_imports_0 in value.dynamic_imports:
        writer.write_string(item_value_dynamic_imports_0)
    writer.write_unsigned(len(value.stylesheets))
    for item_value_stylesheets_0 in value.stylesheets:
        writer.write_string(item_value_stylesheets_0)


def decode_build_manifest_file(reader: BinaryReader) -> BuildManifestFile:
    """Decode one BuildManifestFile."""
    path = reader.read_string()
    type = decode_build_manifest_file_type(reader)
    loader = decode_build_manifest_loader(reader)
    name = reader.read_option(lambda: reader.read_string())
    input = reader.read_option(lambda: reader.read_string())
    is_entry = reader.read_option(lambda: reader.read_bool())
    is_dynamic_entry = reader.read_option(lambda: reader.read_bool())
    imports = [reader.read_string() for _ in range(reader.read_number())]
    dynamic_imports = [reader.read_string() for _ in range(reader.read_number())]
    stylesheets = [reader.read_string() for _ in range(reader.read_number())]

    return BuildManifestFile(
        path=path,
        type=type,
        loader=loader,
        name=name,
        input=input,
        is_entry=is_entry,
        is_dynamic_entry=is_dynamic_entry,
        imports=imports,
        dynamic_imports=dynamic_imports,
        stylesheets=stylesheets,
    )


def to_json_build_manifest_file(value: BuildManifestFile) -> Json:
    """Return one JSON value for one BuildManifestFile."""
    return {
        "path": value.path,
        "type": to_json_build_manifest_file_type(value.type),
        "loader": to_json_build_manifest_loader(value.loader),
        **({} if value.name is None else {"name": value.name}),
        **({} if value.input is None else {"input": value.input}),
        **({} if value.is_entry is None else {"isEntry": value.is_entry}),
        **(
            {}
            if value.is_dynamic_entry is None
            else {"isDynamicEntry": value.is_dynamic_entry}
        ),
        "imports": [item_0 for item_0 in value.imports],
        "dynamicImports": [item_0 for item_0 in value.dynamic_imports],
        "stylesheets": [item_0 for item_0 in value.stylesheets],
    }


def from_json_build_manifest_file(value: Json) -> BuildManifestFile:
    """Return one BuildManifestFile from one JSON value."""
    object_ = json_object(value)

    return BuildManifestFile(
        path=json_string(json_field(object_, "path")),
        type=from_json_build_manifest_file_type(json_field(object_, "type")),
        loader=from_json_build_manifest_loader(json_field(object_, "loader")),
        name=json_optional(object_, "name", lambda value: json_string(value)),
        input=json_optional(object_, "input", lambda value: json_string(value)),
        is_entry=json_optional(object_, "isEntry", lambda value: json_bool(value)),
        is_dynamic_entry=json_optional(
            object_, "isDynamicEntry", lambda value: json_bool(value)
        ),
        imports=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "imports"))
        ],
        dynamic_imports=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "dynamicImports"))
        ],
        stylesheets=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "stylesheets"))
        ],
    )


"""One public build manifest file kind."""
BuildManifestFileType: typing.TypeAlias = (
    typing.Literal["chunk"] | typing.Literal["asset"] | typing.Literal["binary"]
)


def encode_build_manifest_file_type(
    writer: BinaryWriter, value: BuildManifestFileType
) -> None:
    """Encode one BuildManifestFileType."""
    if value == "chunk":
        writer.write_unsigned(0)
    elif value == "asset":
        writer.write_unsigned(1)
    elif value == "binary":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_build_manifest_file_type(reader: BinaryReader) -> BuildManifestFileType:
    """Decode one BuildManifestFileType."""
    variant = reader.read_number()

    if variant == 0:
        return "chunk"
    elif variant == 1:
        return "asset"
    elif variant == 2:
        return "binary"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_build_manifest_file_type(value: BuildManifestFileType) -> Json:
    """Return one JSON value for one BuildManifestFileType."""
    return value


def from_json_build_manifest_file_type(value: Json) -> BuildManifestFileType:
    """Return one BuildManifestFileType from one JSON value."""
    variant = json_string(value)

    if variant == "chunk":
        return "chunk"
    elif variant == "asset":
        return "asset"
    elif variant == "binary":
        return "binary"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""One public build manifest loader name."""
BuildManifestLoader: typing.TypeAlias = (
    typing.Literal["js"]
    | typing.Literal["css"]
    | typing.Literal["ts"]
    | typing.Literal["map"]
    | typing.Literal["json"]
    | typing.Literal["dts"]
    | typing.Literal["wasm"]
    | typing.Literal["object"]
    | typing.Literal["asset"]
)


def encode_build_manifest_loader(
    writer: BinaryWriter, value: BuildManifestLoader
) -> None:
    """Encode one BuildManifestLoader."""
    if value == "js":
        writer.write_unsigned(0)
    elif value == "css":
        writer.write_unsigned(1)
    elif value == "ts":
        writer.write_unsigned(2)
    elif value == "map":
        writer.write_unsigned(3)
    elif value == "json":
        writer.write_unsigned(4)
    elif value == "dts":
        writer.write_unsigned(5)
    elif value == "wasm":
        writer.write_unsigned(6)
    elif value == "object":
        writer.write_unsigned(7)
    elif value == "asset":
        writer.write_unsigned(8)
    else:
        raise SerdeError("unknown enum variant")


def decode_build_manifest_loader(reader: BinaryReader) -> BuildManifestLoader:
    """Decode one BuildManifestLoader."""
    variant = reader.read_number()

    if variant == 0:
        return "js"
    elif variant == 1:
        return "css"
    elif variant == 2:
        return "ts"
    elif variant == 3:
        return "map"
    elif variant == 4:
        return "json"
    elif variant == 5:
        return "dts"
    elif variant == 6:
        return "wasm"
    elif variant == 7:
        return "object"
    elif variant == 8:
        return "asset"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_build_manifest_loader(value: BuildManifestLoader) -> Json:
    """Return one JSON value for one BuildManifestLoader."""
    return value


def from_json_build_manifest_loader(value: Json) -> BuildManifestLoader:
    """Return one BuildManifestLoader from one JSON value."""
    variant = json_string(value)

    if variant == "js":
        return "js"
    elif variant == "css":
        return "css"
    elif variant == "ts":
        return "ts"
    elif variant == "map":
        return "map"
    elif variant == "json":
        return "json"
    elif variant == "dts":
        return "dts"
    elif variant == "wasm":
        return "wasm"
    elif variant == "object":
        return "object"
    elif variant == "asset":
        return "asset"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class BuildManifest:
    """One public build manifest for one linked target."""

    # the primary entry path when one exists
    index: str | None
    # the emitted file records for this target
    files: Sequence[BuildManifestFile]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_build_manifest(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BuildManifest:
        """Decode one BuildManifest."""
        return decode_build_manifest(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_build_manifest(self)

    @classmethod
    def from_json(cls, value: Json) -> BuildManifest:
        """Return one BuildManifest from one JSON value."""
        return from_json_build_manifest(value)


def encode_build_manifest(writer: BinaryWriter, value: BuildManifest) -> None:
    """Encode one BuildManifest."""
    if value.index is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.index)
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        encode_build_manifest_file(writer, item_value_files_0)


def decode_build_manifest(reader: BinaryReader) -> BuildManifest:
    """Decode one BuildManifest."""
    index = reader.read_option(lambda: reader.read_string())
    files = [decode_build_manifest_file(reader) for _ in range(reader.read_number())]

    return BuildManifest(
        index=index,
        files=files,
    )


def to_json_build_manifest(value: BuildManifest) -> Json:
    """Return one JSON value for one BuildManifest."""
    return {
        **({} if value.index is None else {"index": value.index}),
        "files": [to_json_build_manifest_file(item_0) for item_0 in value.files],
    }


def from_json_build_manifest(value: Json) -> BuildManifest:
    """Return one BuildManifest from one JSON value."""
    object_ = json_object(value)

    return BuildManifest(
        index=json_optional(object_, "index", lambda value: json_string(value)),
        files=[
            from_json_build_manifest_file(item_0)
            for item_0 in json_array(json_field(object_, "files"))
        ],
    )


__all__ = [
    "BuildManifestFile",
    "encode_build_manifest_file",
    "decode_build_manifest_file",
    "to_json_build_manifest_file",
    "from_json_build_manifest_file",
    "BuildManifestFileType",
    "encode_build_manifest_file_type",
    "decode_build_manifest_file_type",
    "to_json_build_manifest_file_type",
    "from_json_build_manifest_file_type",
    "BuildManifestLoader",
    "encode_build_manifest_loader",
    "decode_build_manifest_loader",
    "to_json_build_manifest_loader",
    "from_json_build_manifest_loader",
    "BuildManifest",
    "encode_build_manifest",
    "decode_build_manifest",
    "to_json_build_manifest",
    "from_json_build_manifest",
]
