# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.file.model.component
import destack._generated.protocol.source.file.model.module
import destack._generated.protocol.source.file.model.package
import destack._generated.protocol.source.file.model.product
import destack._generated.protocol.source.file.model.profile
import destack._generated.protocol.source.file.model.target

if TYPE_CHECKING:
    from destack._generated.protocol.source.file.model.component import (
        ComponentId,
    )

    from destack._generated.protocol.source.file.model.module import (
        ModuleId,
    )

    from destack._generated.protocol.source.file.model.package import (
        PackageId,
    )

    from destack._generated.protocol.source.file.model.product import (
        ProductId,
    )

    from destack._generated.protocol.source.file.model.profile import (
        ProfileId,
    )

    from destack._generated.protocol.source.file.model.target import (
        TargetId,
    )


@dataclass(frozen=True, slots=True)
class ArtifactKeyBuild:
    """Toolchain build payload for one target."""

    target: TargetId
    kind: Literal["build"] = "build"


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirParsed:
    """Parsed module DIR."""

    module: ModuleId
    kind: Literal["dirParsed"] = "dirParsed"


@dataclass(frozen=True, slots=True)
class ArtifactKeyData:
    """Parsed non-code module data."""

    module: ModuleId
    kind: Literal["data"] = "data"


@dataclass(frozen=True, slots=True)
class ArtifactKeyGlobalEnvironment:
    """Explicit global environment for one profile."""

    profile: ProfileId
    kind: Literal["globalEnvironment"] = "globalEnvironment"


@dataclass(frozen=True, slots=True)
class ArtifactKeyPackageIndex:
    """Active dependency index for one profile."""

    profile: ProfileId
    kind: Literal["packageIndex"] = "packageIndex"


@dataclass(frozen=True, slots=True)
class ArtifactKeyComponentGraph:
    """Strongly connected component partition for one profile."""

    profile: ProfileId
    kind: Literal["componentGraph"] = "componentGraph"


@dataclass(frozen=True, slots=True)
class ArtifactKeyProgramAnalysis:
    """Whole-program analysis for one profile and target."""

    profile: ProfileId
    target: TargetId
    kind: Literal["programAnalysis"] = "programAnalysis"


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirBound:
    """Bound DIR."""

    module: ModuleId
    profile: ProfileId
    kind: Literal["dirBound"] = "dirBound"


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirImported:
    """Imported DIR."""

    module: ModuleId
    profile: ProfileId
    kind: Literal["dirImported"] = "dirImported"


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirExpanded:
    """Expanded DIR."""

    module: ModuleId
    profile: ProfileId
    kind: Literal["dirExpanded"] = "dirExpanded"


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirExported:
    """Exported DIR."""

    module: ModuleId
    profile: ProfileId
    kind: Literal["dirExported"] = "dirExported"


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirResolved:
    """Resolved DIR imports."""

    module: ModuleId
    profile: ProfileId
    kind: Literal["dirResolved"] = "dirResolved"


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirCheckedComponent:
    """Checked DIR component."""

    entry: ModuleId
    component: ComponentId
    profile: ProfileId
    kind: Literal["dirCheckedComponent"] = "dirCheckedComponent"


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirChecked:
    """Checked DIR facade."""

    module: ModuleId
    profile: ProfileId
    kind: Literal["dirChecked"] = "dirChecked"


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirMaterialized:
    """Materialized DIR."""

    module: ModuleId
    profile: ProfileId
    kind: Literal["dirMaterialized"] = "dirMaterialized"


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirElaborated:
    """Elaborated DIR."""

    module: ModuleId
    profile: ProfileId
    kind: Literal["dirElaborated"] = "dirElaborated"


@dataclass(frozen=True, slots=True)
class ArtifactKeyMirLowered:
    """Lowered MIR before optimization."""

    module: ModuleId
    profile: ProfileId
    target: TargetId
    kind: Literal["mirLowered"] = "mirLowered"


@dataclass(frozen=True, slots=True)
class ArtifactKeyMirVerified:
    """Verified MIR after required semantic verification."""

    module: ModuleId
    profile: ProfileId
    target: TargetId
    kind: Literal["mirVerified"] = "mirVerified"


@dataclass(frozen=True, slots=True)
class ArtifactKeyMirAnalyzed:
    """Per-module link summary for whole-program analysis."""

    module: ModuleId
    profile: ProfileId
    target: TargetId
    kind: Literal["mirAnalyzed"] = "mirAnalyzed"


@dataclass(frozen=True, slots=True)
class ArtifactKeyMirOptimized:
    """Optimized MIR."""

    module: ModuleId
    profile: ProfileId
    target: TargetId
    kind: Literal["mirOptimized"] = "mirOptimized"


@dataclass(frozen=True, slots=True)
class ArtifactKeyModuleQueryIndex:
    """Query index for one module profile."""

    module: ModuleId
    profile: ProfileId
    kind: Literal["moduleQueryIndex"] = "moduleQueryIndex"


