# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
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
    json_int,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

import destack._generated.repository.config.target.output

"""Module format for emitted JavaScript output."""
JsModuleFormat: typing.TypeAlias = (
    typing.Literal["es2015"]
    | typing.Literal["es2020"]
    | typing.Literal["es2022"]
    | typing.Literal["esNext"]
)


def encode_js_module_format(writer: BinaryWriter, value: JsModuleFormat) -> None:
    """Encode one JsModuleFormat."""
    if value == "es2015":
        writer.write_unsigned(0)
    elif value == "es2020":
        writer.write_unsigned(1)
    elif value == "es2022":
        writer.write_unsigned(2)
    elif value == "esNext":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_js_module_format(reader: BinaryReader) -> JsModuleFormat:
    """Decode one JsModuleFormat."""
    variant = reader.read_number()

    if variant == 0:
        return "es2015"
    elif variant == 1:
        return "es2020"
    elif variant == 2:
        return "es2022"
    elif variant == 3:
        return "esNext"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_js_module_format(value: JsModuleFormat) -> Json:
    """Return one JSON value for one JsModuleFormat."""
    return value


def from_json_js_module_format(value: Json) -> JsModuleFormat:
    """Return one JsModuleFormat from one JSON value."""
    variant = json_string(value)

    if variant == "es2015":
        return "es2015"
    elif variant == "es2020":
        return "es2020"
    elif variant == "es2022":
        return "es2022"
    elif variant == "esNext":
        return "esNext"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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


def encode_es_target(writer: BinaryWriter, value: EsTarget) -> None:
    """Encode one EsTarget."""
    if value == "es5":
        writer.write_unsigned(0)
    elif value == "es2015":
        writer.write_unsigned(1)
    elif value == "es2016":
        writer.write_unsigned(2)
    elif value == "es2017":
        writer.write_unsigned(3)
    elif value == "es2018":
        writer.write_unsigned(4)
    elif value == "es2019":
        writer.write_unsigned(5)
    elif value == "es2020":
        writer.write_unsigned(6)
    elif value == "es2021":
        writer.write_unsigned(7)
    elif value == "es2022":
        writer.write_unsigned(8)
    elif value == "es2023":
        writer.write_unsigned(9)
    elif value == "es2024":
        writer.write_unsigned(10)
    elif value == "esNext":
        writer.write_unsigned(11)
    else:
        raise SerdeError("unknown enum variant")


def decode_es_target(reader: BinaryReader) -> EsTarget:
    """Decode one EsTarget."""
    variant = reader.read_number()

    if variant == 0:
        return "es5"
    elif variant == 1:
        return "es2015"
    elif variant == 2:
        return "es2016"
    elif variant == 3:
        return "es2017"
    elif variant == 4:
        return "es2018"
    elif variant == 5:
        return "es2019"
    elif variant == 6:
        return "es2020"
    elif variant == 7:
        return "es2021"
    elif variant == 8:
        return "es2022"
    elif variant == 9:
        return "es2023"
    elif variant == 10:
        return "es2024"
    elif variant == 11:
        return "esNext"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_es_target(value: EsTarget) -> Json:
    """Return one JSON value for one EsTarget."""
    return value


