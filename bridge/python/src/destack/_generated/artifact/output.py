# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.source.file
import destack._generated.source.target

if TYPE_CHECKING:
    from destack._generated.source.file import (
        ContentId,
    )

    from destack._generated.source.target import (
        TargetId,
    )

"""Build distribution profile crossing bridge boundaries."""
BuildProfile: TypeAlias = Literal["full"] | Literal["minimal"] | Literal["freestanding"]


def encode_build_profile(writer: Writer, value: BuildProfile) -> None:
    if value == "full":
        writer.write_unsigned(0)
    elif value == "minimal":
        writer.write_unsigned(1)
    elif value == "freestanding":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_build_profile(reader: Reader) -> BuildProfile:
    variant = reader.read_number()

    if variant == 0:
        return "full"
    elif variant == 1:
        return "minimal"
    elif variant == 2:
        return "freestanding"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


"""Build payload linkage crossing bridge boundaries."""
BuildLinkage: TypeAlias = Literal["portable"] | Literal["static"] | Literal["dynamic"]


def encode_build_linkage(writer: Writer, value: BuildLinkage) -> None:
    if value == "portable":
        writer.write_unsigned(0)
    elif value == "static":
        writer.write_unsigned(1)
    elif value == "dynamic":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_build_linkage(reader: Reader) -> BuildLinkage:
    variant = reader.read_number()

    if variant == 0:
        return "portable"
    elif variant == 1:
        return "static"
    elif variant == 2:
        return "dynamic"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


"""Emitted artifact family crossing bridge boundaries."""
EmitFormat: TypeAlias = (
    Literal["js"] | Literal["ts"] | Literal["wasm"] | Literal["native"]
)


def encode_emit_format(writer: Writer, value: EmitFormat) -> None:
    if value == "js":
        writer.write_unsigned(0)
    elif value == "ts":
        writer.write_unsigned(1)
    elif value == "wasm":
        writer.write_unsigned(2)
    elif value == "native":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_emit_format(reader: Reader) -> EmitFormat:
    variant = reader.read_number()

    if variant == 0:
        return "js"
    elif variant == 1:
        return "ts"
    elif variant == 2:
        return "wasm"
    elif variant == 3:
        return "native"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


"""Source file type crossing bridge boundaries."""
FileType: TypeAlias = (
    Literal["destack"]
    | Literal["destackDeclaration"]
    | Literal["javaScript"]
    | Literal["javaScriptXml"]
    | Literal["typeScript"]
    | Literal["typeScriptXml"]
    | Literal["typeScriptDeclaration"]
    | Literal["text"]
    | Literal["toml"]
    | Literal["yaml"]
    | Literal["json"]
    | Literal["env"]
    | Literal["html"]
    | Literal["markdown"]
    | Literal["css"]
    | Literal["svg"]
    | Literal["wasm"]
    | Literal["node"]
    | Literal["sourceMap"]
    | Literal["object"]
    | Literal["image"]
    | Literal["font"]
    | Literal["audio"]
    | Literal["video"]
    | Literal["model"]
    | Literal["neural"]
    | Literal["document"]
    | Literal["binary"]
    | Literal["unknown"]
)


