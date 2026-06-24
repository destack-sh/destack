# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.component
import destack._generated.source.file.model.module
import destack._generated.source.file.model.package
import destack._generated.source.file.model.product
import destack._generated.source.file.model.profile
import destack._generated.source.file.model.target

@dataclass(frozen=True, slots=True)
class ArtifactKeyBuild:
    """Toolchain build payload for one target."""

    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["build"] = "build"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirParsed:
    """Parsed module DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["dirParsed"] = "dirParsed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyData:
    """Parsed non-code module data."""

    module: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["data"] = "data"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyGlobalEnvironment:
    """Explicit global environment for one profile."""

    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["globalEnvironment"] = "globalEnvironment"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyPackageIndex:
    """Active dependency index for one profile."""

    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["packageIndex"] = "packageIndex"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyComponentGraph:
    """Strongly connected component partition for one profile."""

    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["componentGraph"] = "componentGraph"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyProgramAnalysis:
    """Whole-program analysis for one profile and target."""

    profile: destack._generated.source.file.model.profile.ProfileId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["programAnalysis"] = "programAnalysis"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirBound:
    """Bound DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirBound"] = "dirBound"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirImported:
    """Imported DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirImported"] = "dirImported"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirExpanded:
    """Expanded DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirExpanded"] = "dirExpanded"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirExported:
    """Exported DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirExported"] = "dirExported"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirResolved:
    """Resolved DIR imports."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirResolved"] = "dirResolved"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirCheckedComponent:
    """Checked DIR component."""

    entry: destack._generated.source.file.model.module.ModuleId
    component: destack._generated.source.file.model.component.ComponentId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirCheckedComponent"] = "dirCheckedComponent"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirChecked:
    """Checked DIR facade."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirChecked"] = "dirChecked"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirMaterialized:
    """Materialized DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirMaterialized"] = "dirMaterialized"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirElaborated:
    """Elaborated DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirElaborated"] = "dirElaborated"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyMirLowered:
    """Lowered MIR before optimization."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["mirLowered"] = "mirLowered"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyMirVerified:
    """Verified MIR after required semantic verification."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["mirVerified"] = "mirVerified"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyMirAnalyzed:
    """Per-module link summary for whole-program analysis."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["mirAnalyzed"] = "mirAnalyzed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyMirOptimized:
    """Optimized MIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["mirOptimized"] = "mirOptimized"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyModuleQueryIndex:
    """Query index for one module profile."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["moduleQueryIndex"] = "moduleQueryIndex"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyWorkspaceQueryIndex:
    """Query index for one workspace profile."""

    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["workspaceQueryIndex"] = "workspaceQueryIndex"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyScript:
    """One structured linker input for one target."""

    module: destack._generated.source.file.model.module.ModuleId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["script"] = "script"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyObject:
    """One compiled-code linker input for one target."""

    module: destack._generated.source.file.model.module.ModuleId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyAsset:
    """One opaque linker input for one target."""

    module: destack._generated.source.file.model.module.ModuleId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["asset"] = "asset"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyBundle:
    """Linked file graph for one package target."""

    package: destack._generated.source.file.model.package.PackageId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["bundle"] = "bundle"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyProgram:
    """Program for one package target."""

    package: destack._generated.source.file.model.package.PackageId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["program"] = "program"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyProduct:
    """Linked product assembled from configured target artifacts."""

    package: destack._generated.source.file.model.package.PackageId
    product: destack._generated.source.file.model.product.ProductId
    kind: typing.Literal["product"] = "product"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyModuleLinted:
    """Realized lint diagnostics for one module profile."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["moduleLinted"] = "moduleLinted"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyPackageLinted:
    """Realized lint diagnostics for one package."""

    package: destack._generated.source.file.model.package.PackageId
    kind: typing.Literal["packageLinted"] = "packageLinted"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactKeyWorkspaceLinted:
    """Realized lint diagnostics for the workspace."""

    kind: typing.Literal["workspaceLinted"] = "workspaceLinted"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Semantic artifact identity."""
ArtifactKey: typing.TypeAlias = (
    ArtifactKeyBuild
    | ArtifactKeyDirParsed
    | ArtifactKeyData
    | ArtifactKeyGlobalEnvironment
    | ArtifactKeyPackageIndex
    | ArtifactKeyComponentGraph
    | ArtifactKeyProgramAnalysis
    | ArtifactKeyDirBound
    | ArtifactKeyDirImported
    | ArtifactKeyDirExpanded
    | ArtifactKeyDirExported
    | ArtifactKeyDirResolved
    | ArtifactKeyDirCheckedComponent
    | ArtifactKeyDirChecked
    | ArtifactKeyDirMaterialized
    | ArtifactKeyDirElaborated
    | ArtifactKeyMirLowered
    | ArtifactKeyMirVerified
    | ArtifactKeyMirAnalyzed
    | ArtifactKeyMirOptimized
    | ArtifactKeyModuleQueryIndex
    | ArtifactKeyWorkspaceQueryIndex
    | ArtifactKeyScript
    | ArtifactKeyObject
    | ArtifactKeyAsset
    | ArtifactKeyBundle
    | ArtifactKeyProgram
    | ArtifactKeyProduct
    | ArtifactKeyModuleLinted
    | ArtifactKeyPackageLinted
    | ArtifactKeyWorkspaceLinted
)

def encode_artifact_key(writer: BinaryWriter, value: ArtifactKey) -> None: ...
def decode_artifact_key(reader: BinaryReader) -> ArtifactKey: ...
def to_json_artifact_key(value: ArtifactKey) -> Json: ...
def from_json_artifact_key(value: Json) -> ArtifactKey: ...

__all__ = [
    "ArtifactKey",
    "encode_artifact_key",
    "decode_artifact_key",
    "to_json_artifact_key",
    "from_json_artifact_key",
    "ArtifactKeyBuild",
    "ArtifactKeyDirParsed",
    "ArtifactKeyData",
    "ArtifactKeyGlobalEnvironment",
    "ArtifactKeyPackageIndex",
    "ArtifactKeyComponentGraph",
    "ArtifactKeyProgramAnalysis",
    "ArtifactKeyDirBound",
    "ArtifactKeyDirImported",
    "ArtifactKeyDirExpanded",
    "ArtifactKeyDirExported",
    "ArtifactKeyDirResolved",
    "ArtifactKeyDirCheckedComponent",
    "ArtifactKeyDirChecked",
    "ArtifactKeyDirMaterialized",
    "ArtifactKeyDirElaborated",
    "ArtifactKeyMirLowered",
    "ArtifactKeyMirVerified",
    "ArtifactKeyMirAnalyzed",
    "ArtifactKeyMirOptimized",
    "ArtifactKeyModuleQueryIndex",
    "ArtifactKeyWorkspaceQueryIndex",
    "ArtifactKeyScript",
    "ArtifactKeyObject",
    "ArtifactKeyAsset",
    "ArtifactKeyBundle",
    "ArtifactKeyProgram",
    "ArtifactKeyProduct",
    "ArtifactKeyModuleLinted",
    "ArtifactKeyPackageLinted",
    "ArtifactKeyWorkspaceLinted",
]