@dataclass(frozen=True, slots=True)
class ArtifactKeyWorkspaceQueryIndex:
    """Query index for one workspace profile."""

    profile: ProfileId
    kind: Literal["workspaceQueryIndex"] = "workspaceQueryIndex"


@dataclass(frozen=True, slots=True)
class ArtifactKeyScript:
    """One structured linker input for one target."""

    module: ModuleId
    target: TargetId
    kind: Literal["script"] = "script"


@dataclass(frozen=True, slots=True)
class ArtifactKeyObject:
    """One compiled-code linker input for one target."""

    module: ModuleId
    target: TargetId
    kind: Literal["object"] = "object"


@dataclass(frozen=True, slots=True)
class ArtifactKeyAsset:
    """One opaque linker input for one target."""

    module: ModuleId
    target: TargetId
    kind: Literal["asset"] = "asset"


@dataclass(frozen=True, slots=True)
class ArtifactKeyBundle:
    """Linked file graph for one package target."""

    package: PackageId
    target: TargetId
    kind: Literal["bundle"] = "bundle"


@dataclass(frozen=True, slots=True)
class ArtifactKeyProgram:
    """Program for one package target."""

    package: PackageId
    target: TargetId
    kind: Literal["program"] = "program"


@dataclass(frozen=True, slots=True)
class ArtifactKeyProduct:
    """Linked product assembled from configured target artifacts."""

    package: PackageId
    product: ProductId
    kind: Literal["product"] = "product"


@dataclass(frozen=True, slots=True)
class ArtifactKeyModuleLinted:
    """Realized lint diagnostics for one module profile."""

    module: ModuleId
    profile: ProfileId
    kind: Literal["moduleLinted"] = "moduleLinted"


@dataclass(frozen=True, slots=True)
class ArtifactKeyPackageLinted:
    """Realized lint diagnostics for one package."""

    package: PackageId
    kind: Literal["packageLinted"] = "packageLinted"


@dataclass(frozen=True, slots=True)
class ArtifactKeyWorkspaceLinted:
    """Realized lint diagnostics for the workspace."""

    kind: Literal["workspaceLinted"] = "workspaceLinted"


"""Semantic artifact identity."""
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