def encode_file_type(writer: Writer, value: FileType) -> None:
    if value == "destack":
        writer.write_unsigned(0)
    elif value == "destackDeclaration":
        writer.write_unsigned(1)
    elif value == "javaScript":
        writer.write_unsigned(2)
    elif value == "javaScriptXml":
        writer.write_unsigned(3)
    elif value == "typeScript":
        writer.write_unsigned(4)
    elif value == "typeScriptXml":
        writer.write_unsigned(5)
    elif value == "typeScriptDeclaration":
        writer.write_unsigned(6)
    elif value == "text":
        writer.write_unsigned(7)
    elif value == "toml":
        writer.write_unsigned(8)
    elif value == "yaml":
        writer.write_unsigned(9)
    elif value == "json":
        writer.write_unsigned(10)
    elif value == "env":
        writer.write_unsigned(11)
    elif value == "html":
        writer.write_unsigned(12)
    elif value == "markdown":
        writer.write_unsigned(13)
    elif value == "css":
        writer.write_unsigned(14)
    elif value == "svg":
        writer.write_unsigned(15)
    elif value == "wasm":
        writer.write_unsigned(16)
    elif value == "node":
        writer.write_unsigned(17)
    elif value == "sourceMap":
        writer.write_unsigned(18)
    elif value == "object":
        writer.write_unsigned(19)
    elif value == "image":
        writer.write_unsigned(20)
    elif value == "font":
        writer.write_unsigned(21)
    elif value == "audio":
        writer.write_unsigned(22)
    elif value == "video":
        writer.write_unsigned(23)
    elif value == "model":
        writer.write_unsigned(24)
    elif value == "neural":
        writer.write_unsigned(25)
    elif value == "document":
        writer.write_unsigned(26)
    elif value == "binary":
        writer.write_unsigned(27)
    elif value == "unknown":
        writer.write_unsigned(28)
    else:
        raise SerdeError("unknown enum variant")


def decode_file_type(reader: Reader) -> FileType:
    variant = reader.read_number()

    if variant == 0:
        return "destack"
    elif variant == 1:
        return "destackDeclaration"
    elif variant == 2:
        return "javaScript"
    elif variant == 3:
        return "javaScriptXml"
    elif variant == 4:
        return "typeScript"
    elif variant == 5:
        return "typeScriptXml"
    elif variant == 6:
        return "typeScriptDeclaration"
    elif variant == 7:
        return "text"
    elif variant == 8:
        return "toml"
    elif variant == 9:
        return "yaml"
    elif variant == 10:
        return "json"
    elif variant == 11:
        return "env"
    elif variant == 12:
        return "html"
    elif variant == 13:
        return "markdown"
    elif variant == 14:
        return "css"
    elif variant == 15:
        return "svg"
    elif variant == 16:
        return "wasm"
    elif variant == 17:
        return "node"
    elif variant == 18:
        return "sourceMap"
    elif variant == 19:
        return "object"
    elif variant == 20:
        return "image"
    elif variant == 21:
        return "font"
    elif variant == 22:
        return "audio"
    elif variant == 23:
        return "video"
    elif variant == 24:
        return "model"
    elif variant == 25:
        return "neural"
    elif variant == 26:
        return "document"
    elif variant == 27:
        return "binary"
    elif variant == 28:
        return "unknown"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class SourceMapSource:
    """One emitted or linked source map crossing bridge boundaries."""

    """The mapped source names."""
    name: str
    """The embedded source contents when they exist."""
    content: str | None


def encode_source_map_source(writer: Writer, value: SourceMapSource) -> None:
    writer.write_string(value.name)
    if value.content is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.content)


def decode_source_map_source(reader: Reader) -> SourceMapSource:
    field_0 = reader.read_string()
    field_1 = reader.read_option(lambda: reader.read_string())

    return SourceMapSource(
        name=field_0,
        content=field_1,
    )


@dataclass(frozen=True, slots=True)
class SourceMap:
    """One emitted or linked source map crossing bridge boundaries."""

    """The source map version."""
    version: int
    """The emitted file name when one exists."""
    file: str | None
    """The source root when one exists."""
    source_root: str | None
    """The mapped sources."""
    sources: Sequence[SourceMapSource]
    """The recorded symbol names."""
    names: Sequence[str]
    """The VLQ mapping payload."""
    mappings: str
    """The debug id when one exists."""
    debug_id: str | None


def encode_source_map(writer: Writer, value: SourceMap) -> None:
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
    for item_0 in value.sources:
        encode_source_map_source(writer, item_0)
    writer.write_unsigned(len(value.names))
    for item_0 in value.names:
        writer.write_string(item_0)
    writer.write_string(value.mappings)
    if value.debug_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.debug_id)


