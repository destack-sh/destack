# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.target.output

"""Module format for emitted JavaScript output."""
JsModuleFormat: typing.TypeAlias = (
    typing.Literal["es2015"]
    | typing.Literal["es2020"]
    | typing.Literal["es2022"]
    | typing.Literal["esNext"]
)

def encode_js_module_format(writer: BinaryWriter, value: JsModuleFormat) -> None: ...
def decode_js_module_format(reader: BinaryReader) -> JsModuleFormat: ...
def to_json_js_module_format(value: JsModuleFormat) -> Json: ...
def from_json_js_module_format(value: Json) -> JsModuleFormat: ...

"""ECMAScript target for emitted JavaScript output."""
EsTarget: typing.TypeAlias = (
    typing.Literal["es5"]
    | typing.Literal["es2015"]
    | typing.Literal["es2016"]
    | typing.Literal["es2017"]
    | typing.Literal["es2018"]
    | typing.Literal["es2019"]
    | typing.Literal["es2020"]
    | typing.Literal["es2021"]
    | typing.Literal["es2022"]
    | typing.Literal["es2023"]
    | typing.Literal["es2024"]
    | typing.Literal["esNext"]
)

def encode_es_target(writer: BinaryWriter, value: EsTarget) -> None: ...
def decode_es_target(reader: BinaryReader) -> EsTarget: ...
def to_json_es_target(value: EsTarget) -> Json: ...
def from_json_es_target(value: Json) -> EsTarget: ...

@dataclass(frozen=True, slots=True)
class TargetJsOptions:
    """JavaScript output configuration."""

    # javaScript module format
    module: JsModuleFormat
    # ECMAScript target version
    target: EsTarget
    # javaScript output topology
    mode: JsOutputMode
    # whether to preserve one emitted module file per reachable module
    preserve_modules: bool
    # root directory for preserved module paths
    preserve_modules_root: str | None
    # manual chunk assignments keyed by chunk name
    manual_chunks: Mapping[str, Sequence[str]]
    # whether to only honor explicit manual chunk declarations
    only_explicit_manual_chunks: bool
    # dependency and resolution options
    dependencies: JsDependencyOptions
    # asset handling options
    assets: JsAssetOptions
    # output configuration for assembled products
    output: JsOutputOptions
    # minification options
    minify: JsMinifyOptions

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetJsOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TargetJsOptions: ...

def encode_target_js_options(writer: BinaryWriter, value: TargetJsOptions) -> None: ...
def decode_target_js_options(reader: BinaryReader) -> TargetJsOptions: ...
def to_json_target_js_options(value: TargetJsOptions) -> Json: ...
def from_json_target_js_options(value: Json) -> TargetJsOptions: ...

"""Output topology for one JavaScript target."""
JsOutputMode: typing.TypeAlias = (
    typing.Literal["singleFile"]
    | typing.Literal["preserveModules"]
    | typing.Literal["chunked"]
)

def encode_js_output_mode(writer: BinaryWriter, value: JsOutputMode) -> None: ...
def decode_js_output_mode(reader: BinaryReader) -> JsOutputMode: ...
def to_json_js_output_mode(value: JsOutputMode) -> Json: ...
def from_json_js_output_mode(value: Json) -> JsOutputMode: ...

@dataclass(frozen=True, slots=True)
class JsDependencyOptions:
    """JavaScript dependency options."""

    # module specifiers to leave external
    external: Sequence[str]
    # module specifiers that must remain external
    never_bundle: Sequence[str]
    # module specifiers that must always be bundled
    always_bundle: Sequence[str]
    # module specifiers that are the only allowed bundle inputs
    only_bundle: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> JsDependencyOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> JsDependencyOptions: ...

def encode_js_dependency_options(
    writer: BinaryWriter, value: JsDependencyOptions
) -> None: ...
def decode_js_dependency_options(reader: BinaryReader) -> JsDependencyOptions: ...
def to_json_js_dependency_options(value: JsDependencyOptions) -> Json: ...
def from_json_js_dependency_options(value: Json) -> JsDependencyOptions: ...

@dataclass(frozen=True, slots=True)
class JsAssetOptions:
    """JavaScript asset handling options."""

    # asset handling mode for referenced assets
    mode: JsAssetMode
    # inline asset payloads smaller than this many bytes
    inline_limit: int | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> JsAssetOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> JsAssetOptions: ...

def encode_js_asset_options(writer: BinaryWriter, value: JsAssetOptions) -> None: ...
def decode_js_asset_options(reader: BinaryReader) -> JsAssetOptions: ...
def to_json_js_asset_options(value: JsAssetOptions) -> Json: ...
def from_json_js_asset_options(value: Json) -> JsAssetOptions: ...

"""Asset handling policy for one JavaScript target."""
JsAssetMode: typing.TypeAlias = (
    typing.Literal["emit"] | typing.Literal["inline"] | typing.Literal["reference"]
)

def encode_js_asset_mode(writer: BinaryWriter, value: JsAssetMode) -> None: ...
def decode_js_asset_mode(reader: BinaryReader) -> JsAssetMode: ...
def to_json_js_asset_mode(value: JsAssetMode) -> Json: ...
def from_json_js_asset_mode(value: Json) -> JsAssetMode: ...