def from_json_es_target(value: Json) -> EsTarget:
    """Return one EsTarget from one JSON value."""
    variant = json_string(value)

    if variant == "es5":
        return "es5"
    elif variant == "es2015":
        return "es2015"
    elif variant == "es2016":
        return "es2016"
    elif variant == "es2017":
        return "es2017"
    elif variant == "es2018":
        return "es2018"
    elif variant == "es2019":
        return "es2019"
    elif variant == "es2020":
        return "es2020"
    elif variant == "es2021":
        return "es2021"
    elif variant == "es2022":
        return "es2022"
    elif variant == "es2023":
        return "es2023"
    elif variant == "es2024":
        return "es2024"
    elif variant == "esNext":
        return "esNext"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_js_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetJsOptions:
        """Decode one TargetJsOptions."""
        return decode_target_js_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_js_options(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetJsOptions:
        """Return one TargetJsOptions from one JSON value."""
        return from_json_target_js_options(value)


def encode_target_js_options(writer: BinaryWriter, value: TargetJsOptions) -> None:
    """Encode one TargetJsOptions."""
    encode_js_module_format(writer, value.module)
    encode_es_target(writer, value.target)
    encode_js_output_mode(writer, value.mode)
    writer.write_bool(value.preserve_modules)
    if value.preserve_modules_root is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.preserve_modules_root)
    entries_value_manual_chunks_0 = []
    for (
        key_value_manual_chunks_0,
        item_value_manual_chunks_0,
    ) in value.manual_chunks.items():

        def write_key_value_manual_chunks_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_manual_chunks_0)

        key_bytes = nested_bytes(write_key_value_manual_chunks_0)
        entries_value_manual_chunks_0.append(
            (key_value_manual_chunks_0, item_value_manual_chunks_0, key_bytes)
        )
    entries_value_manual_chunks_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_manual_chunks_0))
    for entry_value_manual_chunks_0 in entries_value_manual_chunks_0:
        writer.write_string(entry_value_manual_chunks_0[0])
        writer.write_unsigned(len(entry_value_manual_chunks_0[1]))
        for item_entry_value_manual_chunks_0_1_1 in entry_value_manual_chunks_0[1]:
            writer.write_string(item_entry_value_manual_chunks_0_1_1)
    writer.write_bool(value.only_explicit_manual_chunks)
    encode_js_dependency_options(writer, value.dependencies)
    encode_js_asset_options(writer, value.assets)
    encode_js_output_options(writer, value.output)
    encode_js_minify_options(writer, value.minify)