def decode_source_map(reader: Reader) -> SourceMap:
    field_0 = reader.read_number()
    field_1 = reader.read_option(lambda: reader.read_string())
    field_2 = reader.read_option(lambda: reader.read_string())
    field_3 = [decode_source_map_source(reader) for _ in range(reader.read_number())]
    field_4 = [reader.read_string() for _ in range(reader.read_number())]
    field_5 = reader.read_string()
    field_6 = reader.read_option(lambda: reader.read_string())

    return SourceMap(
        version=field_0,
        file=field_1,
        source_root=field_2,
        sources=field_3,
        names=field_4,
        mappings=field_5,
        debug_id=field_6,
    )


@dataclass(frozen=True, slots=True)
class Declaration:
    """One emitted declaration crossing bridge boundaries."""

    """The declaration text."""
    text: str


def encode_declaration(writer: Writer, value: Declaration) -> None:
    writer.write_string(value.text)


def decode_declaration(reader: Reader) -> Declaration:
    field_0 = reader.read_string()

    return Declaration(
        text=field_0,
    )


"""Structured script language crossing bridge boundaries."""
ScriptLanguage: TypeAlias = Literal["javaScript"] | Literal["typeScript"]


def encode_script_language(writer: Writer, value: ScriptLanguage) -> None:
    if value == "javaScript":
        writer.write_unsigned(0)
    elif value == "typeScript":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_script_language(reader: Reader) -> ScriptLanguage:
    variant = reader.read_number()

    if variant == 0:
        return "javaScript"
    elif variant == 1:
        return "typeScript"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class Script:
    """One structured script artifact crossing bridge boundaries."""

    """The target language of this script."""
    language: ScriptLanguage
    """The emitted declaration when one exists."""
    declaration: Declaration | None
    """The source map when one exists."""
    map: SourceMap | None
    """Whether this script has top-level side effects."""
    has_top_level_side_effects: bool


def encode_script(writer: Writer, value: Script) -> None:
    encode_script_language(writer, value.language)
    if value.declaration is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_declaration(writer, value.declaration)
    if value.map is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_source_map(writer, value.map)
    writer.write_bool(value.has_top_level_side_effects)


def decode_script(reader: Reader) -> Script:
    field_0 = decode_script_language(reader)
    field_1 = reader.read_option(lambda: decode_declaration(reader))
    field_2 = reader.read_option(lambda: decode_source_map(reader))
    field_3 = reader.read_bool()

    return Script(
        language=field_0,
        declaration=field_1,
        map=field_2,
        has_top_level_side_effects=field_3,
    )


"""Compiled-code object format crossing bridge boundaries."""
ObjectFormat: TypeAlias = Literal["object"] | Literal["wasm"]


def encode_object_format(writer: Writer, value: ObjectFormat) -> None:
    if value == "object":
        writer.write_unsigned(0)
    elif value == "wasm":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_object_format(reader: Reader) -> ObjectFormat:
    variant = reader.read_number()

    if variant == 0:
        return "object"
    elif variant == 1:
        return "wasm"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class Object:
    """One compiled-code object artifact crossing bridge boundaries."""

    """The compiled-code object format."""
    format: ObjectFormat
    """The encoded object content identity."""
    content: ContentId
    """The source map when one exists."""
    map: SourceMap | None


def encode_object(writer: Writer, value: Object) -> None:
    encode_object_format(writer, value.format)
    destack._generated.source.file.encode_content_id(writer, value.content)
    if value.map is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_source_map(writer, value.map)


def decode_object(reader: Reader) -> Object:
    field_0 = decode_object_format(reader)
    field_1 = destack._generated.source.file.decode_content_id(reader)
    field_2 = reader.read_option(lambda: decode_source_map(reader))

    return Object(
        format=field_0,
        content=field_1,
        map=field_2,
    )


