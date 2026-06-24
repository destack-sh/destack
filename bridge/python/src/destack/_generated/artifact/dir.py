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
    json_object,
    json_string,
)

import destack._generated.artifact.core.dependency
import destack._generated.dir.source.token
import destack._generated.dir.symbol.scope
import destack._generated.dir.table.annotation
import destack._generated.dir.table.binding
import destack._generated.dir.table.capture
import destack._generated.dir.table.coercion
import destack._generated.dir.table.definition
import destack._generated.dir.table.export
import destack._generated.dir.table.generic
import destack._generated.dir.table.global_
import destack._generated.dir.table.guard
import destack._generated.dir.table.import_
import destack._generated.dir.table.layout
import destack._generated.dir.table.macro
import destack._generated.dir.table.module
import destack._generated.dir.table.reference
import destack._generated.dir.table.resolution
import destack._generated.dir.table.static
import destack._generated.dir.table.type
import destack._generated.dir.tree.node
import destack._generated.dir.tree.patch
import destack._generated.dir.tree.tree
import destack._generated.source.file.model.component
import destack._generated.source.file.model.file
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class DirParsedFile:
    """Parsed roots and side data for one physical file in a canonical module."""

    # the source file id
    file_id: destack._generated.source.file.model.file.FileId
    # the condition aliases attached to this file
    aliases: Sequence[str]
    # the top-level expressions parsed from this file
    roots: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the source file tokens
    tokens: Sequence[destack._generated.dir.source.token.TokenRecord]
    # the source file side tokens
    side_tokens: Sequence[destack._generated.dir.source.token.TokenRecord]
    # stable anchor expression for diagnostics in this file
    anchor_expression: destack._generated.dir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_parsed_file(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirParsedFile:
        """Decode one DirParsedFile."""
        return decode_dir_parsed_file(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_parsed_file(self)

    @classmethod
    def from_json(cls, value: Json) -> DirParsedFile:
        """Return one DirParsedFile from one JSON value."""
        return from_json_dir_parsed_file(value)


def encode_dir_parsed_file(writer: BinaryWriter, value: DirParsedFile) -> None:
    """Encode one DirParsedFile."""
    destack._generated.source.file.model.file.encode_file_id(writer, value.file_id)
    writer.write_unsigned(len(value.aliases))
    for item_value_aliases_0 in value.aliases:
        writer.write_string(item_value_aliases_0)
    writer.write_unsigned(len(value.roots))
    for item_value_roots_0 in value.roots:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_roots_0
        )
    writer.write_unsigned(len(value.tokens))
    for item_value_tokens_0 in value.tokens:
        destack._generated.dir.source.token.encode_token_record(
            writer, item_value_tokens_0
        )
    writer.write_unsigned(len(value.side_tokens))
    for item_value_side_tokens_0 in value.side_tokens:
        destack._generated.dir.source.token.encode_token_record(
            writer, item_value_side_tokens_0
        )
    destack._generated.dir.tree.node.encode_local_node_id(
        writer, value.anchor_expression
    )


def decode_dir_parsed_file(reader: BinaryReader) -> DirParsedFile:
    """Decode one DirParsedFile."""
    file_id = destack._generated.source.file.model.file.decode_file_id(reader)
    aliases = [reader.read_string() for _ in range(reader.read_number())]
    roots = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    tokens = [
        destack._generated.dir.source.token.decode_token_record(reader)
        for _ in range(reader.read_number())
    ]
    side_tokens = [
        destack._generated.dir.source.token.decode_token_record(reader)
        for _ in range(reader.read_number())
    ]
    anchor_expression = destack._generated.dir.tree.node.decode_local_node_id(reader)

    return DirParsedFile(
        file_id=file_id,
        aliases=aliases,
        roots=roots,
        tokens=tokens,
        side_tokens=side_tokens,
        anchor_expression=anchor_expression,
    )


def to_json_dir_parsed_file(value: DirParsedFile) -> Json:
    """Return one JSON value for one DirParsedFile."""
    return {
        "fileId": destack._generated.source.file.model.file.to_json_file_id(
            value.file_id
        ),
        "aliases": [item_0 for item_0 in value.aliases],
        "roots": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.roots
        ],
        "tokens": [
            destack._generated.dir.source.token.to_json_token_record(item_0)
            for item_0 in value.tokens
        ],
        "sideTokens": [
            destack._generated.dir.source.token.to_json_token_record(item_0)
            for item_0 in value.side_tokens
        ],
        "anchorExpression": destack._generated.dir.tree.node.to_json_local_node_id(
            value.anchor_expression
        ),
    }


def from_json_dir_parsed_file(value: Json) -> DirParsedFile:
    """Return one DirParsedFile from one JSON value."""
    object_ = json_object(value)

    return DirParsedFile(
        file_id=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "fileId")
        ),
        aliases=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "aliases"))
        ],
        roots=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "roots"))
        ],
        tokens=[
            destack._generated.dir.source.token.from_json_token_record(item_0)
            for item_0 in json_array(json_field(object_, "tokens"))
        ],
        side_tokens=[
            destack._generated.dir.source.token.from_json_token_record(item_0)
            for item_0 in json_array(json_field(object_, "sideTokens"))
        ],
        anchor_expression=destack._generated.dir.tree.node.from_json_local_node_id(
            json_field(object_, "anchorExpression")
        ),
    )


@dataclass(frozen=True, slots=True)
class DirCheckedComponentEntry:
    """Checked DIR entry for one module in a checked component."""

    # the checked module id
    module: destack._generated.source.file.model.module.ModuleId
    # the stable fingerprint of this module's checked output
    fingerprint: (
        destack._generated.artifact.core.dependency.ArtifactProjectionFingerprint
    )
    # the checked side tables for this module
    checked: DirCheckedModule

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_checked_component_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirCheckedComponentEntry:
        """Decode one DirCheckedComponentEntry."""
        return decode_dir_checked_component_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_checked_component_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> DirCheckedComponentEntry:
        """Return one DirCheckedComponentEntry from one JSON value."""
        return from_json_dir_checked_component_entry(value)