def encode_artifact_key(writer: Writer, value: ArtifactKey) -> None:
    if value.kind == "build":
        writer.write_unsigned(0)
        destack._generated.protocol.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "dirParsed":
        writer.write_unsigned(1)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
    elif value.kind == "data":
        writer.write_unsigned(2)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
    elif value.kind == "globalEnvironment":
        writer.write_unsigned(3)
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "packageIndex":
        writer.write_unsigned(4)
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "componentGraph":
        writer.write_unsigned(5)
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "programAnalysis":
        writer.write_unsigned(6)
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
        destack._generated.protocol.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "dirBound":
        writer.write_unsigned(7)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirImported":
        writer.write_unsigned(8)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirExpanded":
        writer.write_unsigned(9)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirExported":
        writer.write_unsigned(10)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirResolved":
        writer.write_unsigned(11)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirCheckedComponent":
        writer.write_unsigned(12)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.entry
        )
        destack._generated.protocol.source.file.model.component.encode_component_id(
            writer, value.component
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirChecked":
        writer.write_unsigned(13)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirMaterialized":
        writer.write_unsigned(14)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirElaborated":
        writer.write_unsigned(15)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "mirLowered":
        writer.write_unsigned(16)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
        destack._generated.protocol.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "mirVerified":
        writer.write_unsigned(17)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
        destack._generated.protocol.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "mirAnalyzed":
        writer.write_unsigned(18)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
        destack._generated.protocol.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "mirOptimized":
        writer.write_unsigned(19)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
        destack._generated.protocol.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "moduleQueryIndex":
        writer.write_unsigned(20)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "workspaceQueryIndex":
        writer.write_unsigned(21)
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "script":
        writer.write_unsigned(22)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "object":
        writer.write_unsigned(23)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "asset":
        writer.write_unsigned(24)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "bundle":
        writer.write_unsigned(25)
        destack._generated.protocol.source.file.model.package.encode_package_id(
            writer, value.package
        )
        destack._generated.protocol.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "program":
        writer.write_unsigned(26)
        destack._generated.protocol.source.file.model.package.encode_package_id(
            writer, value.package
        )
        destack._generated.protocol.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "product":
        writer.write_unsigned(27)
        destack._generated.protocol.source.file.model.package.encode_package_id(
            writer, value.package
        )
        destack._generated.protocol.source.file.model.product.encode_product_id(
            writer, value.product
        )
    elif value.kind == "moduleLinted":
        writer.write_unsigned(28)
        destack._generated.protocol.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "packageLinted":
        writer.write_unsigned(29)
        destack._generated.protocol.source.file.model.package.encode_package_id(
            writer, value.package
        )
    elif value.kind == "workspaceLinted":
        writer.write_unsigned(30)
    else:
        raise SerdeError("unknown enum variant")


def decode_artifact_key(reader: Reader) -> ArtifactKey:
    variant = reader.read_number()

    if variant == 0:
        field_0 = destack._generated.protocol.source.file.model.target.decode_target_id(
            reader
        )

        return ArtifactKeyBuild(
            target=field_0,
        )
    elif variant == 1:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )

        return ArtifactKeyDirParsed(
            module=field_0,
        )
    elif variant == 2:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )

        return ArtifactKeyData(
            module=field_0,
        )
    elif variant == 3:
        field_0 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyGlobalEnvironment(
            profile=field_0,
        )
    elif variant == 4:
        field_0 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyPackageIndex(
            profile=field_0,
        )
    elif variant == 5:
        field_0 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyComponentGraph(
            profile=field_0,
        )
    elif variant == 6:
        field_0 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )
        field_1 = destack._generated.protocol.source.file.model.target.decode_target_id(
            reader
        )

        return ArtifactKeyProgramAnalysis(
            profile=field_0,
            target=field_1,
        )
    elif variant == 7:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyDirBound(
            module=field_0,
            profile=field_1,
        )
    elif variant == 8:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyDirImported(
            module=field_0,
            profile=field_1,
        )
    elif variant == 9:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyDirExpanded(
            module=field_0,
            profile=field_1,
        )
    elif variant == 10:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyDirExported(
            module=field_0,
            profile=field_1,
        )
    elif variant == 11:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyDirResolved(
            module=field_0,
            profile=field_1,
        )
    elif variant == 12:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.component.decode_component_id(
                reader
            )
        )
        field_2 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyDirCheckedComponent(
            entry=field_0,
            component=field_1,
            profile=field_2,
        )
    elif variant == 13:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyDirChecked(
            module=field_0,
            profile=field_1,
        )
    elif variant == 14:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyDirMaterialized(
            module=field_0,
            profile=field_1,
        )
    elif variant == 15:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyDirElaborated(
            module=field_0,
            profile=field_1,
        )
    elif variant == 16:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )
        field_2 = destack._generated.protocol.source.file.model.target.decode_target_id(
            reader
        )

        return ArtifactKeyMirLowered(
            module=field_0,
            profile=field_1,
            target=field_2,
        )
    elif variant == 17:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )
        field_2 = destack._generated.protocol.source.file.model.target.decode_target_id(
            reader
        )

        return ArtifactKeyMirVerified(
            module=field_0,
            profile=field_1,
            target=field_2,
        )
    elif variant == 18:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )
        field_2 = destack._generated.protocol.source.file.model.target.decode_target_id(
            reader
        )

        return ArtifactKeyMirAnalyzed(
            module=field_0,
            profile=field_1,
            target=field_2,
        )
    elif variant == 19:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )
        field_2 = destack._generated.protocol.source.file.model.target.decode_target_id(
            reader
        )

        return ArtifactKeyMirOptimized(
            module=field_0,
            profile=field_1,
            target=field_2,
        )
    elif variant == 20:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyModuleQueryIndex(
            module=field_0,
            profile=field_1,
        )
    elif variant == 21:
        field_0 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyWorkspaceQueryIndex(
            profile=field_0,
        )
    elif variant == 22:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = destack._generated.protocol.source.file.model.target.decode_target_id(
            reader
        )

        return ArtifactKeyScript(
            module=field_0,
            target=field_1,
        )
    elif variant == 23:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = destack._generated.protocol.source.file.model.target.decode_target_id(
            reader
        )

        return ArtifactKeyObject(
            module=field_0,
            target=field_1,
        )
    elif variant == 24:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = destack._generated.protocol.source.file.model.target.decode_target_id(
            reader
        )

        return ArtifactKeyAsset(
            module=field_0,
            target=field_1,
        )
    elif variant == 25:
        field_0 = (
            destack._generated.protocol.source.file.model.package.decode_package_id(
                reader
            )
        )
        field_1 = destack._generated.protocol.source.file.model.target.decode_target_id(
            reader
        )

        return ArtifactKeyBundle(
            package=field_0,
            target=field_1,
        )
    elif variant == 26:
        field_0 = (
            destack._generated.protocol.source.file.model.package.decode_package_id(
                reader
            )
        )
        field_1 = destack._generated.protocol.source.file.model.target.decode_target_id(
            reader
        )

        return ArtifactKeyProgram(
            package=field_0,
            target=field_1,
        )
    elif variant == 27:
        field_0 = (
            destack._generated.protocol.source.file.model.package.decode_package_id(
                reader
            )
        )
        field_1 = (
            destack._generated.protocol.source.file.model.product.decode_product_id(
                reader
            )
        )

        return ArtifactKeyProduct(
            package=field_0,
            product=field_1,
        )
    elif variant == 28:
        field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
            reader
        )
        field_1 = (
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
        )

        return ArtifactKeyModuleLinted(
            module=field_0,
            profile=field_1,
        )
    elif variant == 29:
        field_0 = (
            destack._generated.protocol.source.file.model.package.decode_package_id(
                reader
            )
        )

        return ArtifactKeyPackageLinted(
            package=field_0,
        )
    elif variant == 30:
        return ArtifactKeyWorkspaceLinted()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


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