@dataclass(frozen=True, slots=True)
class Asset:
    """One opaque asset artifact crossing bridge boundaries."""

    """The asset file type."""
    file_type: FileType
    """The asset content identity."""
    content: ContentId
    """The source module URI when one exists."""
    source: str | None
    """The source map when one exists."""
    map: SourceMap | None


def encode_asset(writer: Writer, value: Asset) -> None:
    encode_file_type(writer, value.file_type)
    destack._generated.source.file.encode_content_id(writer, value.content)
    if value.source is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.source)
    if value.map is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_source_map(writer, value.map)


def decode_asset(reader: Reader) -> Asset:
    field_0 = decode_file_type(reader)
    field_1 = destack._generated.source.file.decode_content_id(reader)
    field_2 = reader.read_option(lambda: reader.read_string())
    field_3 = reader.read_option(lambda: decode_source_map(reader))

    return Asset(
        file_type=field_0,
        content=field_1,
        source=field_2,
        map=field_3,
    )


@dataclass(frozen=True, slots=True)
class Build:
    """One target-built toolchain payload crossing bridge boundaries."""

    """The build distribution profile."""
    profile: BuildProfile
    """The build linkage."""
    linkage: BuildLinkage
    """The encoded build content."""
    content: ContentId


def encode_build(writer: Writer, value: Build) -> None:
    encode_build_profile(writer, value.profile)
    encode_build_linkage(writer, value.linkage)
    destack._generated.source.file.encode_content_id(writer, value.content)


def decode_build(reader: Reader) -> Build:
    field_0 = decode_build_profile(reader)
    field_1 = decode_build_linkage(reader)
    field_2 = destack._generated.source.file.decode_content_id(reader)

    return Build(
        profile=field_0,
        linkage=field_1,
        content=field_2,
    )


"""One section of a linked bundle crossing bridge boundaries."""
BundleSection: TypeAlias = (
    Literal["module"]
    | Literal["entry"]
    | Literal["declaration"]
    | Literal["asset"]
    | Literal["manifest"]
    | Literal["sourceMap"]
    | Literal["native"]
)


def encode_bundle_section(writer: Writer, value: BundleSection) -> None:
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


def decode_bundle_section(reader: Reader) -> BundleSection:
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


"""Bundle assembly mode crossing bridge boundaries."""
BundleMode: TypeAlias = (
    Literal["preserveModules"] | Literal["singleFile"] | Literal["chunked"]
)


def encode_bundle_mode(writer: Writer, value: BundleMode) -> None:
    if value == "preserveModules":
        writer.write_unsigned(0)
    elif value == "singleFile":
        writer.write_unsigned(1)
    elif value == "chunked":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_bundle_mode(reader: Reader) -> BundleMode:
    variant = reader.read_number()

    if variant == 0:
        return "preserveModules"
    elif variant == 1:
        return "singleFile"
    elif variant == 2:
        return "chunked"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class BundleFile:
    """One derived bundle file crossing bridge boundaries."""

    """The bundle section this file belongs to."""
    section: BundleSection
    """The output URI."""
    uri: str
    """The emitted file type."""
    file_type: FileType
    """The output content identity."""
    content: ContentId
    """The related source URI when one exists."""
    source: str | None


def encode_bundle_file(writer: Writer, value: BundleFile) -> None:
    encode_bundle_section(writer, value.section)
    writer.write_string(value.uri)
    encode_file_type(writer, value.file_type)
    destack._generated.source.file.encode_content_id(writer, value.content)
    if value.source is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.source)


def decode_bundle_file(reader: Reader) -> BundleFile:
    field_0 = decode_bundle_section(reader)
    field_1 = reader.read_string()
    field_2 = decode_file_type(reader)
    field_3 = destack._generated.source.file.decode_content_id(reader)
    field_4 = reader.read_option(lambda: reader.read_string())

    return BundleFile(
        section=field_0,
        uri=field_1,
        file_type=field_2,
        content=field_3,
        source=field_4,
    )