def encode_dir_checked_component_entry(
    writer: BinaryWriter, value: DirCheckedComponentEntry
) -> None:
    """Encode one DirCheckedComponentEntry."""
    destack._generated.source.file.model.module.encode_module_id(writer, value.module)
    destack._generated.artifact.core.dependency.encode_artifact_projection_fingerprint(
        writer, value.fingerprint
    )
    encode_dir_checked_module(writer, value.checked)


def decode_dir_checked_component_entry(
    reader: BinaryReader,
) -> DirCheckedComponentEntry:
    """Decode one DirCheckedComponentEntry."""
    module = destack._generated.source.file.model.module.decode_module_id(reader)
    fingerprint = destack._generated.artifact.core.dependency.decode_artifact_projection_fingerprint(
        reader
    )
    checked = decode_dir_checked_module(reader)

    return DirCheckedComponentEntry(
        module=module,
        fingerprint=fingerprint,
        checked=checked,
    )


def to_json_dir_checked_component_entry(value: DirCheckedComponentEntry) -> Json:
    """Return one JSON value for one DirCheckedComponentEntry."""
    return {
        "module": destack._generated.source.file.model.module.to_json_module_id(
            value.module
        ),
        "fingerprint": destack._generated.artifact.core.dependency.to_json_artifact_projection_fingerprint(
            value.fingerprint
        ),
        "checked": to_json_dir_checked_module(value.checked),
    }


def from_json_dir_checked_component_entry(value: Json) -> DirCheckedComponentEntry:
    """Return one DirCheckedComponentEntry from one JSON value."""
    object_ = json_object(value)

    return DirCheckedComponentEntry(
        module=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "module")
        ),
        fingerprint=destack._generated.artifact.core.dependency.from_json_artifact_projection_fingerprint(
            json_field(object_, "fingerprint")
        ),
        checked=from_json_dir_checked_module(json_field(object_, "checked")),
    )


