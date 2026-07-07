# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.asset
import destack._generated.artifact.build
import destack._generated.artifact.bundle
import destack._generated.artifact.component
import destack._generated.artifact.data
import destack._generated.artifact.dir
import destack._generated.artifact.environment
import destack._generated.artifact.index
import destack._generated.artifact.lint
import destack._generated.artifact.mir
import destack._generated.artifact.object
import destack._generated.artifact.package
import destack._generated.artifact.product
import destack._generated.artifact.script
import destack._generated.program.model

@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirParsed:
    """Parsed module DIR."""

    dir_parsed: destack._generated.artifact.dir.DirParsed
    kind: typing.Literal["dirParsed"] = "dirParsed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadData:
    """Parsed non-code module data."""

    data: destack._generated.artifact.data.Data
    kind: typing.Literal["data"] = "data"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadGlobalEnvironment:
    """Explicit global environment for one profile."""

    global_environment: destack._generated.artifact.environment.GlobalEnvironment
    kind: typing.Literal["globalEnvironment"] = "globalEnvironment"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadPackageIndex:
    """Active dependency index for one profile."""

    package_index: destack._generated.artifact.package.PackageIndex
    kind: typing.Literal["packageIndex"] = "packageIndex"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadComponentGraph:
    """Component partition for one profile."""

    component_graph: destack._generated.artifact.component.ComponentGraph
    kind: typing.Literal["componentGraph"] = "componentGraph"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadProgramAnalysis:
    """Whole-program analysis for one profile and target."""

    program_analysis: destack._generated.artifact.mir.ProgramAnalysis
    kind: typing.Literal["programAnalysis"] = "programAnalysis"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirBound:
    """Bound DIR."""

    dir_bound: destack._generated.artifact.dir.DirBound
    kind: typing.Literal["dirBound"] = "dirBound"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirImported:
    """Imported DIR."""

    dir_imported: destack._generated.artifact.dir.DirImported
    kind: typing.Literal["dirImported"] = "dirImported"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirExpanded:
    """Expanded DIR."""

    dir_expanded: destack._generated.artifact.dir.DirExpanded
    kind: typing.Literal["dirExpanded"] = "dirExpanded"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirExported:
    """Exported DIR."""

    dir_exported: destack._generated.artifact.dir.DirExported
    kind: typing.Literal["dirExported"] = "dirExported"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirResolved:
    """Resolved DIR imports."""

    dir_resolved: destack._generated.artifact.dir.DirResolved
    kind: typing.Literal["dirResolved"] = "dirResolved"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirCheckedComponent:
    """Checked DIR component."""

    dir_checked_component: destack._generated.artifact.dir.DirCheckedComponent
    kind: typing.Literal["dirCheckedComponent"] = "dirCheckedComponent"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirChecked:
    """Checked DIR facade."""

    dir_checked: destack._generated.artifact.dir.DirChecked
    kind: typing.Literal["dirChecked"] = "dirChecked"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirMaterialized:
    """Materialized DIR."""

    dir_materialized: destack._generated.artifact.dir.DirMaterialized
    kind: typing.Literal["dirMaterialized"] = "dirMaterialized"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadMirLowered:
    """Lowered MIR before optimization."""

    mir_lowered: destack._generated.artifact.mir.MirLowered
    kind: typing.Literal["mirLowered"] = "mirLowered"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadMirVerified:
    """Verified MIR marker after required semantic verification."""

    mir_verified: destack._generated.artifact.mir.MirVerified
    kind: typing.Literal["mirVerified"] = "mirVerified"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadMirAnalyzed:
    """Per-module link summary for whole-program analysis."""

    mir_analyzed: destack._generated.artifact.mir.MirAnalyzed
    kind: typing.Literal["mirAnalyzed"] = "mirAnalyzed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadMirOptimized:
    """Optimized MIR."""

    mir_optimized: destack._generated.artifact.mir.MirOptimized
    kind: typing.Literal["mirOptimized"] = "mirOptimized"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadModuleIndex:
    """Index for one module profile."""

    module_index: destack._generated.artifact.index.ModuleIndex
    kind: typing.Literal["moduleIndex"] = "moduleIndex"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadProgramIndex:
    """Index for one program profile."""

    program_index: destack._generated.artifact.index.ProgramIndex
    kind: typing.Literal["programIndex"] = "programIndex"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadScript:
    """One structured linker input for one target."""

    script: destack._generated.artifact.script.Script
    kind: typing.Literal["script"] = "script"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadObject:
    """One compiled-code linker input for one target."""

    object: destack._generated.artifact.object.Object
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadAsset:
    """One opaque linker input for one target."""

    asset: destack._generated.artifact.asset.Asset
    kind: typing.Literal["asset"] = "asset"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadBuild:
    """Target-built toolchain payload."""

    build: destack._generated.artifact.build.Build
    kind: typing.Literal["build"] = "build"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadBundle:
    """Linked file graph for one package target."""

    bundle: destack._generated.artifact.bundle.Bundle
    kind: typing.Literal["bundle"] = "bundle"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadProgram:
    """Program for one package target."""

    program: destack._generated.program.model.Program
    kind: typing.Literal["program"] = "program"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadProduct:
    """Linked product assembled from configured target artifacts."""

    product: destack._generated.artifact.product.Product
    kind: typing.Literal["product"] = "product"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadModuleLinted:
    """Realized lint diagnostics for one module profile."""

    module_linted: destack._generated.artifact.lint.ModuleLinted
    kind: typing.Literal["moduleLinted"] = "moduleLinted"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadPackageLinted:
    """Realized lint diagnostics for one package."""

    package_linted: destack._generated.artifact.lint.PackageLinted
    kind: typing.Literal["packageLinted"] = "packageLinted"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactPayloadWorkspaceLinted:
    """Realized lint diagnostics for the workspace."""

    workspace_linted: destack._generated.artifact.lint.WorkspaceLinted
    kind: typing.Literal["workspaceLinted"] = "workspaceLinted"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One typed artifact payload."""
ArtifactPayload: typing.TypeAlias = (
    ArtifactPayloadDirParsed
    | ArtifactPayloadData
    | ArtifactPayloadGlobalEnvironment
    | ArtifactPayloadPackageIndex
    | ArtifactPayloadComponentGraph
    | ArtifactPayloadProgramAnalysis
    | ArtifactPayloadDirBound
    | ArtifactPayloadDirImported
    | ArtifactPayloadDirExpanded
    | ArtifactPayloadDirExported
    | ArtifactPayloadDirResolved
    | ArtifactPayloadDirCheckedComponent
    | ArtifactPayloadDirChecked
    | ArtifactPayloadDirMaterialized
    | ArtifactPayloadMirLowered
    | ArtifactPayloadMirVerified
    | ArtifactPayloadMirAnalyzed
    | ArtifactPayloadMirOptimized
    | ArtifactPayloadModuleIndex
    | ArtifactPayloadProgramIndex
    | ArtifactPayloadScript
    | ArtifactPayloadObject
    | ArtifactPayloadAsset
    | ArtifactPayloadBuild
    | ArtifactPayloadBundle
    | ArtifactPayloadProgram
    | ArtifactPayloadProduct
    | ArtifactPayloadModuleLinted
    | ArtifactPayloadPackageLinted
    | ArtifactPayloadWorkspaceLinted
)

def encode_artifact_payload(writer: BinaryWriter, value: ArtifactPayload) -> None: ...
def decode_artifact_payload(reader: BinaryReader) -> ArtifactPayload: ...
def to_json_artifact_payload(value: ArtifactPayload) -> Json: ...
def from_json_artifact_payload(value: Json) -> ArtifactPayload: ...

__all__ = [
    "ArtifactPayload",
    "encode_artifact_payload",
    "decode_artifact_payload",
    "to_json_artifact_payload",
    "from_json_artifact_payload",
    "ArtifactPayloadDirParsed",
    "ArtifactPayloadData",
    "ArtifactPayloadGlobalEnvironment",
    "ArtifactPayloadPackageIndex",
    "ArtifactPayloadComponentGraph",
    "ArtifactPayloadProgramAnalysis",
    "ArtifactPayloadDirBound",
    "ArtifactPayloadDirImported",
    "ArtifactPayloadDirExpanded",
    "ArtifactPayloadDirExported",
    "ArtifactPayloadDirResolved",
    "ArtifactPayloadDirCheckedComponent",
    "ArtifactPayloadDirChecked",
    "ArtifactPayloadDirMaterialized",
    "ArtifactPayloadMirLowered",
    "ArtifactPayloadMirVerified",
    "ArtifactPayloadMirAnalyzed",
    "ArtifactPayloadMirOptimized",
    "ArtifactPayloadModuleIndex",
    "ArtifactPayloadProgramIndex",
    "ArtifactPayloadScript",
    "ArtifactPayloadObject",
    "ArtifactPayloadAsset",
    "ArtifactPayloadBuild",
    "ArtifactPayloadBundle",
    "ArtifactPayloadProgram",
    "ArtifactPayloadProduct",
    "ArtifactPayloadModuleLinted",
    "ArtifactPayloadPackageLinted",
    "ArtifactPayloadWorkspaceLinted",
]