def decode_target_js_options(reader: BinaryReader) -> TargetJsOptions:
    """Decode one TargetJsOptions."""
    module = decode_js_module_format(reader)
    target = decode_es_target(reader)
    mode = decode_js_output_mode(reader)
    preserve_modules = reader.read_bool()
    preserve_modules_root = reader.read_option(lambda: reader.read_string())
    manual_chunks = {
        reader.read_string(): [
            reader.read_string() for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    only_explicit_manual_chunks = reader.read_bool()
    dependencies = decode_js_dependency_options(reader)
    assets = decode_js_asset_options(reader)
    output = decode_js_output_options(reader)
    minify = decode_js_minify_options(reader)

    return TargetJsOptions(
        module=module,
        target=target,
        mode=mode,
        preserve_modules=preserve_modules,
        preserve_modules_root=preserve_modules_root,
        manual_chunks=manual_chunks,
        only_explicit_manual_chunks=only_explicit_manual_chunks,
        dependencies=dependencies,
        assets=assets,
        output=output,
        minify=minify,
    )


def to_json_target_js_options(value: TargetJsOptions) -> Json:
    """Return one JSON value for one TargetJsOptions."""
    return {
        "module": to_json_js_module_format(value.module),
        "target": to_json_es_target(value.target),
        "mode": to_json_js_output_mode(value.mode),
        "preserveModules": value.preserve_modules,
        **(
            {}
            if value.preserve_modules_root is None
            else {"preserveModulesRoot": value.preserve_modules_root}
        ),
        "manualChunks": {
            key_0: [item_1 for item_1 in item_0]
            for key_0, item_0 in value.manual_chunks.items()
        },
        "onlyExplicitManualChunks": value.only_explicit_manual_chunks,
        "dependencies": to_json_js_dependency_options(value.dependencies),
        "assets": to_json_js_asset_options(value.assets),
        "output": to_json_js_output_options(value.output),
        "minify": to_json_js_minify_options(value.minify),
    }


def from_json_target_js_options(value: Json) -> TargetJsOptions:
    """Return one TargetJsOptions from one JSON value."""
    object_ = json_object(value)

    return TargetJsOptions(
        module=from_json_js_module_format(json_field(object_, "module")),
        target=from_json_es_target(json_field(object_, "target")),
        mode=from_json_js_output_mode(json_field(object_, "mode")),
        preserve_modules=json_bool(json_field(object_, "preserveModules")),
        preserve_modules_root=json_optional(
            object_, "preserveModulesRoot", lambda value: json_string(value)
        ),
        manual_chunks={
            key_0: [json_string(item_1) for item_1 in json_array(item_0)]
            for key_0, item_0 in json_object(
                json_field(object_, "manualChunks")
            ).items()
        },
        only_explicit_manual_chunks=json_bool(
            json_field(object_, "onlyExplicitManualChunks")
        ),
        dependencies=from_json_js_dependency_options(
            json_field(object_, "dependencies")
        ),
        assets=from_json_js_asset_options(json_field(object_, "assets")),
        output=from_json_js_output_options(json_field(object_, "output")),
        minify=from_json_js_minify_options(json_field(object_, "minify")),
    )


"""Output topology for one JavaScript target."""
JsOutputMode: typing.TypeAlias = (
    typing.Literal["singleFile"]
    | typing.Literal["preserveModules"]
    | typing.Literal["chunked"]
)


def encode_js_output_mode(writer: BinaryWriter, value: JsOutputMode) -> None:
    """Encode one JsOutputMode."""
    if value == "singleFile":
        writer.write_unsigned(0)
    elif value == "preserveModules":
        writer.write_unsigned(1)
    elif value == "chunked":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_js_output_mode(reader: BinaryReader) -> JsOutputMode:
    """Decode one JsOutputMode."""
    variant = reader.read_number()

    if variant == 0:
        return "singleFile"
    elif variant == 1:
        return "preserveModules"
    elif variant == 2:
        return "chunked"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_js_output_mode(value: JsOutputMode) -> Json:
    """Return one JSON value for one JsOutputMode."""
    return value


def from_json_js_output_mode(value: Json) -> JsOutputMode:
    """Return one JsOutputMode from one JSON value."""
    variant = json_string(value)

    if variant == "singleFile":
        return "singleFile"
    elif variant == "preserveModules":
        return "preserveModules"
    elif variant == "chunked":
        return "chunked"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_js_dependency_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> JsDependencyOptions:
        """Decode one JsDependencyOptions."""
        return decode_js_dependency_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_js_dependency_options(self)

    @classmethod
    def from_json(cls, value: Json) -> JsDependencyOptions:
        """Return one JsDependencyOptions from one JSON value."""
        return from_json_js_dependency_options(value)


def encode_js_dependency_options(
    writer: BinaryWriter, value: JsDependencyOptions
) -> None:
    """Encode one JsDependencyOptions."""
    writer.write_unsigned(len(value.external))
    for item_value_external_0 in value.external:
        writer.write_string(item_value_external_0)
    writer.write_unsigned(len(value.never_bundle))
    for item_value_never_bundle_0 in value.never_bundle:
        writer.write_string(item_value_never_bundle_0)
    writer.write_unsigned(len(value.always_bundle))
    for item_value_always_bundle_0 in value.always_bundle:
        writer.write_string(item_value_always_bundle_0)
    writer.write_unsigned(len(value.only_bundle))
    for item_value_only_bundle_0 in value.only_bundle:
        writer.write_string(item_value_only_bundle_0)


def decode_js_dependency_options(reader: BinaryReader) -> JsDependencyOptions:
    """Decode one JsDependencyOptions."""
    external = [reader.read_string() for _ in range(reader.read_number())]
    never_bundle = [reader.read_string() for _ in range(reader.read_number())]
    always_bundle = [reader.read_string() for _ in range(reader.read_number())]
    only_bundle = [reader.read_string() for _ in range(reader.read_number())]

    return JsDependencyOptions(
        external=external,
        never_bundle=never_bundle,
        always_bundle=always_bundle,
        only_bundle=only_bundle,
    )


def to_json_js_dependency_options(value: JsDependencyOptions) -> Json:
    """Return one JSON value for one JsDependencyOptions."""
    return {
        "external": [item_0 for item_0 in value.external],
        "neverBundle": [item_0 for item_0 in value.never_bundle],
        "alwaysBundle": [item_0 for item_0 in value.always_bundle],
        "onlyBundle": [item_0 for item_0 in value.only_bundle],
    }


def from_json_js_dependency_options(value: Json) -> JsDependencyOptions:
    """Return one JsDependencyOptions from one JSON value."""
    object_ = json_object(value)

    return JsDependencyOptions(
        external=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "external"))
        ],
        never_bundle=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "neverBundle"))
        ],
        always_bundle=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "alwaysBundle"))
        ],
        only_bundle=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "onlyBundle"))
        ],
    )


