# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

import destack._generated.source.component
import destack._generated.source.module
import destack._generated.source.package
import destack._generated.source.product
import destack._generated.source.profile
import destack._generated.source.target

if TYPE_CHECKING:
    from destack._generated.source.component import (
        ComponentId,
    )

    from destack._generated.source.module import (
        ModuleId,
    )

    from destack._generated.source.package import (
        PackageId,
    )

    from destack._generated.source.product import (
        ProductId,
    )

    from destack._generated.source.profile import (
        ProfileId,
    )

    from destack._generated.source.target import (
        TargetId,
    )

@dataclass(frozen=True, slots=True)
class ArtifactKeyBuild:
    """Toolchain build payload for one target."""

    """Build target."""
    target: TargetId
    kind: Literal["build"] = "build"

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirParsed:
    """Parsed module DIR."""

    """Source module."""
    module: ModuleId
    kind: Literal["dirParsed"] = "dirParsed"

@dataclass(frozen=True, slots=True)
class ArtifactKeyData:
    """Parsed non-code module data."""

    """Source module."""
    module: ModuleId
    kind: Literal["data"] = "data"

@dataclass(frozen=True, slots=True)
class ArtifactKeyGlobalEnvironment:
    """Explicit global environment for one profile."""

    """Semantic profile."""
    profile: ProfileId
    kind: Literal["globalEnvironment"] = "globalEnvironment"

@dataclass(frozen=True, slots=True)
class ArtifactKeyPackageIndex:
    """Active dependency index for one profile."""

    """Semantic profile."""
    profile: ProfileId
    kind: Literal["packageIndex"] = "packageIndex"

@dataclass(frozen=True, slots=True)
class ArtifactKeyComponentGraph:
    """Component partition for one profile."""

    """Semantic profile."""
    profile: ProfileId
    kind: Literal["componentGraph"] = "componentGraph"

@dataclass(frozen=True, slots=True)
class ArtifactKeyProgramAnalysis:
    """Whole-program analysis for one profile and target."""

    """Semantic profile."""
    profile: ProfileId
    """Build target."""
    target: TargetId
    kind: Literal["programAnalysis"] = "programAnalysis"

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirBound:
    """Bound DIR."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    kind: Literal["dirBound"] = "dirBound"

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirImported:
    """Imported DIR."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    kind: Literal["dirImported"] = "dirImported"

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirExpanded:
    """Expanded DIR."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    kind: Literal["dirExpanded"] = "dirExpanded"

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirExported:
    """Exported DIR."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    kind: Literal["dirExported"] = "dirExported"

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirResolved:
    """Resolved DIR imports."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    kind: Literal["dirResolved"] = "dirResolved"

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirCheckedComponent:
    """Checked DIR component."""

    """Component entry module."""
    entry: ModuleId
    """Checked component id."""
    component: ComponentId
    """Semantic profile."""
    profile: ProfileId
    kind: Literal["dirCheckedComponent"] = "dirCheckedComponent"

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirChecked:
    """Checked DIR facade."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    kind: Literal["dirChecked"] = "dirChecked"

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirMaterialized:
    """Materialized DIR."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    kind: Literal["dirMaterialized"] = "dirMaterialized"

@dataclass(frozen=True, slots=True)
class ArtifactKeyDirElaborated:
    """Elaborated DIR."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    kind: Literal["dirElaborated"] = "dirElaborated"

@dataclass(frozen=True, slots=True)
class ArtifactKeyMirLowered:
    """Lowered MIR before optimization."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    """Build target."""
    target: TargetId
    kind: Literal["mirLowered"] = "mirLowered"

@dataclass(frozen=True, slots=True)
class ArtifactKeyMirVerified:
    """Verified MIR after required semantic verification."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    """Build target."""
    target: TargetId
    kind: Literal["mirVerified"] = "mirVerified"

@dataclass(frozen=True, slots=True)
class ArtifactKeyMirAnalyzed:
    """Per-module link summary for whole-program analysis."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    """Build target."""
    target: TargetId
    kind: Literal["mirAnalyzed"] = "mirAnalyzed"

@dataclass(frozen=True, slots=True)
class ArtifactKeyMirOptimized:
    """Optimized MIR."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    """Build target."""
    target: TargetId
    kind: Literal["mirOptimized"] = "mirOptimized"

@dataclass(frozen=True, slots=True)
class ArtifactKeyModuleQueryIndex:
    """Query index for one module profile."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    kind: Literal["moduleQueryIndex"] = "moduleQueryIndex"

@dataclass(frozen=True, slots=True)
class ArtifactKeyWorkspaceQueryIndex:
    """Query index for one workspace profile."""

    """Semantic profile."""
    profile: ProfileId
    kind: Literal["workspaceQueryIndex"] = "workspaceQueryIndex"

@dataclass(frozen=True, slots=True)
class ArtifactKeyScript:
    """One structured linker input for one target."""

    """Source module."""
    module: ModuleId
    """Build target."""
    target: TargetId
    kind: Literal["script"] = "script"

@dataclass(frozen=True, slots=True)
class ArtifactKeyObject:
    """One compiled-code linker input for one target."""

    """Source module."""
    module: ModuleId
    """Build target."""
    target: TargetId
    kind: Literal["object"] = "object"

@dataclass(frozen=True, slots=True)
class ArtifactKeyAsset:
    """One opaque linker input for one target."""

    """Source module."""
    module: ModuleId
    """Build target."""
    target: TargetId
    kind: Literal["asset"] = "asset"

@dataclass(frozen=True, slots=True)
class ArtifactKeyBundle:
    """Linked file graph for one package target."""

    """Source package."""
    package: PackageId
    """Build target."""
    target: TargetId
    kind: Literal["bundle"] = "bundle"

@dataclass(frozen=True, slots=True)
class ArtifactKeyProgram:
    """Executable program for one package target."""

    """Source package."""
    package: PackageId
    """Build target."""
    target: TargetId
    kind: Literal["program"] = "program"

@dataclass(frozen=True, slots=True)
class ArtifactKeyProduct:
    """Linked product assembled from configured target artifacts."""

    """Source package."""
    package: PackageId
    """Product."""
    product: ProductId
    kind: Literal["product"] = "product"

@dataclass(frozen=True, slots=True)
class ArtifactKeyModuleLinted:
    """Realized lint diagnostics for one module profile."""

    """Source module."""
    module: ModuleId
    """Semantic profile."""
    profile: ProfileId
    kind: Literal["moduleLinted"] = "moduleLinted"

@dataclass(frozen=True, slots=True)
class ArtifactKeyPackageLinted:
    """Realized lint diagnostics for one package."""

    """Source package."""
    package: PackageId
    kind: Literal["packageLinted"] = "packageLinted"

@dataclass(frozen=True, slots=True)
class ArtifactKeyWorkspaceLinted:
    """Realized lint diagnostics for the workspace."""

    kind: Literal["workspaceLinted"] = "workspaceLinted"

"""External artifact key crossing bridge boundaries."""
ArtifactKey: TypeAlias = (
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

def encode_artifact_key(writer: Writer, value: ArtifactKey) -> None: ...
def decode_artifact_key(reader: Reader) -> ArtifactKey: ...

__all__ = [
    "ArtifactKey",
    "encode_artifact_key",
    "decode_artifact_key",
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