@dataclass(frozen=True, slots=True)
class Bundle:
    """One linked file graph crossing bridge boundaries."""

    """The emitted artifact family."""
    emit: EmitFormat
    """The target-level assembly mode."""
    mode: BundleMode
    """The files in this bundle."""
    files: Sequence[BundleFile]


def encode_bundle(writer: Writer, value: Bundle) -> None:
    encode_emit_format(writer, value.emit)
    encode_bundle_mode(writer, value.mode)
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        encode_bundle_file(writer, item_0)


def decode_bundle(reader: Reader) -> Bundle:
    field_0 = decode_emit_format(reader)
    field_1 = decode_bundle_mode(reader)
    field_2 = [decode_bundle_file(reader) for _ in range(reader.read_number())]

    return Bundle(
        emit=field_0,
        mode=field_1,
        files=field_2,
    )


"""Preferred program execution format crossing bridge boundaries."""
ProgramFormat: TypeAlias = Literal["vm"] | Literal["native"]


def encode_program_format(writer: Writer, value: ProgramFormat) -> None:
    if value == "vm":
        writer.write_unsigned(0)
    elif value == "native":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_program_format(reader: Reader) -> ProgramFormat:
    variant = reader.read_number()

    if variant == 0:
        return "vm"
    elif variant == 1:
        return "native"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class ProgramHeader:
    """Durable program header crossing bridge boundaries."""

    """Pointer byte width required by this program."""
    pointer_bytes: int


def encode_program_header(writer: Writer, value: ProgramHeader) -> None:
    writer.write_unsigned(value.pointer_bytes)


def decode_program_header(reader: Reader) -> ProgramHeader:
    field_0 = reader.read_number()

    return ProgramHeader(
        pointer_bytes=field_0,
    )


@dataclass(frozen=True, slots=True)
class Program:
    """Durable program crossing bridge boundaries."""

    """The program identity and compatibility header."""
    header: ProgramHeader
    """The preferred execution format."""
    format: ProgramFormat
    """Content blobs referenced by the executable payload."""
    contents: Sequence[ContentId]


def encode_program(writer: Writer, value: Program) -> None:
    encode_program_header(writer, value.header)
    encode_program_format(writer, value.format)
    writer.write_unsigned(len(value.contents))
    for item_0 in value.contents:
        destack._generated.source.file.encode_content_id(writer, item_0)


def decode_program(reader: Reader) -> Program:
    field_0 = decode_program_header(reader)
    field_1 = decode_program_format(reader)
    field_2 = [
        destack._generated.source.file.decode_content_id(reader)
        for _ in range(reader.read_number())
    ]

    return Program(
        header=field_0,
        format=field_1,
        contents=field_2,
    )


"""Semantic runtime contract crossing bridge boundaries."""
Runtime: TypeAlias = Literal["destack"] | Literal["js"]


def encode_runtime(writer: Writer, value: Runtime) -> None:
    if value == "destack":
        writer.write_unsigned(0)
    elif value == "js":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_runtime(reader: Reader) -> Runtime:
    variant = reader.read_number()

    if variant == 0:
        return "destack"
    elif variant == 1:
        return "js"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


"""Host environment crossing bridge boundaries."""
Host: TypeAlias = (
    Literal["native"]
    | Literal["browser"]
    | Literal["wasi"]
    | Literal["emscripten"]
    | Literal["freestanding"]
)


def encode_host(writer: Writer, value: Host) -> None:
    if value == "native":
        writer.write_unsigned(0)
    elif value == "browser":
        writer.write_unsigned(1)
    elif value == "wasi":
        writer.write_unsigned(2)
    elif value == "emscripten":
        writer.write_unsigned(3)
    elif value == "freestanding":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_host(reader: Reader) -> Host:
    variant = reader.read_number()

    if variant == 0:
        return "native"
    elif variant == 1:
        return "browser"
    elif variant == 2:
        return "wasi"
    elif variant == 3:
        return "emscripten"
    elif variant == 4:
        return "freestanding"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class ProductTarget:
    """One linked product target crossing bridge boundaries."""

    """The configured product target name."""
    name: str
    """The repository target assembled into this product."""
    target: TargetId
    """The runtime contract this target expects."""
    runtime: Runtime
    """The host environment this target expects."""
    host: Host
    """The platform this target expects."""
    platform: str
    """Whether this product target includes its toolchain build payload."""
    includes_build: bool
    """Whether this product target includes its linked bundle."""
    includes_bundle: bool
    """Whether this product target includes its executable program."""
    includes_program: bool