@dataclass(frozen=True, slots=True)
class JsAssetOptions:
    """JavaScript asset handling options."""

    # asset handling mode for referenced assets
    mode: JsAssetMode
    # inline asset payloads smaller than this many bytes
    inline_limit: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_js_asset_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> JsAssetOptions:
        """Decode one JsAssetOptions."""
        return decode_js_asset_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_js_asset_options(self)

    @classmethod
    def from_json(cls, value: Json) -> JsAssetOptions:
        """Return one JsAssetOptions from one JSON value."""
        return from_json_js_asset_options(value)


def encode_js_asset_options(writer: BinaryWriter, value: JsAssetOptions) -> None:
    """Encode one JsAssetOptions."""
    encode_js_asset_mode(writer, value.mode)
    if value.inline_limit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.inline_limit)


def decode_js_asset_options(reader: BinaryReader) -> JsAssetOptions:
    """Decode one JsAssetOptions."""
    mode = decode_js_asset_mode(reader)
    inline_limit = reader.read_option(lambda: reader.read_number())

    return JsAssetOptions(
        mode=mode,
        inline_limit=inline_limit,
    )


def to_json_js_asset_options(value: JsAssetOptions) -> Json:
    """Return one JSON value for one JsAssetOptions."""
    return {
        "mode": to_json_js_asset_mode(value.mode),
        **({} if value.inline_limit is None else {"inlineLimit": value.inline_limit}),
    }


def from_json_js_asset_options(value: Json) -> JsAssetOptions:
    """Return one JsAssetOptions from one JSON value."""
    object_ = json_object(value)

    return JsAssetOptions(
        mode=from_json_js_asset_mode(json_field(object_, "mode")),
        inline_limit=json_optional(
            object_, "inlineLimit", lambda value: json_int(value)
        ),
    )


"""Asset handling policy for one JavaScript target."""
JsAssetMode: typing.TypeAlias = (
    typing.Literal["emit"] | typing.Literal["inline"] | typing.Literal["reference"]
)


def encode_js_asset_mode(writer: BinaryWriter, value: JsAssetMode) -> None:
    """Encode one JsAssetMode."""
    if value == "emit":
        writer.write_unsigned(0)
    elif value == "inline":
        writer.write_unsigned(1)
    elif value == "reference":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_js_asset_mode(reader: BinaryReader) -> JsAssetMode:
    """Decode one JsAssetMode."""
    variant = reader.read_number()

    if variant == 0:
        return "emit"
    elif variant == 1:
        return "inline"
    elif variant == 2:
        return "reference"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_js_asset_mode(value: JsAssetMode) -> Json:
    """Return one JSON value for one JsAssetMode."""
    return value


def from_json_js_asset_mode(value: Json) -> JsAssetMode:
    """Return one JsAssetMode from one JSON value."""
    variant = json_string(value)

    if variant == "emit":
        return "emit"
    elif variant == "inline":
        return "inline"
    elif variant == "reference":
        return "reference"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_js_output_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> JsOutputOptions:
        """Decode one JsOutputOptions."""
        return decode_js_output_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_js_output_options(self)

    @classmethod
    def from_json(cls, value: Json) -> JsOutputOptions:
        """Return one JsOutputOptions from one JSON value."""
        return from_json_js_output_options(value)


