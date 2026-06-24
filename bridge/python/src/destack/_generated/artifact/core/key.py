# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirParsed:
    """Parsed module DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["dirParsed"] = "dirParsed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyData:
    """Parsed non-code module data."""

    module: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["data"] = "data"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyGlobalEnvironment:
    """Explicit global environment for one profile."""

    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["globalEnvironment"] = "globalEnvironment"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyPackageIndex:
    """Active dependency index for one profile."""

    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["packageIndex"] = "packageIndex"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyComponentGraph:
    """Strongly connected component partition for one profile."""

    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["componentGraph"] = "componentGraph"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyProgramAnalysis:
    """Whole-program analysis for one profile and target."""

    profile: destack._generated.source.file.model.profile.ProfileId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["programAnalysis"] = "programAnalysis"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirBound:
    """Bound DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirBound"] = "dirBound"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirImported:
    """Imported DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirImported"] = "dirImported"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirExpanded:
    """Expanded DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirExpanded"] = "dirExpanded"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirExported:
    """Exported DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirExported"] = "dirExported"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirResolved:
    """Resolved DIR imports."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirResolved"] = "dirResolved"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirCheckedComponent:
    """Checked DIR component."""

    entry: destack._generated.source.file.model.module.ModuleId
    component: destack._generated.source.file.model.component.ComponentId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirCheckedComponent"] = "dirCheckedComponent"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirChecked:
    """Checked DIR facade."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirChecked"] = "dirChecked"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirMaterialized:
    """Materialized DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirMaterialized"] = "dirMaterialized"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyDirElaborated:
    """Elaborated DIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["dirElaborated"] = "dirElaborated"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyMirLowered:
    """Lowered MIR before optimization."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["mirLowered"] = "mirLowered"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyMirVerified:
    """Verified MIR after required semantic verification."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["mirVerified"] = "mirVerified"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyMirAnalyzed:
    """Per-module link summary for whole-program analysis."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["mirAnalyzed"] = "mirAnalyzed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyMirOptimized:
    """Optimized MIR."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["mirOptimized"] = "mirOptimized"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyModuleQueryIndex:
    """Query index for one module profile."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["moduleQueryIndex"] = "moduleQueryIndex"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyWorkspaceQueryIndex:
    """Query index for one workspace profile."""

    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["workspaceQueryIndex"] = "workspaceQueryIndex"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyScript:
    """One structured linker input for one target."""

    module: destack._generated.source.file.model.module.ModuleId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["script"] = "script"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyObject:
    """One compiled-code linker input for one target."""

    module: destack._generated.source.file.model.module.ModuleId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyAsset:
    """One opaque linker input for one target."""

    module: destack._generated.source.file.model.module.ModuleId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["asset"] = "asset"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyBundle:
    """Linked file graph for one package target."""

    package: destack._generated.source.file.model.package.PackageId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["bundle"] = "bundle"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyProgram:
    """Program for one package target."""

    package: destack._generated.source.file.model.package.PackageId
    target: destack._generated.source.file.model.target.TargetId
    kind: typing.Literal["program"] = "program"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyProduct:
    """Linked product assembled from configured target artifacts."""

    package: destack._generated.source.file.model.package.PackageId
    product: destack._generated.source.file.model.product.ProductId
    kind: typing.Literal["product"] = "product"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyModuleLinted:
    """Realized lint diagnostics for one module profile."""

    module: destack._generated.source.file.model.module.ModuleId
    profile: destack._generated.source.file.model.profile.ProfileId
    kind: typing.Literal["moduleLinted"] = "moduleLinted"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyPackageLinted:
    """Realized lint diagnostics for one package."""

    package: destack._generated.source.file.model.package.PackageId
    kind: typing.Literal["packageLinted"] = "packageLinted"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactKeyWorkspaceLinted:
    """Realized lint diagnostics for the workspace."""

    kind: typing.Literal["workspaceLinted"] = "workspaceLinted"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_key(self)


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