def encode_product_target(writer: Writer, value: ProductTarget) -> None:
    writer.write_string(value.name)
    destack._generated.source.target.encode_target_id(writer, value.target)
    encode_runtime(writer, value.runtime)
    encode_host(writer, value.host)
    writer.write_string(value.platform)
    writer.write_bool(value.includes_build)
    writer.write_bool(value.includes_bundle)
    writer.write_bool(value.includes_program)


def decode_product_target(reader: Reader) -> ProductTarget:
    field_0 = reader.read_string()
    field_1 = destack._generated.source.target.decode_target_id(reader)
    field_2 = decode_runtime(reader)
    field_3 = decode_host(reader)
    field_4 = reader.read_string()
    field_5 = reader.read_bool()
    field_6 = reader.read_bool()
    field_7 = reader.read_bool()

    return ProductTarget(
        name=field_0,
        target=field_1,
        runtime=field_2,
        host=field_3,
        platform=field_4,
        includes_build=field_5,
        includes_bundle=field_6,
        includes_program=field_7,
    )


@dataclass(frozen=True, slots=True)
class Product:
    """One linked product crossing bridge boundaries."""

    """The configured product name."""
    name: str
    """The linked targets in deterministic order."""
    targets: Sequence[ProductTarget]


def encode_product(writer: Writer, value: Product) -> None:
    writer.write_string(value.name)
    writer.write_unsigned(len(value.targets))
    for item_0 in value.targets:
        encode_product_target(writer, item_0)


def decode_product(reader: Reader) -> Product:
    field_0 = reader.read_string()
    field_1 = [decode_product_target(reader) for _ in range(reader.read_number())]

    return Product(
        name=field_0,
        targets=field_1,
    )


__all__ = [
    "BuildProfile",
    "encode_build_profile",
    "decode_build_profile",
    "BuildLinkage",
    "encode_build_linkage",
    "decode_build_linkage",
    "EmitFormat",
    "encode_emit_format",
    "decode_emit_format",
    "FileType",
    "encode_file_type",
    "decode_file_type",
    "SourceMapSource",
    "encode_source_map_source",
    "decode_source_map_source",
    "SourceMap",
    "encode_source_map",
    "decode_source_map",
    "Declaration",
    "encode_declaration",
    "decode_declaration",
    "ScriptLanguage",
    "encode_script_language",
    "decode_script_language",
    "Script",
    "encode_script",
    "decode_script",
    "ObjectFormat",
    "encode_object_format",
    "decode_object_format",
    "Object",
    "encode_object",
    "decode_object",
    "Asset",
    "encode_asset",
    "decode_asset",
    "Build",
    "encode_build",
    "decode_build",
    "BundleSection",
    "encode_bundle_section",
    "decode_bundle_section",
    "BundleMode",
    "encode_bundle_mode",
    "decode_bundle_mode",
    "BundleFile",
    "encode_bundle_file",
    "decode_bundle_file",
    "Bundle",
    "encode_bundle",
    "decode_bundle",
    "ProgramFormat",
    "encode_program_format",
    "decode_program_format",
    "ProgramHeader",
    "encode_program_header",
    "decode_program_header",
    "Program",
    "encode_program",
    "decode_program",
    "Runtime",
    "encode_runtime",
    "decode_runtime",
    "Host",
    "encode_host",
    "decode_host",
    "ProductTarget",
    "encode_product_target",
    "decode_product_target",
    "Product",
    "encode_product",
    "decode_product",
]