def encode_js_output_options(writer: BinaryWriter, value: JsOutputOptions) -> None:
    """Encode one JsOutputOptions."""
    if value.format is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_js_output_format(writer, value.format)
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
    if value.entry_file_names is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.entry_file_names)
    if value.chunk_file_names is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.chunk_file_names)
    if value.asset_file_names is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.asset_file_names)
    if value.public_path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.public_path)
    writer.write_bool(value.manifest)
    encode_js_legal_comment(writer, value.legal_comments)
    if value.banner is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.banner)
    if value.footer is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.footer)
    if value.generated_code is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_js_generated_code_options(writer, value.generated_code)
    if value.source_map is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.target.output.encode_source_map_mode(
            writer, value.source_map
        )
    writer.write_bool(value.source_map_exclude_sources)
    writer.write_bool(value.source_map_debug_ids)


def decode_js_output_options(reader: BinaryReader) -> JsOutputOptions:
    """Decode one JsOutputOptions."""
    format = reader.read_option(lambda: decode_js_output_format(reader))
    name = reader.read_option(lambda: reader.read_string())
    entry_file_names = reader.read_option(lambda: reader.read_string())
    chunk_file_names = reader.read_option(lambda: reader.read_string())
    asset_file_names = reader.read_option(lambda: reader.read_string())
    public_path = reader.read_option(lambda: reader.read_string())
    manifest = reader.read_bool()
    legal_comments = decode_js_legal_comment(reader)
    banner = reader.read_option(lambda: reader.read_string())
    footer = reader.read_option(lambda: reader.read_string())
    generated_code = reader.read_option(
        lambda: decode_js_generated_code_options(reader)
    )
    source_map = reader.read_option(
        lambda: (
            destack._generated.repository.config.target.output.decode_source_map_mode(
                reader
            )
        )
    )
    source_map_exclude_sources = reader.read_bool()
    source_map_debug_ids = reader.read_bool()

    return JsOutputOptions(
        format=format,
        name=name,
        entry_file_names=entry_file_names,
        chunk_file_names=chunk_file_names,
        asset_file_names=asset_file_names,
        public_path=public_path,
        manifest=manifest,
        legal_comments=legal_comments,
        banner=banner,
        footer=footer,
        generated_code=generated_code,
        source_map=source_map,
        source_map_exclude_sources=source_map_exclude_sources,
        source_map_debug_ids=source_map_debug_ids,
    )


def to_json_js_output_options(value: JsOutputOptions) -> Json:
    """Return one JSON value for one JsOutputOptions."""
    return {
        **(
            {}
            if value.format is None
            else {"format": to_json_js_output_format(value.format)}
        ),
        **({} if value.name is None else {"name": value.name}),
        **(
            {}
            if value.entry_file_names is None
            else {"entryFileNames": value.entry_file_names}
        ),
        **(
            {}
            if value.chunk_file_names is None
            else {"chunkFileNames": value.chunk_file_names}
        ),
        **(
            {}
            if value.asset_file_names is None
            else {"assetFileNames": value.asset_file_names}
        ),
        **({} if value.public_path is None else {"publicPath": value.public_path}),
        "manifest": value.manifest,
        "legalComments": to_json_js_legal_comment(value.legal_comments),
        **({} if value.banner is None else {"banner": value.banner}),
        **({} if value.footer is None else {"footer": value.footer}),
        **(
            {}
            if value.generated_code is None
            else {
                "generatedCode": to_json_js_generated_code_options(value.generated_code)
            }
        ),
        **(
            {}
            if value.source_map is None
            else {
                "sourceMap": destack._generated.repository.config.target.output.to_json_source_map_mode(
                    value.source_map
                )
            }
        ),
        "sourceMapExcludeSources": value.source_map_exclude_sources,
        "sourceMapDebugIds": value.source_map_debug_ids,
    }


