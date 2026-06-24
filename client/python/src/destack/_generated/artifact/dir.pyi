# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirParsedFile: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirParsedFile: ...

def encode_dir_parsed_file(writer: BinaryWriter, value: DirParsedFile) -> None: ...
def decode_dir_parsed_file(reader: BinaryReader) -> DirParsedFile: ...
def to_json_dir_parsed_file(value: DirParsedFile) -> Json: ...
def from_json_dir_parsed_file(value: Json) -> DirParsedFile: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirCheckedComponentEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirCheckedComponentEntry: ...

def encode_dir_checked_component_entry(
    writer: BinaryWriter, value: DirCheckedComponentEntry
) -> None: ...
def decode_dir_checked_component_entry(
    reader: BinaryReader,
) -> DirCheckedComponentEntry: ...
def to_json_dir_checked_component_entry(value: DirCheckedComponentEntry) -> Json: ...
def from_json_dir_checked_component_entry(value: Json) -> DirCheckedComponentEntry: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirCheckedModule: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirCheckedModule: ...

def encode_dir_checked_module(
    writer: BinaryWriter, value: DirCheckedModule
) -> None: ...
def decode_dir_checked_module(reader: BinaryReader) -> DirCheckedModule: ...
def to_json_dir_checked_module(value: DirCheckedModule) -> Json: ...
def from_json_dir_checked_module(value: Json) -> DirCheckedModule: ...

@dataclass(frozen=True, slots=True)
class DirParsed:
    """Parsed DIR for one source module."""

    # the parsed tree, indexed with its structural parents
    tree: destack._generated.dir.tree.tree.Tree
    # the parsed physical files
    files: Sequence[DirParsedFile]
    # stable anchor expression for diagnostics
    anchor_expression: destack._generated.dir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirParsed: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirParsed: ...

def encode_dir_parsed(writer: BinaryWriter, value: DirParsed) -> None: ...
def decode_dir_parsed(reader: BinaryReader) -> DirParsed: ...
def to_json_dir_parsed(value: DirParsed) -> Json: ...
def from_json_dir_parsed(value: Json) -> DirParsed: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirBound: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirBound: ...

def encode_dir_bound(writer: BinaryWriter, value: DirBound) -> None: ...
def decode_dir_bound(reader: BinaryReader) -> DirBound: ...
def to_json_dir_bound(value: DirBound) -> Json: ...
def from_json_dir_bound(value: Json) -> DirBound: ...

@dataclass(frozen=True, slots=True)
class DirImported:
    """Source import resolution for one profile-scoped module."""

    # resolved module imports
    modules: destack._generated.dir.table.module.ModuleSegment

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirImported: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirImported: ...

def encode_dir_imported(writer: BinaryWriter, value: DirImported) -> None: ...
def decode_dir_imported(reader: BinaryReader) -> DirImported: ...
def to_json_dir_imported(value: DirImported) -> Json: ...
def from_json_dir_imported(value: Json) -> DirImported: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirExpanded: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirExpanded: ...

def encode_dir_expanded(writer: BinaryWriter, value: DirExpanded) -> None: ...
def decode_dir_expanded(reader: BinaryReader) -> DirExpanded: ...
def to_json_dir_expanded(value: DirExpanded) -> Json: ...
def from_json_dir_expanded(value: Json) -> DirExpanded: ...

@dataclass(frozen=True, slots=True)
class DirExported:
    """Export table over the expanded view for one profile-scoped module."""

    # resolved exports
    exports: destack._generated.dir.table.export.ExportTable
    # global declarations contributed by this module
    globals: destack._generated.dir.table.global_.GlobalTable

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirExported: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirExported: ...

def encode_dir_exported(writer: BinaryWriter, value: DirExported) -> None: ...
def decode_dir_exported(reader: BinaryReader) -> DirExported: ...
def to_json_dir_exported(value: DirExported) -> Json: ...
def from_json_dir_exported(value: Json) -> DirExported: ...

@dataclass(frozen=True, slots=True)
class DirResolved:
    """Resolved import targets for one profile-scoped module."""

    # resolved imports
    imports: destack._generated.dir.table.import_.ImportTable
    # resolved source references
    references: destack._generated.dir.table.reference.ReferenceTable

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirResolved: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirResolved: ...

def encode_dir_resolved(writer: BinaryWriter, value: DirResolved) -> None: ...
def decode_dir_resolved(reader: BinaryReader) -> DirResolved: ...
def to_json_dir_resolved(value: DirResolved) -> Json: ...
def from_json_dir_resolved(value: Json) -> DirResolved: ...

@dataclass(frozen=True, slots=True)
class DirCheckedComponent:
    """Checked DIR output for one source component."""

    # the checked component id
    component: destack._generated.source.file.model.component.ComponentId
    # the checked module outputs in stable module order
    modules: Sequence[DirCheckedComponentEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirCheckedComponent: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirCheckedComponent: ...

def encode_dir_checked_component(
    writer: BinaryWriter, value: DirCheckedComponent
) -> None: ...
def decode_dir_checked_component(reader: BinaryReader) -> DirCheckedComponent: ...
def to_json_dir_checked_component(value: DirCheckedComponent) -> Json: ...
def from_json_dir_checked_component(value: Json) -> DirCheckedComponent: ...

@dataclass(frozen=True, slots=True)
class DirChecked:
    """Facade artifact for one module checked inside a component."""

    # the component that owns this module's checked output
    component: destack._generated.source.file.model.component.ComponentId
    # the module used to enter the checked component graph
    entry: destack._generated.source.file.model.module.ModuleId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirChecked: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirChecked: ...

def encode_dir_checked(writer: BinaryWriter, value: DirChecked) -> None: ...
def decode_dir_checked(reader: BinaryReader) -> DirChecked: ...
def to_json_dir_checked(value: DirChecked) -> Json: ...
def from_json_dir_checked(value: Json) -> DirChecked: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirMaterialized: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirMaterialized: ...

def encode_dir_materialized(writer: BinaryWriter, value: DirMaterialized) -> None: ...
def decode_dir_materialized(reader: BinaryReader) -> DirMaterialized: ...
def to_json_dir_materialized(value: DirMaterialized) -> Json: ...
def from_json_dir_materialized(value: Json) -> DirMaterialized: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DirElaborated: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DirElaborated: ...

def encode_dir_elaborated(writer: BinaryWriter, value: DirElaborated) -> None: ...
def decode_dir_elaborated(reader: BinaryReader) -> DirElaborated: ...
def to_json_dir_elaborated(value: DirElaborated) -> Json: ...
def from_json_dir_elaborated(value: Json) -> DirElaborated: ...

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