@dataclass(frozen=True, slots=True)
class DirCheckedModule:
    """Type-checking segment for one profile-scoped module."""

    # new annotation invocations
    annotations: destack._generated.dir.table.annotation.AnnotationSegment
    # new types
    types: destack._generated.dir.table.type.TypeSegment
    # new static values
    statics: destack._generated.dir.table.static.StaticSegment
    # new resolutions
    resolutions: destack._generated.dir.table.resolution.ResolutionSegment
    # new generic slots and instances
    generics: destack._generated.dir.table.generic.GenericSegment
    # new declaration definitions
    definitions: destack._generated.dir.table.definition.DefinitionSegment
    # new implicit coercions
    coercions: destack._generated.dir.table.coercion.CoercionSegment
    # new layouts
    layouts: destack._generated.dir.table.layout.LayoutSegment
    # new captures
    captures: destack._generated.dir.table.capture.CaptureSegment

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_checked_module(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirCheckedModule:
        """Decode one DirCheckedModule."""
        return decode_dir_checked_module(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_checked_module(self)

    @classmethod
    def from_json(cls, value: Json) -> DirCheckedModule:
        """Return one DirCheckedModule from one JSON value."""
        return from_json_dir_checked_module(value)


def encode_dir_checked_module(writer: BinaryWriter, value: DirCheckedModule) -> None:
    """Encode one DirCheckedModule."""
    destack._generated.dir.table.annotation.encode_annotation_segment(
        writer, value.annotations
    )
    destack._generated.dir.table.type.encode_type_segment(writer, value.types)
    destack._generated.dir.table.static.encode_static_segment(writer, value.statics)
    destack._generated.dir.table.resolution.encode_resolution_segment(
        writer, value.resolutions
    )
    destack._generated.dir.table.generic.encode_generic_segment(writer, value.generics)
    destack._generated.dir.table.definition.encode_definition_segment(
        writer, value.definitions
    )
    destack._generated.dir.table.coercion.encode_coercion_segment(
        writer, value.coercions
    )
    destack._generated.dir.table.layout.encode_layout_segment(writer, value.layouts)
    destack._generated.dir.table.capture.encode_capture_segment(writer, value.captures)


def decode_dir_checked_module(reader: BinaryReader) -> DirCheckedModule:
    """Decode one DirCheckedModule."""
    annotations = destack._generated.dir.table.annotation.decode_annotation_segment(
        reader
    )
    types = destack._generated.dir.table.type.decode_type_segment(reader)
    statics = destack._generated.dir.table.static.decode_static_segment(reader)
    resolutions = destack._generated.dir.table.resolution.decode_resolution_segment(
        reader
    )
    generics = destack._generated.dir.table.generic.decode_generic_segment(reader)
    definitions = destack._generated.dir.table.definition.decode_definition_segment(
        reader
    )
    coercions = destack._generated.dir.table.coercion.decode_coercion_segment(reader)
    layouts = destack._generated.dir.table.layout.decode_layout_segment(reader)
    captures = destack._generated.dir.table.capture.decode_capture_segment(reader)

    return DirCheckedModule(
        annotations=annotations,
        types=types,
        statics=statics,
        resolutions=resolutions,
        generics=generics,
        definitions=definitions,
        coercions=coercions,
        layouts=layouts,
        captures=captures,
    )


def to_json_dir_checked_module(value: DirCheckedModule) -> Json:
    """Return one JSON value for one DirCheckedModule."""
    return {
        "annotations": destack._generated.dir.table.annotation.to_json_annotation_segment(
            value.annotations
        ),
        "types": destack._generated.dir.table.type.to_json_type_segment(value.types),
        "statics": destack._generated.dir.table.static.to_json_static_segment(
            value.statics
        ),
        "resolutions": destack._generated.dir.table.resolution.to_json_resolution_segment(
            value.resolutions
        ),
        "generics": destack._generated.dir.table.generic.to_json_generic_segment(
            value.generics
        ),
        "definitions": destack._generated.dir.table.definition.to_json_definition_segment(
            value.definitions
        ),
        "coercions": destack._generated.dir.table.coercion.to_json_coercion_segment(
            value.coercions
        ),
        "layouts": destack._generated.dir.table.layout.to_json_layout_segment(
            value.layouts
        ),
        "captures": destack._generated.dir.table.capture.to_json_capture_segment(
            value.captures
        ),
    }


def from_json_dir_checked_module(value: Json) -> DirCheckedModule:
    """Return one DirCheckedModule from one JSON value."""
    object_ = json_object(value)

    return DirCheckedModule(
        annotations=destack._generated.dir.table.annotation.from_json_annotation_segment(
            json_field(object_, "annotations")
        ),
        types=destack._generated.dir.table.type.from_json_type_segment(
            json_field(object_, "types")
        ),
        statics=destack._generated.dir.table.static.from_json_static_segment(
            json_field(object_, "statics")
        ),
        resolutions=destack._generated.dir.table.resolution.from_json_resolution_segment(
            json_field(object_, "resolutions")
        ),
        generics=destack._generated.dir.table.generic.from_json_generic_segment(
            json_field(object_, "generics")
        ),
        definitions=destack._generated.dir.table.definition.from_json_definition_segment(
            json_field(object_, "definitions")
        ),
        coercions=destack._generated.dir.table.coercion.from_json_coercion_segment(
            json_field(object_, "coercions")
        ),
        layouts=destack._generated.dir.table.layout.from_json_layout_segment(
            json_field(object_, "layouts")
        ),
        captures=destack._generated.dir.table.capture.from_json_capture_segment(
            json_field(object_, "captures")
        ),
    )


@dataclass(frozen=True, slots=True)
class DirParsed:
    """Parsed DIR for one source module."""

    # the parsed tree, indexed with its structural parents
    tree: destack._generated.dir.tree.tree.Tree
    # the parsed physical files
    files: Sequence[DirParsedFile]
    # stable anchor expression for diagnostics
    anchor_expression: destack._generated.dir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_parsed(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirParsed:
        """Decode one DirParsed."""
        return decode_dir_parsed(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_parsed(self)

    @classmethod
    def from_json(cls, value: Json) -> DirParsed:
        """Return one DirParsed from one JSON value."""
        return from_json_dir_parsed(value)


def encode_dir_parsed(writer: BinaryWriter, value: DirParsed) -> None:
    """Encode one DirParsed."""
    destack._generated.dir.tree.tree.encode_tree(writer, value.tree)
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        encode_dir_parsed_file(writer, item_value_files_0)
    destack._generated.dir.tree.node.encode_local_node_id(
        writer, value.anchor_expression
    )


def decode_dir_parsed(reader: BinaryReader) -> DirParsed:
    """Decode one DirParsed."""
    tree = destack._generated.dir.tree.tree.decode_tree(reader)
    files = [decode_dir_parsed_file(reader) for _ in range(reader.read_number())]
    anchor_expression = destack._generated.dir.tree.node.decode_local_node_id(reader)

    return DirParsed(
        tree=tree,
        files=files,
        anchor_expression=anchor_expression,
    )


def to_json_dir_parsed(value: DirParsed) -> Json:
    """Return one JSON value for one DirParsed."""
    return {
        "tree": destack._generated.dir.tree.tree.to_json_tree(value.tree),
        "files": [to_json_dir_parsed_file(item_0) for item_0 in value.files],
        "anchorExpression": destack._generated.dir.tree.node.to_json_local_node_id(
            value.anchor_expression
        ),
    }


def from_json_dir_parsed(value: Json) -> DirParsed:
    """Return one DirParsed from one JSON value."""
    object_ = json_object(value)

    return DirParsed(
        tree=destack._generated.dir.tree.tree.from_json_tree(
            json_field(object_, "tree")
        ),
        files=[
            from_json_dir_parsed_file(item_0)
            for item_0 in json_array(json_field(object_, "files"))
        ],
        anchor_expression=destack._generated.dir.tree.node.from_json_local_node_id(
            json_field(object_, "anchorExpression")
        ),
    )


@dataclass(frozen=True, slots=True)
class DirBound:
    """Bound DIR base for one source module."""

    # source bindings
    bindings: destack._generated.dir.table.binding.BindingSegment
    # source types
    types: destack._generated.dir.table.type.TypeSegment
    # source static values
    statics: destack._generated.dir.table.static.StaticSegment
    # top-level expressions
    roots: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # stable module node for module-level state
    module_node: destack._generated.dir.tree.node.LocalNodeIdAny
    # the module namespace scope
    namespace_scope: destack._generated.dir.symbol.scope.LocalScopeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_bound(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirBound:
        """Decode one DirBound."""
        return decode_dir_bound(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_bound(self)

    @classmethod
    def from_json(cls, value: Json) -> DirBound:
        """Return one DirBound from one JSON value."""
        return from_json_dir_bound(value)


def encode_dir_bound(writer: BinaryWriter, value: DirBound) -> None:
    """Encode one DirBound."""
    destack._generated.dir.table.binding.encode_binding_segment(writer, value.bindings)
    destack._generated.dir.table.type.encode_type_segment(writer, value.types)
    destack._generated.dir.table.static.encode_static_segment(writer, value.statics)
    writer.write_unsigned(len(value.roots))
    for item_value_roots_0 in value.roots:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_roots_0
        )
    destack._generated.dir.tree.node.encode_local_node_id_any(writer, value.module_node)
    destack._generated.dir.symbol.scope.encode_local_scope_id(
        writer, value.namespace_scope
    )


def decode_dir_bound(reader: BinaryReader) -> DirBound:
    """Decode one DirBound."""
    bindings = destack._generated.dir.table.binding.decode_binding_segment(reader)
    types = destack._generated.dir.table.type.decode_type_segment(reader)
    statics = destack._generated.dir.table.static.decode_static_segment(reader)
    roots = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    module_node = destack._generated.dir.tree.node.decode_local_node_id_any(reader)
    namespace_scope = destack._generated.dir.symbol.scope.decode_local_scope_id(reader)

    return DirBound(
        bindings=bindings,
        types=types,
        statics=statics,
        roots=roots,
        module_node=module_node,
        namespace_scope=namespace_scope,
    )


def to_json_dir_bound(value: DirBound) -> Json:
    """Return one JSON value for one DirBound."""
    return {
        "bindings": destack._generated.dir.table.binding.to_json_binding_segment(
            value.bindings
        ),
        "types": destack._generated.dir.table.type.to_json_type_segment(value.types),
        "statics": destack._generated.dir.table.static.to_json_static_segment(
            value.statics
        ),
        "roots": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.roots
        ],
        "moduleNode": destack._generated.dir.tree.node.to_json_local_node_id_any(
            value.module_node
        ),
        "namespaceScope": destack._generated.dir.symbol.scope.to_json_local_scope_id(
            value.namespace_scope
        ),
    }


def from_json_dir_bound(value: Json) -> DirBound:
    """Return one DirBound from one JSON value."""
    object_ = json_object(value)

    return DirBound(
        bindings=destack._generated.dir.table.binding.from_json_binding_segment(
            json_field(object_, "bindings")
        ),
        types=destack._generated.dir.table.type.from_json_type_segment(
            json_field(object_, "types")
        ),
        statics=destack._generated.dir.table.static.from_json_static_segment(
            json_field(object_, "statics")
        ),
        roots=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "roots"))
        ],
        module_node=destack._generated.dir.tree.node.from_json_local_node_id_any(
            json_field(object_, "moduleNode")
        ),
        namespace_scope=destack._generated.dir.symbol.scope.from_json_local_scope_id(
            json_field(object_, "namespaceScope")
        ),
    )


@dataclass(frozen=True, slots=True)
class DirImported:
    """Source import resolution for one profile-scoped module."""

    # resolved module imports
    modules: destack._generated.dir.table.module.ModuleSegment

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_imported(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirImported:
        """Decode one DirImported."""
        return decode_dir_imported(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_imported(self)

    @classmethod
    def from_json(cls, value: Json) -> DirImported:
        """Return one DirImported from one JSON value."""
        return from_json_dir_imported(value)


def encode_dir_imported(writer: BinaryWriter, value: DirImported) -> None:
    """Encode one DirImported."""
    destack._generated.dir.table.module.encode_module_segment(writer, value.modules)


def decode_dir_imported(reader: BinaryReader) -> DirImported:
    """Decode one DirImported."""
    modules = destack._generated.dir.table.module.decode_module_segment(reader)

    return DirImported(
        modules=modules,
    )


def to_json_dir_imported(value: DirImported) -> Json:
    """Return one JSON value for one DirImported."""
    return {
        "modules": destack._generated.dir.table.module.to_json_module_segment(
            value.modules
        ),
    }


def from_json_dir_imported(value: Json) -> DirImported:
    """Return one DirImported from one JSON value."""
    object_ = json_object(value)

    return DirImported(
        modules=destack._generated.dir.table.module.from_json_module_segment(
            json_field(object_, "modules")
        ),
    )


@dataclass(frozen=True, slots=True)
class DirExpanded:
    """Fixed-point macro expansion segment for one profile-scoped module."""

    # tree changes
    patch: destack._generated.dir.tree.patch.Patch
    # new bindings
    bindings: destack._generated.dir.table.binding.BindingSegment
    # new module imports
    modules: destack._generated.dir.table.module.ModuleSegment
    # new types
    types: destack._generated.dir.table.type.TypeSegment
    # new static values
    statics: destack._generated.dir.table.static.StaticSegment
    # expanded macro invocations
    macros: destack._generated.dir.table.macro.MacroTable
    # top-level expressions
    roots: Sequence[destack._generated.dir.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_expanded(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirExpanded:
        """Decode one DirExpanded."""
        return decode_dir_expanded(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_expanded(self)

    @classmethod
    def from_json(cls, value: Json) -> DirExpanded:
        """Return one DirExpanded from one JSON value."""
        return from_json_dir_expanded(value)


def encode_dir_expanded(writer: BinaryWriter, value: DirExpanded) -> None:
    """Encode one DirExpanded."""
    destack._generated.dir.tree.patch.encode_patch(writer, value.patch)
    destack._generated.dir.table.binding.encode_binding_segment(writer, value.bindings)
    destack._generated.dir.table.module.encode_module_segment(writer, value.modules)
    destack._generated.dir.table.type.encode_type_segment(writer, value.types)
    destack._generated.dir.table.static.encode_static_segment(writer, value.statics)
    destack._generated.dir.table.macro.encode_macro_table(writer, value.macros)
    writer.write_unsigned(len(value.roots))
    for item_value_roots_0 in value.roots:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_roots_0
        )


def decode_dir_expanded(reader: BinaryReader) -> DirExpanded:
    """Decode one DirExpanded."""
    patch = destack._generated.dir.tree.patch.decode_patch(reader)
    bindings = destack._generated.dir.table.binding.decode_binding_segment(reader)
    modules = destack._generated.dir.table.module.decode_module_segment(reader)
    types = destack._generated.dir.table.type.decode_type_segment(reader)
    statics = destack._generated.dir.table.static.decode_static_segment(reader)
    macros = destack._generated.dir.table.macro.decode_macro_table(reader)
    roots = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return DirExpanded(
        patch=patch,
        bindings=bindings,
        modules=modules,
        types=types,
        statics=statics,
        macros=macros,
        roots=roots,
    )


def to_json_dir_expanded(value: DirExpanded) -> Json:
    """Return one JSON value for one DirExpanded."""
    return {
        "patch": destack._generated.dir.tree.patch.to_json_patch(value.patch),
        "bindings": destack._generated.dir.table.binding.to_json_binding_segment(
            value.bindings
        ),
        "modules": destack._generated.dir.table.module.to_json_module_segment(
            value.modules
        ),
        "types": destack._generated.dir.table.type.to_json_type_segment(value.types),
        "statics": destack._generated.dir.table.static.to_json_static_segment(
            value.statics
        ),
        "macros": destack._generated.dir.table.macro.to_json_macro_table(value.macros),
        "roots": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.roots
        ],
    }


def from_json_dir_expanded(value: Json) -> DirExpanded:
    """Return one DirExpanded from one JSON value."""
    object_ = json_object(value)

    return DirExpanded(
        patch=destack._generated.dir.tree.patch.from_json_patch(
            json_field(object_, "patch")
        ),
        bindings=destack._generated.dir.table.binding.from_json_binding_segment(
            json_field(object_, "bindings")
        ),
        modules=destack._generated.dir.table.module.from_json_module_segment(
            json_field(object_, "modules")
        ),
        types=destack._generated.dir.table.type.from_json_type_segment(
            json_field(object_, "types")
        ),
        statics=destack._generated.dir.table.static.from_json_static_segment(
            json_field(object_, "statics")
        ),
        macros=destack._generated.dir.table.macro.from_json_macro_table(
            json_field(object_, "macros")
        ),
        roots=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "roots"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DirExported:
    """Export table over the expanded view for one profile-scoped module."""

    # resolved exports
    exports: destack._generated.dir.table.export.ExportTable
    # global declarations contributed by this module
    globals: destack._generated.dir.table.global_.GlobalTable

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_exported(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirExported:
        """Decode one DirExported."""
        return decode_dir_exported(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_exported(self)

    @classmethod
    def from_json(cls, value: Json) -> DirExported:
        """Return one DirExported from one JSON value."""
        return from_json_dir_exported(value)


def encode_dir_exported(writer: BinaryWriter, value: DirExported) -> None:
    """Encode one DirExported."""
    destack._generated.dir.table.export.encode_export_table(writer, value.exports)
    destack._generated.dir.table.global_.encode_global_table(writer, value.globals)


def decode_dir_exported(reader: BinaryReader) -> DirExported:
    """Decode one DirExported."""
    exports = destack._generated.dir.table.export.decode_export_table(reader)
    globals = destack._generated.dir.table.global_.decode_global_table(reader)

    return DirExported(
        exports=exports,
        globals=globals,
    )


def to_json_dir_exported(value: DirExported) -> Json:
    """Return one JSON value for one DirExported."""
    return {
        "exports": destack._generated.dir.table.export.to_json_export_table(
            value.exports
        ),
        "globals": destack._generated.dir.table.global_.to_json_global_table(
            value.globals
        ),
    }


def from_json_dir_exported(value: Json) -> DirExported:
    """Return one DirExported from one JSON value."""
    object_ = json_object(value)

    return DirExported(
        exports=destack._generated.dir.table.export.from_json_export_table(
            json_field(object_, "exports")
        ),
        globals=destack._generated.dir.table.global_.from_json_global_table(
            json_field(object_, "globals")
        ),
    )


@dataclass(frozen=True, slots=True)
class DirResolved:
    """Resolved import targets for one profile-scoped module."""

    # resolved imports
    imports: destack._generated.dir.table.import_.ImportTable
    # resolved source references
    references: destack._generated.dir.table.reference.ReferenceTable

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_resolved(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirResolved:
        """Decode one DirResolved."""
        return decode_dir_resolved(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_resolved(self)

    @classmethod
    def from_json(cls, value: Json) -> DirResolved:
        """Return one DirResolved from one JSON value."""
        return from_json_dir_resolved(value)


def encode_dir_resolved(writer: BinaryWriter, value: DirResolved) -> None:
    """Encode one DirResolved."""
    destack._generated.dir.table.import_.encode_import_table(writer, value.imports)
    destack._generated.dir.table.reference.encode_reference_table(
        writer, value.references
    )


def decode_dir_resolved(reader: BinaryReader) -> DirResolved:
    """Decode one DirResolved."""
    imports = destack._generated.dir.table.import_.decode_import_table(reader)
    references = destack._generated.dir.table.reference.decode_reference_table(reader)

    return DirResolved(
        imports=imports,
        references=references,
    )


def to_json_dir_resolved(value: DirResolved) -> Json:
    """Return one JSON value for one DirResolved."""
    return {
        "imports": destack._generated.dir.table.import_.to_json_import_table(
            value.imports
        ),
        "references": destack._generated.dir.table.reference.to_json_reference_table(
            value.references
        ),
    }


def from_json_dir_resolved(value: Json) -> DirResolved:
    """Return one DirResolved from one JSON value."""
    object_ = json_object(value)

    return DirResolved(
        imports=destack._generated.dir.table.import_.from_json_import_table(
            json_field(object_, "imports")
        ),
        references=destack._generated.dir.table.reference.from_json_reference_table(
            json_field(object_, "references")
        ),
    )


@dataclass(frozen=True, slots=True)
class DirCheckedComponent:
    """Checked DIR output for one source component."""

    # the checked component id
    component: destack._generated.source.file.model.component.ComponentId
    # the checked module outputs in stable module order
    modules: Sequence[DirCheckedComponentEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_checked_component(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirCheckedComponent:
        """Decode one DirCheckedComponent."""
        return decode_dir_checked_component(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_checked_component(self)

    @classmethod
    def from_json(cls, value: Json) -> DirCheckedComponent:
        """Return one DirCheckedComponent from one JSON value."""
        return from_json_dir_checked_component(value)


def encode_dir_checked_component(
    writer: BinaryWriter, value: DirCheckedComponent
) -> None:
    """Encode one DirCheckedComponent."""
    destack._generated.source.file.model.component.encode_component_id(
        writer, value.component
    )
    writer.write_unsigned(len(value.modules))
    for item_value_modules_0 in value.modules:
        encode_dir_checked_component_entry(writer, item_value_modules_0)


def decode_dir_checked_component(reader: BinaryReader) -> DirCheckedComponent:
    """Decode one DirCheckedComponent."""
    component = destack._generated.source.file.model.component.decode_component_id(
        reader
    )
    modules = [
        decode_dir_checked_component_entry(reader) for _ in range(reader.read_number())
    ]

    return DirCheckedComponent(
        component=component,
        modules=modules,
    )


def to_json_dir_checked_component(value: DirCheckedComponent) -> Json:
    """Return one JSON value for one DirCheckedComponent."""
    return {
        "component": destack._generated.source.file.model.component.to_json_component_id(
            value.component
        ),
        "modules": [
            to_json_dir_checked_component_entry(item_0) for item_0 in value.modules
        ],
    }


def from_json_dir_checked_component(value: Json) -> DirCheckedComponent:
    """Return one DirCheckedComponent from one JSON value."""
    object_ = json_object(value)

    return DirCheckedComponent(
        component=destack._generated.source.file.model.component.from_json_component_id(
            json_field(object_, "component")
        ),
        modules=[
            from_json_dir_checked_component_entry(item_0)
            for item_0 in json_array(json_field(object_, "modules"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DirChecked:
    """Facade artifact for one module checked inside a component."""

    # the component that owns this module's checked output
    component: destack._generated.source.file.model.component.ComponentId
    # the module used to enter the checked component graph
    entry: destack._generated.source.file.model.module.ModuleId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_checked(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirChecked:
        """Decode one DirChecked."""
        return decode_dir_checked(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_checked(self)

    @classmethod
    def from_json(cls, value: Json) -> DirChecked:
        """Return one DirChecked from one JSON value."""
        return from_json_dir_checked(value)


def encode_dir_checked(writer: BinaryWriter, value: DirChecked) -> None:
    """Encode one DirChecked."""
    destack._generated.source.file.model.component.encode_component_id(
        writer, value.component
    )
    destack._generated.source.file.model.module.encode_module_id(writer, value.entry)


def decode_dir_checked(reader: BinaryReader) -> DirChecked:
    """Decode one DirChecked."""
    component = destack._generated.source.file.model.component.decode_component_id(
        reader
    )
    entry = destack._generated.source.file.model.module.decode_module_id(reader)

    return DirChecked(
        component=component,
        entry=entry,
    )


def to_json_dir_checked(value: DirChecked) -> Json:
    """Return one JSON value for one DirChecked."""
    return {
        "component": destack._generated.source.file.model.component.to_json_component_id(
            value.component
        ),
        "entry": destack._generated.source.file.model.module.to_json_module_id(
            value.entry
        ),
    }


def from_json_dir_checked(value: Json) -> DirChecked:
    """Return one DirChecked from one JSON value."""
    object_ = json_object(value)

    return DirChecked(
        component=destack._generated.source.file.model.component.from_json_component_id(
            json_field(object_, "component")
        ),
        entry=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "entry")
        ),
    )


@dataclass(frozen=True, slots=True)
class DirMaterialized:
    """Comptime materialization segment for one profile-scoped module."""

    # tree changes
    patch: destack._generated.dir.tree.patch.Patch
    # new bindings
    bindings: destack._generated.dir.table.binding.BindingSegment
    # new types
    types: destack._generated.dir.table.type.TypeSegment
    # new static values
    statics: destack._generated.dir.table.static.StaticSegment
    # new resolutions
    resolutions: destack._generated.dir.table.resolution.ResolutionSegment
    # new generic slots and instances
    generics: destack._generated.dir.table.generic.GenericSegment
    # new implicit coercions
    coercions: destack._generated.dir.table.coercion.CoercionSegment
    # new captures
    captures: destack._generated.dir.table.capture.CaptureSegment
    # new layouts
    layouts: destack._generated.dir.table.layout.LayoutSegment
    # top-level expressions
    roots: Sequence[destack._generated.dir.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_materialized(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirMaterialized:
        """Decode one DirMaterialized."""
        return decode_dir_materialized(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_materialized(self)

    @classmethod
    def from_json(cls, value: Json) -> DirMaterialized:
        """Return one DirMaterialized from one JSON value."""
        return from_json_dir_materialized(value)


def encode_dir_materialized(writer: BinaryWriter, value: DirMaterialized) -> None:
    """Encode one DirMaterialized."""
    destack._generated.dir.tree.patch.encode_patch(writer, value.patch)
    destack._generated.dir.table.binding.encode_binding_segment(writer, value.bindings)
    destack._generated.dir.table.type.encode_type_segment(writer, value.types)
    destack._generated.dir.table.static.encode_static_segment(writer, value.statics)
    destack._generated.dir.table.resolution.encode_resolution_segment(
        writer, value.resolutions
    )
    destack._generated.dir.table.generic.encode_generic_segment(writer, value.generics)
    destack._generated.dir.table.coercion.encode_coercion_segment(
        writer, value.coercions
    )
    destack._generated.dir.table.capture.encode_capture_segment(writer, value.captures)
    destack._generated.dir.table.layout.encode_layout_segment(writer, value.layouts)
    writer.write_unsigned(len(value.roots))
    for item_value_roots_0 in value.roots:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_roots_0
        )


def decode_dir_materialized(reader: BinaryReader) -> DirMaterialized:
    """Decode one DirMaterialized."""
    patch = destack._generated.dir.tree.patch.decode_patch(reader)
    bindings = destack._generated.dir.table.binding.decode_binding_segment(reader)
    types = destack._generated.dir.table.type.decode_type_segment(reader)
    statics = destack._generated.dir.table.static.decode_static_segment(reader)
    resolutions = destack._generated.dir.table.resolution.decode_resolution_segment(
        reader
    )
    generics = destack._generated.dir.table.generic.decode_generic_segment(reader)
    coercions = destack._generated.dir.table.coercion.decode_coercion_segment(reader)
    captures = destack._generated.dir.table.capture.decode_capture_segment(reader)
    layouts = destack._generated.dir.table.layout.decode_layout_segment(reader)
    roots = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return DirMaterialized(
        patch=patch,
        bindings=bindings,
        types=types,
        statics=statics,
        resolutions=resolutions,
        generics=generics,
        coercions=coercions,
        captures=captures,
        layouts=layouts,
        roots=roots,
    )


def to_json_dir_materialized(value: DirMaterialized) -> Json:
    """Return one JSON value for one DirMaterialized."""
    return {
        "patch": destack._generated.dir.tree.patch.to_json_patch(value.patch),
        "bindings": destack._generated.dir.table.binding.to_json_binding_segment(
            value.bindings
        ),
        "types": destack._generated.dir.table.type.to_json_type_segment(value.types),
        "statics": destack._generated.dir.table.static.to_json_static_segment(
            value.statics
        ),
        "resolutions": destack._generated.dir.table.resolution.to_json_resolution_segment(
            value.resolutions
        ),
        "generics": destack._generated.dir.table.generic.to_json_generic_segment(
            value.generics
        ),
        "coercions": destack._generated.dir.table.coercion.to_json_coercion_segment(
            value.coercions
        ),
        "captures": destack._generated.dir.table.capture.to_json_capture_segment(
            value.captures
        ),
        "layouts": destack._generated.dir.table.layout.to_json_layout_segment(
            value.layouts
        ),
        "roots": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.roots
        ],
    }


def from_json_dir_materialized(value: Json) -> DirMaterialized:
    """Return one DirMaterialized from one JSON value."""
    object_ = json_object(value)

    return DirMaterialized(
        patch=destack._generated.dir.tree.patch.from_json_patch(
            json_field(object_, "patch")
        ),
        bindings=destack._generated.dir.table.binding.from_json_binding_segment(
            json_field(object_, "bindings")
        ),
        types=destack._generated.dir.table.type.from_json_type_segment(
            json_field(object_, "types")
        ),
        statics=destack._generated.dir.table.static.from_json_static_segment(
            json_field(object_, "statics")
        ),
        resolutions=destack._generated.dir.table.resolution.from_json_resolution_segment(
            json_field(object_, "resolutions")
        ),
        generics=destack._generated.dir.table.generic.from_json_generic_segment(
            json_field(object_, "generics")
        ),
        coercions=destack._generated.dir.table.coercion.from_json_coercion_segment(
            json_field(object_, "coercions")
        ),
        captures=destack._generated.dir.table.capture.from_json_capture_segment(
            json_field(object_, "captures")
        ),
        layouts=destack._generated.dir.table.layout.from_json_layout_segment(
            json_field(object_, "layouts")
        ),
        roots=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "roots"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DirElaborated:
    """DIR-to-MIR elaboration segment for one profile-scoped module."""

    # tree changes
    patch: destack._generated.dir.tree.patch.Patch
    # new bindings
    bindings: destack._generated.dir.table.binding.BindingSegment
    # new types
    types: destack._generated.dir.table.type.TypeSegment
    # new static values
    statics: destack._generated.dir.table.static.StaticSegment
    # new resolutions
    resolutions: destack._generated.dir.table.resolution.ResolutionSegment
    # new generic slots and instances
    generics: destack._generated.dir.table.generic.GenericSegment
    # new implicit coercions
    coercions: destack._generated.dir.table.coercion.CoercionSegment
    # new captures
    captures: destack._generated.dir.table.capture.CaptureSegment
    # new layouts
    layouts: destack._generated.dir.table.layout.LayoutSegment
    # top-level expressions
    roots: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # new guards
    guards: destack._generated.dir.table.guard.GuardTable

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dir_elaborated(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DirElaborated:
        """Decode one DirElaborated."""
        return decode_dir_elaborated(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dir_elaborated(self)

    @classmethod
    def from_json(cls, value: Json) -> DirElaborated:
        """Return one DirElaborated from one JSON value."""
        return from_json_dir_elaborated(value)


def encode_dir_elaborated(writer: BinaryWriter, value: DirElaborated) -> None:
    """Encode one DirElaborated."""
    destack._generated.dir.tree.patch.encode_patch(writer, value.patch)
    destack._generated.dir.table.binding.encode_binding_segment(writer, value.bindings)
    destack._generated.dir.table.type.encode_type_segment(writer, value.types)
    destack._generated.dir.table.static.encode_static_segment(writer, value.statics)
    destack._generated.dir.table.resolution.encode_resolution_segment(
        writer, value.resolutions
    )
    destack._generated.dir.table.generic.encode_generic_segment(writer, value.generics)
    destack._generated.dir.table.coercion.encode_coercion_segment(
        writer, value.coercions
    )
    destack._generated.dir.table.capture.encode_capture_segment(writer, value.captures)
    destack._generated.dir.table.layout.encode_layout_segment(writer, value.layouts)
    writer.write_unsigned(len(value.roots))
    for item_value_roots_0 in value.roots:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_roots_0
        )
    destack._generated.dir.table.guard.encode_guard_table(writer, value.guards)


def decode_dir_elaborated(reader: BinaryReader) -> DirElaborated:
    """Decode one DirElaborated."""
    patch = destack._generated.dir.tree.patch.decode_patch(reader)
    bindings = destack._generated.dir.table.binding.decode_binding_segment(reader)
    types = destack._generated.dir.table.type.decode_type_segment(reader)
    statics = destack._generated.dir.table.static.decode_static_segment(reader)
    resolutions = destack._generated.dir.table.resolution.decode_resolution_segment(
        reader
    )
    generics = destack._generated.dir.table.generic.decode_generic_segment(reader)
    coercions = destack._generated.dir.table.coercion.decode_coercion_segment(reader)
    captures = destack._generated.dir.table.capture.decode_capture_segment(reader)
    layouts = destack._generated.dir.table.layout.decode_layout_segment(reader)
    roots = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    guards = destack._generated.dir.table.guard.decode_guard_table(reader)

    return DirElaborated(
        patch=patch,
        bindings=bindings,
        types=types,
        statics=statics,
        resolutions=resolutions,
        generics=generics,
        coercions=coercions,
        captures=captures,
        layouts=layouts,
        roots=roots,
        guards=guards,
    )


def to_json_dir_elaborated(value: DirElaborated) -> Json:
    """Return one JSON value for one DirElaborated."""
    return {
        "patch": destack._generated.dir.tree.patch.to_json_patch(value.patch),
        "bindings": destack._generated.dir.table.binding.to_json_binding_segment(
            value.bindings
        ),
        "types": destack._generated.dir.table.type.to_json_type_segment(value.types),
        "statics": destack._generated.dir.table.static.to_json_static_segment(
            value.statics
        ),
        "resolutions": destack._generated.dir.table.resolution.to_json_resolution_segment(
            value.resolutions
        ),
        "generics": destack._generated.dir.table.generic.to_json_generic_segment(
            value.generics
        ),
        "coercions": destack._generated.dir.table.coercion.to_json_coercion_segment(
            value.coercions
        ),
        "captures": destack._generated.dir.table.capture.to_json_capture_segment(
            value.captures
        ),
        "layouts": destack._generated.dir.table.layout.to_json_layout_segment(
            value.layouts
        ),
        "roots": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.roots
        ],
        "guards": destack._generated.dir.table.guard.to_json_guard_table(value.guards),
    }


def from_json_dir_elaborated(value: Json) -> DirElaborated:
    """Return one DirElaborated from one JSON value."""
    object_ = json_object(value)

    return DirElaborated(
        patch=destack._generated.dir.tree.patch.from_json_patch(
            json_field(object_, "patch")
        ),
        bindings=destack._generated.dir.table.binding.from_json_binding_segment(
            json_field(object_, "bindings")
        ),
        types=destack._generated.dir.table.type.from_json_type_segment(
            json_field(object_, "types")
        ),
        statics=destack._generated.dir.table.static.from_json_static_segment(
            json_field(object_, "statics")
        ),
        resolutions=destack._generated.dir.table.resolution.from_json_resolution_segment(
            json_field(object_, "resolutions")
        ),
        generics=destack._generated.dir.table.generic.from_json_generic_segment(
            json_field(object_, "generics")
        ),
        coercions=destack._generated.dir.table.coercion.from_json_coercion_segment(
            json_field(object_, "coercions")
        ),
        captures=destack._generated.dir.table.capture.from_json_capture_segment(
            json_field(object_, "captures")
        ),
        layouts=destack._generated.dir.table.layout.from_json_layout_segment(
            json_field(object_, "layouts")
        ),
        roots=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "roots"))
        ],
        guards=destack._generated.dir.table.guard.from_json_guard_table(
            json_field(object_, "guards")
        ),
    )


__all__ = [
    "DirParsedFile",
    "encode_dir_parsed_file",
    "decode_dir_parsed_file",
    "to_json_dir_parsed_file",
    "from_json_dir_parsed_file",
    "DirCheckedComponentEntry",
    "encode_dir_checked_component_entry",
    "decode_dir_checked_component_entry",
    "to_json_dir_checked_component_entry",
    "from_json_dir_checked_component_entry",
    "DirCheckedModule",
    "encode_dir_checked_module",
    "decode_dir_checked_module",
    "to_json_dir_checked_module",
    "from_json_dir_checked_module",
    "DirParsed",
    "encode_dir_parsed",
    "decode_dir_parsed",
    "to_json_dir_parsed",
    "from_json_dir_parsed",
    "DirBound",
    "encode_dir_bound",
    "decode_dir_bound",
    "to_json_dir_bound",
    "from_json_dir_bound",
    "DirImported",
    "encode_dir_imported",
    "decode_dir_imported",
    "to_json_dir_imported",
    "from_json_dir_imported",
    "DirExpanded",
    "encode_dir_expanded",
    "decode_dir_expanded",
    "to_json_dir_expanded",
    "from_json_dir_expanded",
    "DirExported",
    "encode_dir_exported",
    "decode_dir_exported",
    "to_json_dir_exported",
    "from_json_dir_exported",
    "DirResolved",
    "encode_dir_resolved",
    "decode_dir_resolved",
    "to_json_dir_resolved",
    "from_json_dir_resolved",
    "DirCheckedComponent",
    "encode_dir_checked_component",
    "decode_dir_checked_component",
    "to_json_dir_checked_component",
    "from_json_dir_checked_component",
    "DirChecked",
    "encode_dir_checked",
    "decode_dir_checked",
    "to_json_dir_checked",
    "from_json_dir_checked",
    "DirMaterialized",
    "encode_dir_materialized",
    "decode_dir_materialized",
    "to_json_dir_materialized",
    "from_json_dir_materialized",
    "DirElaborated",
    "encode_dir_elaborated",
    "decode_dir_elaborated",
    "to_json_dir_elaborated",
    "from_json_dir_elaborated",
]
