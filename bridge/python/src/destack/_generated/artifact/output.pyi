# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

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

def encode_build_profile(writer: Writer, value: BuildProfile) -> None: ...
def decode_build_profile(reader: Reader) -> BuildProfile: ...

"""Build payload linkage crossing bridge boundaries."""
BuildLinkage: TypeAlias = Literal["portable"] | Literal["static"] | Literal["dynamic"]

def encode_build_linkage(writer: Writer, value: BuildLinkage) -> None: ...
def decode_build_linkage(reader: Reader) -> BuildLinkage: ...

"""Emitted artifact family crossing bridge boundaries."""
EmitFormat: TypeAlias = (
    Literal["js"] | Literal["ts"] | Literal["wasm"] | Literal["native"]
)

def encode_emit_format(writer: Writer, value: EmitFormat) -> None: ...
def decode_emit_format(reader: Reader) -> EmitFormat: ...

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

def encode_file_type(writer: Writer, value: FileType) -> None: ...
def decode_file_type(reader: Reader) -> FileType: ...

@dataclass(frozen=True, slots=True)
class SourceMapSource:
    """One emitted or linked source map crossing bridge boundaries."""

    """The mapped source names."""
    name: str
    """The embedded source contents when they exist."""
    content: str | None

def encode_source_map_source(writer: Writer, value: SourceMapSource) -> None: ...
def decode_source_map_source(reader: Reader) -> SourceMapSource: ...

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

def encode_source_map(writer: Writer, value: SourceMap) -> None: ...
def decode_source_map(reader: Reader) -> SourceMap: ...

@dataclass(frozen=True, slots=True)
class Declaration:
    """One emitted declaration crossing bridge boundaries."""

    """The declaration text."""
    text: str

def encode_declaration(writer: Writer, value: Declaration) -> None: ...
def decode_declaration(reader: Reader) -> Declaration: ...

"""Structured script language crossing bridge boundaries."""
ScriptLanguage: TypeAlias = Literal["javaScript"] | Literal["typeScript"]

def encode_script_language(writer: Writer, value: ScriptLanguage) -> None: ...
def decode_script_language(reader: Reader) -> ScriptLanguage: ...

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

def encode_script(writer: Writer, value: Script) -> None: ...
def decode_script(reader: Reader) -> Script: ...

"""Compiled-code object format crossing bridge boundaries."""
ObjectFormat: TypeAlias = Literal["object"] | Literal["wasm"]

def encode_object_format(writer: Writer, value: ObjectFormat) -> None: ...
def decode_object_format(reader: Reader) -> ObjectFormat: ...

@dataclass(frozen=True, slots=True)
class Object:
    """One compiled-code object artifact crossing bridge boundaries."""

    """The compiled-code object format."""
    format: ObjectFormat
    """The encoded object content identity."""
    content: ContentId
    """The source map when one exists."""
    map: SourceMap | None

def encode_object(writer: Writer, value: Object) -> None: ...
def decode_object(reader: Reader) -> Object: ...

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

def encode_asset(writer: Writer, value: Asset) -> None: ...
def decode_asset(reader: Reader) -> Asset: ...

@dataclass(frozen=True, slots=True)
class Build:
    """One target-built toolchain payload crossing bridge boundaries."""

    """The build distribution profile."""
    profile: BuildProfile
    """The build linkage."""
    linkage: BuildLinkage
    """The encoded build content."""
    content: ContentId

def encode_build(writer: Writer, value: Build) -> None: ...
def decode_build(reader: Reader) -> Build: ...

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

def encode_bundle_section(writer: Writer, value: BundleSection) -> None: ...
def decode_bundle_section(reader: Reader) -> BundleSection: ...

"""Bundle assembly mode crossing bridge boundaries."""
BundleMode: TypeAlias = (
    Literal["preserveModules"] | Literal["singleFile"] | Literal["chunked"]
)

def encode_bundle_mode(writer: Writer, value: BundleMode) -> None: ...
def decode_bundle_mode(reader: Reader) -> BundleMode: ...

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

def encode_bundle_file(writer: Writer, value: BundleFile) -> None: ...
def decode_bundle_file(reader: Reader) -> BundleFile: ...

@dataclass(frozen=True, slots=True)
class Bundle:
    """One linked file graph crossing bridge boundaries."""

    """The emitted artifact family."""
    emit: EmitFormat
    """The target-level assembly mode."""
    mode: BundleMode
    """The files in this bundle."""
    files: Sequence[BundleFile]

def encode_bundle(writer: Writer, value: Bundle) -> None: ...
def decode_bundle(reader: Reader) -> Bundle: ...

"""Preferred program execution format crossing bridge boundaries."""
ProgramFormat: TypeAlias = Literal["vm"] | Literal["native"]

def encode_program_format(writer: Writer, value: ProgramFormat) -> None: ...
def decode_program_format(reader: Reader) -> ProgramFormat: ...

@dataclass(frozen=True, slots=True)
class ProgramHeader:
    """Durable program header crossing bridge boundaries."""

    """Pointer byte width required by this program."""
    pointer_bytes: int

def encode_program_header(writer: Writer, value: ProgramHeader) -> None: ...
def decode_program_header(reader: Reader) -> ProgramHeader: ...

@dataclass(frozen=True, slots=True)
class Program:
    """Durable program crossing bridge boundaries."""

    """The program identity and compatibility header."""
    header: ProgramHeader
    """The preferred execution format."""
    format: ProgramFormat
    """Content blobs referenced by the executable payload."""
    contents: Sequence[ContentId]

def encode_program(writer: Writer, value: Program) -> None: ...
def decode_program(reader: Reader) -> Program: ...

"""Semantic runtime contract crossing bridge boundaries."""
Runtime: TypeAlias = Literal["destack"] | Literal["js"]

def encode_runtime(writer: Writer, value: Runtime) -> None: ...
def decode_runtime(reader: Reader) -> Runtime: ...

"""Host environment crossing bridge boundaries."""
Host: TypeAlias = (
    Literal["native"]
    | Literal["browser"]
    | Literal["wasi"]
    | Literal["emscripten"]
    | Literal["freestanding"]
)

def encode_host(writer: Writer, value: Host) -> None: ...
def decode_host(reader: Reader) -> Host: ...

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

def encode_product_target(writer: Writer, value: ProductTarget) -> None: ...
def decode_product_target(reader: Reader) -> ProductTarget: ...

@dataclass(frozen=True, slots=True)
class Product:
    """One linked product crossing bridge boundaries."""

    """The configured product name."""
    name: str
    """The linked targets in deterministic order."""
    targets: Sequence[ProductTarget]

def encode_product(writer: Writer, value: Product) -> None: ...
def decode_product(reader: Reader) -> Product: ...

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