@dataclass(frozen=True, slots=True)
class JsOutputOptions:
    """JavaScript output options."""

    # format for assembled JavaScript outputs
    format: JsOutputFormat | None
    # global name for IIFE bundles
    name: str | None
    # output naming template for entry chunks
    entry_file_names: str | None
    # output naming template for shared chunks
    chunk_file_names: str | None
    # output naming template for assets
    asset_file_names: str | None
    # public path prefix for runtime asset resolution
    public_path: str | None
    # whether to emit one build manifest
    manifest: bool
    # legal comment handling policy
    legal_comments: JsLegalComment
    # banner text to prepend to each emitted bundle
    banner: str | None
    # footer text to append to each emitted bundle
    footer: str | None
    # generated code controls for final output rendering
    generated_code: JsGeneratedCodeOptions | None
    # source map emission mode for JavaScript output
    source_map: destack._generated.repository.config.target.output.SourceMapMode | None
    # whether to omit source contents from source maps
    source_map_exclude_sources: bool
    # whether to include debug ids in source maps
    source_map_debug_ids: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> JsOutputOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> JsOutputOptions: ...

def encode_js_output_options(writer: BinaryWriter, value: JsOutputOptions) -> None: ...
def decode_js_output_options(reader: BinaryReader) -> JsOutputOptions: ...
def to_json_js_output_options(value: JsOutputOptions) -> Json: ...
def from_json_js_output_options(value: Json) -> JsOutputOptions: ...

"""Format for assembled JavaScript outputs."""
JsOutputFormat: typing.TypeAlias = typing.Literal["esm"] | typing.Literal["iife"]

def encode_js_output_format(writer: BinaryWriter, value: JsOutputFormat) -> None: ...
def decode_js_output_format(reader: BinaryReader) -> JsOutputFormat: ...
def to_json_js_output_format(value: JsOutputFormat) -> Json: ...
def from_json_js_output_format(value: Json) -> JsOutputFormat: ...

"""Legal comment handling for assembled outputs."""
JsLegalComment: typing.TypeAlias = (
    typing.Literal["inline"] | typing.Literal["endOfFile"] | typing.Literal["none"]
)

def encode_js_legal_comment(writer: BinaryWriter, value: JsLegalComment) -> None: ...
def decode_js_legal_comment(reader: BinaryReader) -> JsLegalComment: ...
def to_json_js_legal_comment(value: JsLegalComment) -> Json: ...
def from_json_js_legal_comment(value: Json) -> JsLegalComment: ...

@dataclass(frozen=True, slots=True)
class JsGeneratedCodeOptions:
    """Generated code controls for one output."""

    # whether to emit object shorthand properties
    object_shorthand: bool | None
    # whether to preserve reserved names as properties
    reserved_names_as_props: bool | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> JsGeneratedCodeOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> JsGeneratedCodeOptions: ...

def encode_js_generated_code_options(
    writer: BinaryWriter, value: JsGeneratedCodeOptions
) -> None: ...
def decode_js_generated_code_options(
    reader: BinaryReader,
) -> JsGeneratedCodeOptions: ...
def to_json_js_generated_code_options(value: JsGeneratedCodeOptions) -> Json: ...
def from_json_js_generated_code_options(value: Json) -> JsGeneratedCodeOptions: ...

@dataclass(frozen=True, slots=True)
class JsMinifyOptions:
    """JavaScript minification options."""

    # whether to minify final bundled output
    enabled: bool
    # whether to minify syntax forms
    syntax: bool
    # whether to minify whitespace
    whitespace: bool
    # whether to minify identifiers
    identifiers: bool
    # whether to preserve function and class names
    keep_names: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> JsMinifyOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> JsMinifyOptions: ...

def encode_js_minify_options(writer: BinaryWriter, value: JsMinifyOptions) -> None: ...
def decode_js_minify_options(reader: BinaryReader) -> JsMinifyOptions: ...
def to_json_js_minify_options(value: JsMinifyOptions) -> Json: ...
def from_json_js_minify_options(value: Json) -> JsMinifyOptions: ...

__all__ = [
    "JsModuleFormat",
    "encode_js_module_format",
    "decode_js_module_format",
    "to_json_js_module_format",
    "from_json_js_module_format",
    "EsTarget",
    "encode_es_target",
    "decode_es_target",
    "to_json_es_target",
    "from_json_es_target",
    "TargetJsOptions",
    "encode_target_js_options",
    "decode_target_js_options",
    "to_json_target_js_options",
    "from_json_target_js_options",
    "JsOutputMode",
    "encode_js_output_mode",
    "decode_js_output_mode",
    "to_json_js_output_mode",
    "from_json_js_output_mode",
    "JsDependencyOptions",
    "encode_js_dependency_options",
    "decode_js_dependency_options",
    "to_json_js_dependency_options",
    "from_json_js_dependency_options",
    "JsAssetOptions",
    "encode_js_asset_options",
    "decode_js_asset_options",
    "to_json_js_asset_options",
    "from_json_js_asset_options",
    "JsAssetMode",
    "encode_js_asset_mode",
    "decode_js_asset_mode",
    "to_json_js_asset_mode",
    "from_json_js_asset_mode",
    "JsOutputOptions",
    "encode_js_output_options",
    "decode_js_output_options",
    "to_json_js_output_options",
    "from_json_js_output_options",
    "JsOutputFormat",
    "encode_js_output_format",
    "decode_js_output_format",
    "to_json_js_output_format",
    "from_json_js_output_format",
    "JsLegalComment",
    "encode_js_legal_comment",
    "decode_js_legal_comment",
    "to_json_js_legal_comment",
    "from_json_js_legal_comment",
    "JsGeneratedCodeOptions",
    "encode_js_generated_code_options",
    "decode_js_generated_code_options",
    "to_json_js_generated_code_options",
    "from_json_js_generated_code_options",
    "JsMinifyOptions",
    "encode_js_minify_options",
    "decode_js_minify_options",
    "to_json_js_minify_options",
    "from_json_js_minify_options",
]