def encode_artifact_key(writer: BinaryWriter, value: ArtifactKey) -> None:
    """Encode one ArtifactKey."""
    if value.kind == "build":
        writer.write_unsigned(0)
        destack._generated.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "dirParsed":
        writer.write_unsigned(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
    elif value.kind == "data":
        writer.write_unsigned(2)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
    elif value.kind == "globalEnvironment":
        writer.write_unsigned(3)
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "packageIndex":
        writer.write_unsigned(4)
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "componentGraph":
        writer.write_unsigned(5)
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "programAnalysis":
        writer.write_unsigned(6)
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
        destack._generated.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "dirBound":
        writer.write_unsigned(7)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirImported":
        writer.write_unsigned(8)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirExpanded":
        writer.write_unsigned(9)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirExported":
        writer.write_unsigned(10)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirResolved":
        writer.write_unsigned(11)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirCheckedComponent":
        writer.write_unsigned(12)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.entry
        )
        destack._generated.source.file.model.component.encode_component_id(
            writer, value.component
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirChecked":
        writer.write_unsigned(13)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirMaterialized":
        writer.write_unsigned(14)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "dirElaborated":
        writer.write_unsigned(15)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "mirLowered":
        writer.write_unsigned(16)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
        destack._generated.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "mirVerified":
        writer.write_unsigned(17)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
        destack._generated.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "mirAnalyzed":
        writer.write_unsigned(18)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
        destack._generated.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "mirOptimized":
        writer.write_unsigned(19)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
        destack._generated.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "moduleQueryIndex":
        writer.write_unsigned(20)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "workspaceQueryIndex":
        writer.write_unsigned(21)
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "script":
        writer.write_unsigned(22)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "object":
        writer.write_unsigned(23)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "asset":
        writer.write_unsigned(24)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "bundle":
        writer.write_unsigned(25)
        destack._generated.source.file.model.package.encode_package_id(
            writer, value.package
        )
        destack._generated.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "program":
        writer.write_unsigned(26)
        destack._generated.source.file.model.package.encode_package_id(
            writer, value.package
        )
        destack._generated.source.file.model.target.encode_target_id(
            writer, value.target
        )
    elif value.kind == "product":
        writer.write_unsigned(27)
        destack._generated.source.file.model.package.encode_package_id(
            writer, value.package
        )
        destack._generated.source.file.model.product.encode_product_id(
            writer, value.product
        )
    elif value.kind == "moduleLinted":
        writer.write_unsigned(28)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module
        )
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, value.profile
        )
    elif value.kind == "packageLinted":
        writer.write_unsigned(29)
        destack._generated.source.file.model.package.encode_package_id(
            writer, value.package
        )
    elif value.kind == "workspaceLinted":
        writer.write_unsigned(30)
    else:
        raise SerdeError("unknown enum variant")