def from_json_js_output_options(value: Json) -> JsOutputOptions:
    """Return one JsOutputOptions from one JSON value."""
    object_ = json_object(value)

    return JsOutputOptions(
        format=json_optional(
            object_, "format", lambda value: from_json_js_output_format(value)
        ),
        name=json_optional(object_, "name", lambda value: json_string(value)),
        entry_file_names=json_optional(
            object_, "entryFileNames", lambda value: json_string(value)
        ),
        chunk_file_names=json_optional(
            object_, "chunkFileNames", lambda value: json_string(value)
        ),
        asset_file_names=json_optional(
            object_, "assetFileNames", lambda value: json_string(value)
        ),
        public_path=json_optional(
            object_, "publicPath", lambda value: json_string(value)
        ),
        manifest=json_bool(json_field(object_, "manifest")),
        legal_comments=from_json_js_legal_comment(json_field(object_, "legalComments")),
        banner=json_optional(object_, "banner", lambda value: json_string(value)),
        footer=json_optional(object_, "footer", lambda value: json_string(value)),
        generated_code=json_optional(
            object_,
            "generatedCode",
            lambda value: from_json_js_generated_code_options(value),
        ),
        source_map=json_optional(
            object_,
            "sourceMap",
            lambda value: (
                destack._generated.repository.config.target.output.from_json_source_map_mode(
                    value
                )
            ),
        ),
        source_map_exclude_sources=json_bool(
            json_field(object_, "sourceMapExcludeSources")
        ),
        source_map_debug_ids=json_bool(json_field(object_, "sourceMapDebugIds")),
    )


"""Format for assembled JavaScript outputs."""
JsOutputFormat: typing.TypeAlias = typing.Literal["esm"] | typing.Literal["iife"]


def encode_js_output_format(writer: BinaryWriter, value: JsOutputFormat) -> None:
    """Encode one JsOutputFormat."""
    if value == "esm":
        writer.write_unsigned(0)
    elif value == "iife":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_js_output_format(reader: BinaryReader) -> JsOutputFormat:
    """Decode one JsOutputFormat."""
    variant = reader.read_number()

    if variant == 0:
        return "esm"
    elif variant == 1:
        return "iife"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_js_output_format(value: JsOutputFormat) -> Json:
    """Return one JSON value for one JsOutputFormat."""
    return value