def decode_artifact_key(reader: BinaryReader) -> ArtifactKey:
    """Decode one ArtifactKey."""
    variant = reader.read_number()

    if variant == 0:
        target = destack._generated.source.file.model.target.decode_target_id(reader)

        return ArtifactKeyBuild(
            target=target,
        )
    elif variant == 1:
        module = destack._generated.source.file.model.module.decode_module_id(reader)

        return ArtifactKeyDirParsed(
            module=module,
        )
    elif variant == 2:
        module = destack._generated.source.file.model.module.decode_module_id(reader)

        return ArtifactKeyData(
            module=module,
        )
    elif variant == 3:
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyGlobalEnvironment(
            profile=profile,
        )
    elif variant == 4:
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyPackageIndex(
            profile=profile,
        )
    elif variant == 5:
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyComponentGraph(
            profile=profile,
        )
    elif variant == 6:
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)
        target = destack._generated.source.file.model.target.decode_target_id(reader)

        return ArtifactKeyProgramAnalysis(
            profile=profile,
            target=target,
        )
    elif variant == 7:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyDirBound(
            module=module,
            profile=profile,
        )
    elif variant == 8:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyDirImported(
            module=module,
            profile=profile,
        )
    elif variant == 9:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyDirExpanded(
            module=module,
            profile=profile,
        )
    elif variant == 10:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyDirExported(
            module=module,
            profile=profile,
        )
    elif variant == 11:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyDirResolved(
            module=module,
            profile=profile,
        )
    elif variant == 12:
        entry = destack._generated.source.file.model.module.decode_module_id(reader)
        component = destack._generated.source.file.model.component.decode_component_id(
            reader
        )
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyDirCheckedComponent(
            entry=entry,
            component=component,
            profile=profile,
        )
    elif variant == 13:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyDirChecked(
            module=module,
            profile=profile,
        )
    elif variant == 14:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyDirMaterialized(
            module=module,
            profile=profile,
        )
    elif variant == 15:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyDirElaborated(
            module=module,
            profile=profile,
        )
    elif variant == 16:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)
        target = destack._generated.source.file.model.target.decode_target_id(reader)

        return ArtifactKeyMirLowered(
            module=module,
            profile=profile,
            target=target,
        )
    elif variant == 17:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)
        target = destack._generated.source.file.model.target.decode_target_id(reader)

        return ArtifactKeyMirVerified(
            module=module,
            profile=profile,
            target=target,
        )
    elif variant == 18:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)
        target = destack._generated.source.file.model.target.decode_target_id(reader)

        return ArtifactKeyMirAnalyzed(
            module=module,
            profile=profile,
            target=target,
        )
    elif variant == 19:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)
        target = destack._generated.source.file.model.target.decode_target_id(reader)

        return ArtifactKeyMirOptimized(
            module=module,
            profile=profile,
            target=target,
        )
    elif variant == 20:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyModuleQueryIndex(
            module=module,
            profile=profile,
        )
    elif variant == 21:
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyWorkspaceQueryIndex(
            profile=profile,
        )
    elif variant == 22:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        target = destack._generated.source.file.model.target.decode_target_id(reader)

        return ArtifactKeyScript(
            module=module,
            target=target,
        )
    elif variant == 23:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        target = destack._generated.source.file.model.target.decode_target_id(reader)

        return ArtifactKeyObject(
            module=module,
            target=target,
        )
    elif variant == 24:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        target = destack._generated.source.file.model.target.decode_target_id(reader)

        return ArtifactKeyAsset(
            module=module,
            target=target,
        )
    elif variant == 25:
        package = destack._generated.source.file.model.package.decode_package_id(reader)
        target = destack._generated.source.file.model.target.decode_target_id(reader)

        return ArtifactKeyBundle(
            package=package,
            target=target,
        )
    elif variant == 26:
        package = destack._generated.source.file.model.package.decode_package_id(reader)
        target = destack._generated.source.file.model.target.decode_target_id(reader)

        return ArtifactKeyProgram(
            package=package,
            target=target,
        )
    elif variant == 27:
        package = destack._generated.source.file.model.package.decode_package_id(reader)
        product = destack._generated.source.file.model.product.decode_product_id(reader)

        return ArtifactKeyProduct(
            package=package,
            product=product,
        )
    elif variant == 28:
        module = destack._generated.source.file.model.module.decode_module_id(reader)
        profile = destack._generated.source.file.model.profile.decode_profile_id(reader)

        return ArtifactKeyModuleLinted(
            module=module,
            profile=profile,
        )
    elif variant == 29:
        package = destack._generated.source.file.model.package.decode_package_id(reader)

        return ArtifactKeyPackageLinted(
            package=package,
        )
    elif variant == 30:
        return ArtifactKeyWorkspaceLinted()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_artifact_key(value: ArtifactKey) -> Json:
    """Return one JSON value for one ArtifactKey."""
    if value.kind == "build":
        return {
            "kind": "build",
            "target": destack._generated.source.file.model.target.to_json_target_id(
                value.target
            ),
        }
    elif value.kind == "dirParsed":
        return {
            "kind": "dirParsed",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
        }
    elif value.kind == "data":
        return {
            "kind": "data",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
        }
    elif value.kind == "globalEnvironment":
        return {
            "kind": "globalEnvironment",
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "packageIndex":
        return {
            "kind": "packageIndex",
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "componentGraph":
        return {
            "kind": "componentGraph",
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "programAnalysis":
        return {
            "kind": "programAnalysis",
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
            "target": destack._generated.source.file.model.target.to_json_target_id(
                value.target
            ),
        }
    elif value.kind == "dirBound":
        return {
            "kind": "dirBound",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "dirImported":
        return {
            "kind": "dirImported",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "dirExpanded":
        return {
            "kind": "dirExpanded",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "dirExported":
        return {
            "kind": "dirExported",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "dirResolved":
        return {
            "kind": "dirResolved",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "dirCheckedComponent":
        return {
            "kind": "dirCheckedComponent",
            "entry": destack._generated.source.file.model.module.to_json_module_id(
                value.entry
            ),
            "component": destack._generated.source.file.model.component.to_json_component_id(
                value.component
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "dirChecked":
        return {
            "kind": "dirChecked",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "dirMaterialized":
        return {
            "kind": "dirMaterialized",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "dirElaborated":
        return {
            "kind": "dirElaborated",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "mirLowered":
        return {
            "kind": "mirLowered",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
            "target": destack._generated.source.file.model.target.to_json_target_id(
                value.target
            ),
        }
    elif value.kind == "mirVerified":
        return {
            "kind": "mirVerified",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
            "target": destack._generated.source.file.model.target.to_json_target_id(
                value.target
            ),
        }
    elif value.kind == "mirAnalyzed":
        return {
            "kind": "mirAnalyzed",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
            "target": destack._generated.source.file.model.target.to_json_target_id(
                value.target
            ),
        }
    elif value.kind == "mirOptimized":
        return {
            "kind": "mirOptimized",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
            "target": destack._generated.source.file.model.target.to_json_target_id(
                value.target
            ),
        }
    elif value.kind == "moduleQueryIndex":
        return {
            "kind": "moduleQueryIndex",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "workspaceQueryIndex":
        return {
            "kind": "workspaceQueryIndex",
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "script":
        return {
            "kind": "script",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "target": destack._generated.source.file.model.target.to_json_target_id(
                value.target
            ),
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "target": destack._generated.source.file.model.target.to_json_target_id(
                value.target
            ),
        }
    elif value.kind == "asset":
        return {
            "kind": "asset",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "target": destack._generated.source.file.model.target.to_json_target_id(
                value.target
            ),
        }
    elif value.kind == "bundle":
        return {
            "kind": "bundle",
            "package": destack._generated.source.file.model.package.to_json_package_id(
                value.package
            ),
            "target": destack._generated.source.file.model.target.to_json_target_id(
                value.target
            ),
        }
    elif value.kind == "program":
        return {
            "kind": "program",
            "package": destack._generated.source.file.model.package.to_json_package_id(
                value.package
            ),
            "target": destack._generated.source.file.model.target.to_json_target_id(
                value.target
            ),
        }
    elif value.kind == "product":
        return {
            "kind": "product",
            "package": destack._generated.source.file.model.package.to_json_package_id(
                value.package
            ),
            "product": destack._generated.source.file.model.product.to_json_product_id(
                value.product
            ),
        }
    elif value.kind == "moduleLinted":
        return {
            "kind": "moduleLinted",
            "module": destack._generated.source.file.model.module.to_json_module_id(
                value.module
            ),
            "profile": destack._generated.source.file.model.profile.to_json_profile_id(
                value.profile
            ),
        }
    elif value.kind == "packageLinted":
        return {
            "kind": "packageLinted",
            "package": destack._generated.source.file.model.package.to_json_package_id(
                value.package
            ),
        }
    elif value.kind == "workspaceLinted":
        return {
            "kind": "workspaceLinted",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_artifact_key(value: Json) -> ArtifactKey:
    """Return one ArtifactKey from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "build":
        return ArtifactKeyBuild(
            target=destack._generated.source.file.model.target.from_json_target_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "dirParsed":
        return ArtifactKeyDirParsed(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
        )
    elif kind == "data":
        return ArtifactKeyData(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
        )
    elif kind == "globalEnvironment":
        return ArtifactKeyGlobalEnvironment(
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "packageIndex":
        return ArtifactKeyPackageIndex(
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "componentGraph":
        return ArtifactKeyComponentGraph(
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "programAnalysis":
        return ArtifactKeyProgramAnalysis(
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
            target=destack._generated.source.file.model.target.from_json_target_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "dirBound":
        return ArtifactKeyDirBound(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "dirImported":
        return ArtifactKeyDirImported(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "dirExpanded":
        return ArtifactKeyDirExpanded(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "dirExported":
        return ArtifactKeyDirExported(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "dirResolved":
        return ArtifactKeyDirResolved(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "dirCheckedComponent":
        return ArtifactKeyDirCheckedComponent(
            entry=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "entry")
            ),
            component=destack._generated.source.file.model.component.from_json_component_id(
                json_field(object_, "component")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "dirChecked":
        return ArtifactKeyDirChecked(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "dirMaterialized":
        return ArtifactKeyDirMaterialized(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "dirElaborated":
        return ArtifactKeyDirElaborated(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "mirLowered":
        return ArtifactKeyMirLowered(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
            target=destack._generated.source.file.model.target.from_json_target_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "mirVerified":
        return ArtifactKeyMirVerified(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
            target=destack._generated.source.file.model.target.from_json_target_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "mirAnalyzed":
        return ArtifactKeyMirAnalyzed(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
            target=destack._generated.source.file.model.target.from_json_target_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "mirOptimized":
        return ArtifactKeyMirOptimized(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
            target=destack._generated.source.file.model.target.from_json_target_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "moduleQueryIndex":
        return ArtifactKeyModuleQueryIndex(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "workspaceQueryIndex":
        return ArtifactKeyWorkspaceQueryIndex(
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "script":
        return ArtifactKeyScript(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            target=destack._generated.source.file.model.target.from_json_target_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "object":
        return ArtifactKeyObject(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            target=destack._generated.source.file.model.target.from_json_target_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "asset":
        return ArtifactKeyAsset(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            target=destack._generated.source.file.model.target.from_json_target_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "bundle":
        return ArtifactKeyBundle(
            package=destack._generated.source.file.model.package.from_json_package_id(
                json_field(object_, "package")
            ),
            target=destack._generated.source.file.model.target.from_json_target_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "program":
        return ArtifactKeyProgram(
            package=destack._generated.source.file.model.package.from_json_package_id(
                json_field(object_, "package")
            ),
            target=destack._generated.source.file.model.target.from_json_target_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "product":
        return ArtifactKeyProduct(
            package=destack._generated.source.file.model.package.from_json_package_id(
                json_field(object_, "package")
            ),
            product=destack._generated.source.file.model.product.from_json_product_id(
                json_field(object_, "product")
            ),
        )
    elif kind == "moduleLinted":
        return ArtifactKeyModuleLinted(
            module=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module")
            ),
            profile=destack._generated.source.file.model.profile.from_json_profile_id(
                json_field(object_, "profile")
            ),
        )
    elif kind == "packageLinted":
        return ArtifactKeyPackageLinted(
            package=destack._generated.source.file.model.package.from_json_package_id(
                json_field(object_, "package")
            ),
        )
    elif kind == "workspaceLinted":
        return ArtifactKeyWorkspaceLinted()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