def from_json_js_output_format(value: Json) -> JsOutputFormat:
    """Return one JsOutputFormat from one JSON value."""
    variant = json_string(value)

    if variant == "esm":
        return "esm"
    elif variant == "iife":
        return "iife"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Legal comment handling for assembled outputs."""
JsLegalComment: typing.TypeAlias = (
    typing.Literal["inline"] | typing.Literal["endOfFile"] | typing.Literal["none"]
)


def encode_js_legal_comment(writer: BinaryWriter, value: JsLegalComment) -> None:
    """Encode one JsLegalComment."""
    if value == "inline":
        writer.write_unsigned(0)
    elif value == "endOfFile":
        writer.write_unsigned(1)
    elif value == "none":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_js_legal_comment(reader: BinaryReader) -> JsLegalComment:
    """Decode one JsLegalComment."""
    variant = reader.read_number()

    if variant == 0:
        return "inline"
    elif variant == 1:
        return "endOfFile"
    elif variant == 2:
        return "none"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_js_legal_comment(value: JsLegalComment) -> Json:
    """Return one JSON value for one JsLegalComment."""
    return value


def from_json_js_legal_comment(value: Json) -> JsLegalComment:
    """Return one JsLegalComment from one JSON value."""
    variant = json_string(value)

    if variant == "inline":
        return "inline"
    elif variant == "endOfFile":
        return "endOfFile"
    elif variant == "none":
        return "none"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class JsGeneratedCodeOptions:
    """Generated code controls for one output."""

    # whether to emit object shorthand properties
    object_shorthand: bool | None
    # whether to preserve reserved names as properties
    reserved_names_as_props: bool | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_js_generated_code_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> JsGeneratedCodeOptions:
        """Decode one JsGeneratedCodeOptions."""
        return decode_js_generated_code_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_js_generated_code_options(self)

    @classmethod
    def from_json(cls, value: Json) -> JsGeneratedCodeOptions:
        """Return one JsGeneratedCodeOptions from one JSON value."""
        return from_json_js_generated_code_options(value)


def encode_js_generated_code_options(
    writer: BinaryWriter, value: JsGeneratedCodeOptions
) -> None:
    """Encode one JsGeneratedCodeOptions."""
    if value.object_shorthand is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_bool(value.object_shorthand)
    if value.reserved_names_as_props is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_bool(value.reserved_names_as_props)


def decode_js_generated_code_options(reader: BinaryReader) -> JsGeneratedCodeOptions:
    """Decode one JsGeneratedCodeOptions."""
    object_shorthand = reader.read_option(lambda: reader.read_bool())
    reserved_names_as_props = reader.read_option(lambda: reader.read_bool())

    return JsGeneratedCodeOptions(
        object_shorthand=object_shorthand,
        reserved_names_as_props=reserved_names_as_props,
    )


def to_json_js_generated_code_options(value: JsGeneratedCodeOptions) -> Json:
    """Return one JSON value for one JsGeneratedCodeOptions."""
    return {
        **(
            {}
            if value.object_shorthand is None
            else {"objectShorthand": value.object_shorthand}
        ),
        **(
            {}
            if value.reserved_names_as_props is None
            else {"reservedNamesAsProps": value.reserved_names_as_props}
        ),
    }


def from_json_js_generated_code_options(value: Json) -> JsGeneratedCodeOptions:
    """Return one JsGeneratedCodeOptions from one JSON value."""
    object_ = json_object(value)

    return JsGeneratedCodeOptions(
        object_shorthand=json_optional(
            object_, "objectShorthand", lambda value: json_bool(value)
        ),
        reserved_names_as_props=json_optional(
            object_, "reservedNamesAsProps", lambda value: json_bool(value)
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_js_minify_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> JsMinifyOptions:
        """Decode one JsMinifyOptions."""
        return decode_js_minify_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_js_minify_options(self)

    @classmethod
    def from_json(cls, value: Json) -> JsMinifyOptions:
        """Return one JsMinifyOptions from one JSON value."""
        return from_json_js_minify_options(value)


def encode_js_minify_options(writer: BinaryWriter, value: JsMinifyOptions) -> None:
    """Encode one JsMinifyOptions."""
    writer.write_bool(value.enabled)
    writer.write_bool(value.syntax)
    writer.write_bool(value.whitespace)
    writer.write_bool(value.identifiers)
    writer.write_bool(value.keep_names)


def decode_js_minify_options(reader: BinaryReader) -> JsMinifyOptions:
    """Decode one JsMinifyOptions."""
    enabled = reader.read_bool()
    syntax = reader.read_bool()
    whitespace = reader.read_bool()
    identifiers = reader.read_bool()
    keep_names = reader.read_bool()

    return JsMinifyOptions(
        enabled=enabled,
        syntax=syntax,
        whitespace=whitespace,
        identifiers=identifiers,
        keep_names=keep_names,
    )


def to_json_js_minify_options(value: JsMinifyOptions) -> Json:
    """Return one JSON value for one JsMinifyOptions."""
    return {
        "enabled": value.enabled,
        "syntax": value.syntax,
        "whitespace": value.whitespace,
        "identifiers": value.identifiers,
        "keepNames": value.keep_names,
    }


def from_json_js_minify_options(value: Json) -> JsMinifyOptions:
    """Return one JsMinifyOptions from one JSON value."""
    object_ = json_object(value)

    return JsMinifyOptions(
        enabled=json_bool(json_field(object_, "enabled")),
        syntax=json_bool(json_field(object_, "syntax")),
        whitespace=json_bool(json_field(object_, "whitespace")),
        identifiers=json_bool(json_field(object_, "identifiers")),
        keep_names=json_bool(json_field(object_, "keepNames")),
    )


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
