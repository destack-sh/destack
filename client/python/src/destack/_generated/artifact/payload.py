# generated client target, do not edit

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

import destack._generated.artifact.asset
import destack._generated.artifact.build
import destack._generated.artifact.bundle
import destack._generated.artifact.component
import destack._generated.artifact.data
import destack._generated.artifact.dir
import destack._generated.artifact.environment
import destack._generated.artifact.lint
import destack._generated.artifact.mir
import destack._generated.artifact.object
import destack._generated.artifact.package
import destack._generated.artifact.product
import destack._generated.artifact.query
import destack._generated.artifact.script
import destack._generated.program.program


@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirParsed:
    """Parsed module DIR."""

    dir_parsed: destack._generated.artifact.dir.DirParsed
    kind: typing.Literal["dirParsed"] = "dirParsed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadData:
    """Parsed non-code module data."""

    data: destack._generated.artifact.data.Data
    kind: typing.Literal["data"] = "data"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadGlobalEnvironment:
    """Explicit global environment for one profile."""

    global_environment: destack._generated.artifact.environment.GlobalEnvironment
    kind: typing.Literal["globalEnvironment"] = "globalEnvironment"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadPackageIndex:
    """Active dependency index for one profile."""

    package_index: destack._generated.artifact.package.PackageIndex
    kind: typing.Literal["packageIndex"] = "packageIndex"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadComponentGraph:
    """Component partition for one profile."""

    component_graph: destack._generated.artifact.component.ComponentGraph
    kind: typing.Literal["componentGraph"] = "componentGraph"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadProgramAnalysis:
    """Whole-program analysis for one profile and target."""

    program_analysis: destack._generated.artifact.mir.ProgramAnalysis
    kind: typing.Literal["programAnalysis"] = "programAnalysis"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirBound:
    """Bound DIR."""

    dir_bound: destack._generated.artifact.dir.DirBound
    kind: typing.Literal["dirBound"] = "dirBound"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirImported:
    """Imported DIR."""

    dir_imported: destack._generated.artifact.dir.DirImported
    kind: typing.Literal["dirImported"] = "dirImported"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirExpanded:
    """Expanded DIR."""

    dir_expanded: destack._generated.artifact.dir.DirExpanded
    kind: typing.Literal["dirExpanded"] = "dirExpanded"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirExported:
    """Exported DIR."""

    dir_exported: destack._generated.artifact.dir.DirExported
    kind: typing.Literal["dirExported"] = "dirExported"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirResolved:
    """Resolved DIR imports."""

    dir_resolved: destack._generated.artifact.dir.DirResolved
    kind: typing.Literal["dirResolved"] = "dirResolved"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirCheckedComponent:
    """Checked DIR component."""

    dir_checked_component: destack._generated.artifact.dir.DirCheckedComponent
    kind: typing.Literal["dirCheckedComponent"] = "dirCheckedComponent"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirChecked:
    """Checked DIR facade."""

    dir_checked: destack._generated.artifact.dir.DirChecked
    kind: typing.Literal["dirChecked"] = "dirChecked"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirMaterialized:
    """Materialized DIR."""

    dir_materialized: destack._generated.artifact.dir.DirMaterialized
    kind: typing.Literal["dirMaterialized"] = "dirMaterialized"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadDirElaborated:
    """Elaborated DIR."""

    dir_elaborated: destack._generated.artifact.dir.DirElaborated
    kind: typing.Literal["dirElaborated"] = "dirElaborated"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadMirLowered:
    """Lowered MIR before optimization."""

    mir_lowered: destack._generated.artifact.mir.MirLowered
    kind: typing.Literal["mirLowered"] = "mirLowered"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadMirVerified:
    """Verified MIR marker after required semantic verification."""

    mir_verified: destack._generated.artifact.mir.MirVerified
    kind: typing.Literal["mirVerified"] = "mirVerified"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadMirAnalyzed:
    """Per-module link summary for whole-program analysis."""

    mir_analyzed: destack._generated.artifact.mir.MirAnalyzed
    kind: typing.Literal["mirAnalyzed"] = "mirAnalyzed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadMirOptimized:
    """Optimized MIR."""

    mir_optimized: destack._generated.artifact.mir.MirOptimized
    kind: typing.Literal["mirOptimized"] = "mirOptimized"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadModuleQueryIndex:
    """Query index for one module profile."""

    module_query_index: destack._generated.artifact.query.ModuleQueryIndex
    kind: typing.Literal["moduleQueryIndex"] = "moduleQueryIndex"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadWorkspaceQueryIndex:
    """Query index for one workspace profile."""

    workspace_query_index: destack._generated.artifact.query.WorkspaceQueryIndex
    kind: typing.Literal["workspaceQueryIndex"] = "workspaceQueryIndex"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadScript:
    """One structured linker input for one target."""

    script: destack._generated.artifact.script.Script
    kind: typing.Literal["script"] = "script"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadObject:
    """One compiled-code linker input for one target."""

    object: destack._generated.artifact.object.Object
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadAsset:
    """One opaque linker input for one target."""

    asset: destack._generated.artifact.asset.Asset
    kind: typing.Literal["asset"] = "asset"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadBuild:
    """Target-built toolchain payload."""

    build: destack._generated.artifact.build.Build
    kind: typing.Literal["build"] = "build"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadBundle:
    """Linked file graph for one package target."""

    bundle: destack._generated.artifact.bundle.Bundle
    kind: typing.Literal["bundle"] = "bundle"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadProgram:
    """Program for one package target."""

    program: destack._generated.program.program.Program
    kind: typing.Literal["program"] = "program"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadProduct:
    """Linked product assembled from configured target artifacts."""

    product: destack._generated.artifact.product.Product
    kind: typing.Literal["product"] = "product"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadModuleLinted:
    """Realized lint diagnostics for one module profile."""

    module_linted: destack._generated.artifact.lint.ModuleLinted
    kind: typing.Literal["moduleLinted"] = "moduleLinted"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadPackageLinted:
    """Realized lint diagnostics for one package."""

    package_linted: destack._generated.artifact.lint.PackageLinted
    kind: typing.Literal["packageLinted"] = "packageLinted"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


@dataclass(frozen=True, slots=True)
class ArtifactPayloadWorkspaceLinted:
    """Realized lint diagnostics for the workspace."""

    workspace_linted: destack._generated.artifact.lint.WorkspaceLinted
    kind: typing.Literal["workspaceLinted"] = "workspaceLinted"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_payload(self)


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
    | ArtifactPayloadDirElaborated
    | ArtifactPayloadMirLowered
    | ArtifactPayloadMirVerified
    | ArtifactPayloadMirAnalyzed
    | ArtifactPayloadMirOptimized
    | ArtifactPayloadModuleQueryIndex
    | ArtifactPayloadWorkspaceQueryIndex
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


def encode_artifact_payload(writer: BinaryWriter, value: ArtifactPayload) -> None:
    """Encode one ArtifactPayload."""
    if value.kind == "dirParsed":
        writer.write_unsigned(0)
        destack._generated.artifact.dir.encode_dir_parsed(writer, value.dir_parsed)
    elif value.kind == "data":
        writer.write_unsigned(1)
        destack._generated.artifact.data.encode_data(writer, value.data)
    elif value.kind == "globalEnvironment":
        writer.write_unsigned(2)
        destack._generated.artifact.environment.encode_global_environment(
            writer, value.global_environment
        )
    elif value.kind == "packageIndex":
        writer.write_unsigned(3)
        destack._generated.artifact.package.encode_package_index(
            writer, value.package_index
        )
    elif value.kind == "componentGraph":
        writer.write_unsigned(4)
        destack._generated.artifact.component.encode_component_graph(
            writer, value.component_graph
        )
    elif value.kind == "programAnalysis":
        writer.write_unsigned(5)
        destack._generated.artifact.mir.encode_program_analysis(
            writer, value.program_analysis
        )
    elif value.kind == "dirBound":
        writer.write_unsigned(6)
        destack._generated.artifact.dir.encode_dir_bound(writer, value.dir_bound)
    elif value.kind == "dirImported":
        writer.write_unsigned(7)
        destack._generated.artifact.dir.encode_dir_imported(writer, value.dir_imported)
    elif value.kind == "dirExpanded":
        writer.write_unsigned(8)
        destack._generated.artifact.dir.encode_dir_expanded(writer, value.dir_expanded)
    elif value.kind == "dirExported":
        writer.write_unsigned(9)
        destack._generated.artifact.dir.encode_dir_exported(writer, value.dir_exported)
    elif value.kind == "dirResolved":
        writer.write_unsigned(10)
        destack._generated.artifact.dir.encode_dir_resolved(writer, value.dir_resolved)
    elif value.kind == "dirCheckedComponent":
        writer.write_unsigned(11)
        destack._generated.artifact.dir.encode_dir_checked_component(
            writer, value.dir_checked_component
        )
    elif value.kind == "dirChecked":
        writer.write_unsigned(12)
        destack._generated.artifact.dir.encode_dir_checked(writer, value.dir_checked)
    elif value.kind == "dirMaterialized":
        writer.write_unsigned(13)
        destack._generated.artifact.dir.encode_dir_materialized(
            writer, value.dir_materialized
        )
    elif value.kind == "dirElaborated":
        writer.write_unsigned(14)
        destack._generated.artifact.dir.encode_dir_elaborated(
            writer, value.dir_elaborated
        )
    elif value.kind == "mirLowered":
        writer.write_unsigned(15)
        destack._generated.artifact.mir.encode_mir_lowered(writer, value.mir_lowered)
    elif value.kind == "mirVerified":
        writer.write_unsigned(16)
        destack._generated.artifact.mir.encode_mir_verified(writer, value.mir_verified)
    elif value.kind == "mirAnalyzed":
        writer.write_unsigned(17)
        destack._generated.artifact.mir.encode_mir_analyzed(writer, value.mir_analyzed)
    elif value.kind == "mirOptimized":
        writer.write_unsigned(18)
        destack._generated.artifact.mir.encode_mir_optimized(
            writer, value.mir_optimized
        )
    elif value.kind == "moduleQueryIndex":
        writer.write_unsigned(19)
        destack._generated.artifact.query.encode_module_query_index(
            writer, value.module_query_index
        )
    elif value.kind == "workspaceQueryIndex":
        writer.write_unsigned(20)
        destack._generated.artifact.query.encode_workspace_query_index(
            writer, value.workspace_query_index
        )
    elif value.kind == "script":
        writer.write_unsigned(21)
        destack._generated.artifact.script.encode_script(writer, value.script)
    elif value.kind == "object":
        writer.write_unsigned(22)
        destack._generated.artifact.object.encode_object(writer, value.object)
    elif value.kind == "asset":
        writer.write_unsigned(23)
        destack._generated.artifact.asset.encode_asset(writer, value.asset)
    elif value.kind == "build":
        writer.write_unsigned(24)
        destack._generated.artifact.build.encode_build(writer, value.build)
    elif value.kind == "bundle":
        writer.write_unsigned(25)
        destack._generated.artifact.bundle.encode_bundle(writer, value.bundle)
    elif value.kind == "program":
        writer.write_unsigned(26)
        destack._generated.program.program.encode_program(writer, value.program)
    elif value.kind == "product":
        writer.write_unsigned(27)
        destack._generated.artifact.product.encode_product(writer, value.product)
    elif value.kind == "moduleLinted":
        writer.write_unsigned(28)
        destack._generated.artifact.lint.encode_module_linted(
            writer, value.module_linted
        )
    elif value.kind == "packageLinted":
        writer.write_unsigned(29)
        destack._generated.artifact.lint.encode_package_linted(
            writer, value.package_linted
        )
    elif value.kind == "workspaceLinted":
        writer.write_unsigned(30)
        destack._generated.artifact.lint.encode_workspace_linted(
            writer, value.workspace_linted
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_artifact_payload(reader: BinaryReader) -> ArtifactPayload:
    """Decode one ArtifactPayload."""
    variant = reader.read_number()

    if variant == 0:
        dir_parsed = destack._generated.artifact.dir.decode_dir_parsed(reader)

        return ArtifactPayloadDirParsed(dir_parsed=dir_parsed)
    elif variant == 1:
        data = destack._generated.artifact.data.decode_data(reader)

        return ArtifactPayloadData(data=data)
    elif variant == 2:
        global_environment = (
            destack._generated.artifact.environment.decode_global_environment(reader)
        )

        return ArtifactPayloadGlobalEnvironment(global_environment=global_environment)
    elif variant == 3:
        package_index = destack._generated.artifact.package.decode_package_index(reader)

        return ArtifactPayloadPackageIndex(package_index=package_index)
    elif variant == 4:
        component_graph = destack._generated.artifact.component.decode_component_graph(
            reader
        )

        return ArtifactPayloadComponentGraph(component_graph=component_graph)
    elif variant == 5:
        program_analysis = destack._generated.artifact.mir.decode_program_analysis(
            reader
        )

        return ArtifactPayloadProgramAnalysis(program_analysis=program_analysis)
    elif variant == 6:
        dir_bound = destack._generated.artifact.dir.decode_dir_bound(reader)

        return ArtifactPayloadDirBound(dir_bound=dir_bound)
    elif variant == 7:
        dir_imported = destack._generated.artifact.dir.decode_dir_imported(reader)

        return ArtifactPayloadDirImported(dir_imported=dir_imported)
    elif variant == 8:
        dir_expanded = destack._generated.artifact.dir.decode_dir_expanded(reader)

        return ArtifactPayloadDirExpanded(dir_expanded=dir_expanded)
    elif variant == 9:
        dir_exported = destack._generated.artifact.dir.decode_dir_exported(reader)

        return ArtifactPayloadDirExported(dir_exported=dir_exported)
    elif variant == 10:
        dir_resolved = destack._generated.artifact.dir.decode_dir_resolved(reader)

        return ArtifactPayloadDirResolved(dir_resolved=dir_resolved)
    elif variant == 11:
        dir_checked_component = (
            destack._generated.artifact.dir.decode_dir_checked_component(reader)
        )

        return ArtifactPayloadDirCheckedComponent(
            dir_checked_component=dir_checked_component
        )
    elif variant == 12:
        dir_checked = destack._generated.artifact.dir.decode_dir_checked(reader)

        return ArtifactPayloadDirChecked(dir_checked=dir_checked)
    elif variant == 13:
        dir_materialized = destack._generated.artifact.dir.decode_dir_materialized(
            reader
        )

        return ArtifactPayloadDirMaterialized(dir_materialized=dir_materialized)
    elif variant == 14:
        dir_elaborated = destack._generated.artifact.dir.decode_dir_elaborated(reader)

        return ArtifactPayloadDirElaborated(dir_elaborated=dir_elaborated)
    elif variant == 15:
        mir_lowered = destack._generated.artifact.mir.decode_mir_lowered(reader)

        return ArtifactPayloadMirLowered(mir_lowered=mir_lowered)
    elif variant == 16:
        mir_verified = destack._generated.artifact.mir.decode_mir_verified(reader)

        return ArtifactPayloadMirVerified(mir_verified=mir_verified)
    elif variant == 17:
        mir_analyzed = destack._generated.artifact.mir.decode_mir_analyzed(reader)

        return ArtifactPayloadMirAnalyzed(mir_analyzed=mir_analyzed)
    elif variant == 18:
        mir_optimized = destack._generated.artifact.mir.decode_mir_optimized(reader)

        return ArtifactPayloadMirOptimized(mir_optimized=mir_optimized)
    elif variant == 19:
        module_query_index = (
            destack._generated.artifact.query.decode_module_query_index(reader)
        )

        return ArtifactPayloadModuleQueryIndex(module_query_index=module_query_index)
    elif variant == 20:
        workspace_query_index = (
            destack._generated.artifact.query.decode_workspace_query_index(reader)
        )

        return ArtifactPayloadWorkspaceQueryIndex(
            workspace_query_index=workspace_query_index
        )
    elif variant == 21:
        script = destack._generated.artifact.script.decode_script(reader)

        return ArtifactPayloadScript(script=script)
    elif variant == 22:
        object = destack._generated.artifact.object.decode_object(reader)

        return ArtifactPayloadObject(object=object)
    elif variant == 23:
        asset = destack._generated.artifact.asset.decode_asset(reader)

        return ArtifactPayloadAsset(asset=asset)
    elif variant == 24:
        build = destack._generated.artifact.build.decode_build(reader)

        return ArtifactPayloadBuild(build=build)
    elif variant == 25:
        bundle = destack._generated.artifact.bundle.decode_bundle(reader)

        return ArtifactPayloadBundle(bundle=bundle)
    elif variant == 26:
        program = destack._generated.program.program.decode_program(reader)

        return ArtifactPayloadProgram(program=program)
    elif variant == 27:
        product = destack._generated.artifact.product.decode_product(reader)

        return ArtifactPayloadProduct(product=product)
    elif variant == 28:
        module_linted = destack._generated.artifact.lint.decode_module_linted(reader)

        return ArtifactPayloadModuleLinted(module_linted=module_linted)
    elif variant == 29:
        package_linted = destack._generated.artifact.lint.decode_package_linted(reader)

        return ArtifactPayloadPackageLinted(package_linted=package_linted)
    elif variant == 30:
        workspace_linted = destack._generated.artifact.lint.decode_workspace_linted(
            reader
        )

        return ArtifactPayloadWorkspaceLinted(workspace_linted=workspace_linted)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_artifact_payload(value: ArtifactPayload) -> Json:
    """Return one JSON value for one ArtifactPayload."""
    if value.kind == "dirParsed":
        return {
            "kind": "dirParsed",
            "dir_parsed": destack._generated.artifact.dir.to_json_dir_parsed(
                value.dir_parsed
            ),
        }
    elif value.kind == "data":
        return {
            "kind": "data",
            "data": destack._generated.artifact.data.to_json_data(value.data),
        }
    elif value.kind == "globalEnvironment":
        return {
            "kind": "globalEnvironment",
            "global_environment": destack._generated.artifact.environment.to_json_global_environment(
                value.global_environment
            ),
        }
    elif value.kind == "packageIndex":
        return {
            "kind": "packageIndex",
            "package_index": destack._generated.artifact.package.to_json_package_index(
                value.package_index
            ),
        }
    elif value.kind == "componentGraph":
        return {
            "kind": "componentGraph",
            "component_graph": destack._generated.artifact.component.to_json_component_graph(
                value.component_graph
            ),
        }
    elif value.kind == "programAnalysis":
        return {
            "kind": "programAnalysis",
            "program_analysis": destack._generated.artifact.mir.to_json_program_analysis(
                value.program_analysis
            ),
        }
    elif value.kind == "dirBound":
        return {
            "kind": "dirBound",
            "dir_bound": destack._generated.artifact.dir.to_json_dir_bound(
                value.dir_bound
            ),
        }
    elif value.kind == "dirImported":
        return {
            "kind": "dirImported",
            "dir_imported": destack._generated.artifact.dir.to_json_dir_imported(
                value.dir_imported
            ),
        }
    elif value.kind == "dirExpanded":
        return {
            "kind": "dirExpanded",
            "dir_expanded": destack._generated.artifact.dir.to_json_dir_expanded(
                value.dir_expanded
            ),
        }
    elif value.kind == "dirExported":
        return {
            "kind": "dirExported",
            "dir_exported": destack._generated.artifact.dir.to_json_dir_exported(
                value.dir_exported
            ),
        }
    elif value.kind == "dirResolved":
        return {
            "kind": "dirResolved",
            "dir_resolved": destack._generated.artifact.dir.to_json_dir_resolved(
                value.dir_resolved
            ),
        }
    elif value.kind == "dirCheckedComponent":
        return {
            "kind": "dirCheckedComponent",
            "dir_checked_component": destack._generated.artifact.dir.to_json_dir_checked_component(
                value.dir_checked_component
            ),
        }
    elif value.kind == "dirChecked":
        return {
            "kind": "dirChecked",
            "dir_checked": destack._generated.artifact.dir.to_json_dir_checked(
                value.dir_checked
            ),
        }
    elif value.kind == "dirMaterialized":
        return {
            "kind": "dirMaterialized",
            "dir_materialized": destack._generated.artifact.dir.to_json_dir_materialized(
                value.dir_materialized
            ),
        }
    elif value.kind == "dirElaborated":
        return {
            "kind": "dirElaborated",
            "dir_elaborated": destack._generated.artifact.dir.to_json_dir_elaborated(
                value.dir_elaborated
            ),
        }
    elif value.kind == "mirLowered":
        return {
            "kind": "mirLowered",
            "mir_lowered": destack._generated.artifact.mir.to_json_mir_lowered(
                value.mir_lowered
            ),
        }
    elif value.kind == "mirVerified":
        return {
            "kind": "mirVerified",
            "mir_verified": destack._generated.artifact.mir.to_json_mir_verified(
                value.mir_verified
            ),
        }
    elif value.kind == "mirAnalyzed":
        return {
            "kind": "mirAnalyzed",
            "mir_analyzed": destack._generated.artifact.mir.to_json_mir_analyzed(
                value.mir_analyzed
            ),
        }
    elif value.kind == "mirOptimized":
        return {
            "kind": "mirOptimized",
            "mir_optimized": destack._generated.artifact.mir.to_json_mir_optimized(
                value.mir_optimized
            ),
        }
    elif value.kind == "moduleQueryIndex":
        return {
            "kind": "moduleQueryIndex",
            "module_query_index": destack._generated.artifact.query.to_json_module_query_index(
                value.module_query_index
            ),
        }
    elif value.kind == "workspaceQueryIndex":
        return {
            "kind": "workspaceQueryIndex",
            "workspace_query_index": destack._generated.artifact.query.to_json_workspace_query_index(
                value.workspace_query_index
            ),
        }
    elif value.kind == "script":
        return {
            "kind": "script",
            "script": destack._generated.artifact.script.to_json_script(value.script),
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "object": destack._generated.artifact.object.to_json_object(value.object),
        }
    elif value.kind == "asset":
        return {
            "kind": "asset",
            "asset": destack._generated.artifact.asset.to_json_asset(value.asset),
        }
    elif value.kind == "build":
        return {
            "kind": "build",
            "build": destack._generated.artifact.build.to_json_build(value.build),
        }
    elif value.kind == "bundle":
        return {
            "kind": "bundle",
            "bundle": destack._generated.artifact.bundle.to_json_bundle(value.bundle),
        }
    elif value.kind == "program":
        return {
            "kind": "program",
            "program": destack._generated.program.program.to_json_program(
                value.program
            ),
        }
    elif value.kind == "product":
        return {
            "kind": "product",
            "product": destack._generated.artifact.product.to_json_product(
                value.product
            ),
        }
    elif value.kind == "moduleLinted":
        return {
            "kind": "moduleLinted",
            "module_linted": destack._generated.artifact.lint.to_json_module_linted(
                value.module_linted
            ),
        }
    elif value.kind == "packageLinted":
        return {
            "kind": "packageLinted",
            "package_linted": destack._generated.artifact.lint.to_json_package_linted(
                value.package_linted
            ),
        }
    elif value.kind == "workspaceLinted":
        return {
            "kind": "workspaceLinted",
            "workspace_linted": destack._generated.artifact.lint.to_json_workspace_linted(
                value.workspace_linted
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_artifact_payload(value: Json) -> ArtifactPayload:
    """Return one ArtifactPayload from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "dirParsed":
        return ArtifactPayloadDirParsed(
            dir_parsed=destack._generated.artifact.dir.from_json_dir_parsed(
                json_field(object_, "dir_parsed")
            )
        )
    elif kind == "data":
        return ArtifactPayloadData(
            data=destack._generated.artifact.data.from_json_data(
                json_field(object_, "data")
            )
        )
    elif kind == "globalEnvironment":
        return ArtifactPayloadGlobalEnvironment(
            global_environment=destack._generated.artifact.environment.from_json_global_environment(
                json_field(object_, "global_environment")
            )
        )
    elif kind == "packageIndex":
        return ArtifactPayloadPackageIndex(
            package_index=destack._generated.artifact.package.from_json_package_index(
                json_field(object_, "package_index")
            )
        )
    elif kind == "componentGraph":
        return ArtifactPayloadComponentGraph(
            component_graph=destack._generated.artifact.component.from_json_component_graph(
                json_field(object_, "component_graph")
            )
        )
    elif kind == "programAnalysis":
        return ArtifactPayloadProgramAnalysis(
            program_analysis=destack._generated.artifact.mir.from_json_program_analysis(
                json_field(object_, "program_analysis")
            )
        )
    elif kind == "dirBound":
        return ArtifactPayloadDirBound(
            dir_bound=destack._generated.artifact.dir.from_json_dir_bound(
                json_field(object_, "dir_bound")
            )
        )
    elif kind == "dirImported":
        return ArtifactPayloadDirImported(
            dir_imported=destack._generated.artifact.dir.from_json_dir_imported(
                json_field(object_, "dir_imported")
            )
        )
    elif kind == "dirExpanded":
        return ArtifactPayloadDirExpanded(
            dir_expanded=destack._generated.artifact.dir.from_json_dir_expanded(
                json_field(object_, "dir_expanded")
            )
        )
    elif kind == "dirExported":
        return ArtifactPayloadDirExported(
            dir_exported=destack._generated.artifact.dir.from_json_dir_exported(
                json_field(object_, "dir_exported")
            )
        )
    elif kind == "dirResolved":
        return ArtifactPayloadDirResolved(
            dir_resolved=destack._generated.artifact.dir.from_json_dir_resolved(
                json_field(object_, "dir_resolved")
            )
        )
    elif kind == "dirCheckedComponent":
        return ArtifactPayloadDirCheckedComponent(
            dir_checked_component=destack._generated.artifact.dir.from_json_dir_checked_component(
                json_field(object_, "dir_checked_component")
            )
        )
    elif kind == "dirChecked":
        return ArtifactPayloadDirChecked(
            dir_checked=destack._generated.artifact.dir.from_json_dir_checked(
                json_field(object_, "dir_checked")
            )
        )
    elif kind == "dirMaterialized":
        return ArtifactPayloadDirMaterialized(
            dir_materialized=destack._generated.artifact.dir.from_json_dir_materialized(
                json_field(object_, "dir_materialized")
            )
        )
    elif kind == "dirElaborated":
        return ArtifactPayloadDirElaborated(
            dir_elaborated=destack._generated.artifact.dir.from_json_dir_elaborated(
                json_field(object_, "dir_elaborated")
            )
        )
    elif kind == "mirLowered":
        return ArtifactPayloadMirLowered(
            mir_lowered=destack._generated.artifact.mir.from_json_mir_lowered(
                json_field(object_, "mir_lowered")
            )
        )
    elif kind == "mirVerified":
        return ArtifactPayloadMirVerified(
            mir_verified=destack._generated.artifact.mir.from_json_mir_verified(
                json_field(object_, "mir_verified")
            )
        )
    elif kind == "mirAnalyzed":
        return ArtifactPayloadMirAnalyzed(
            mir_analyzed=destack._generated.artifact.mir.from_json_mir_analyzed(
                json_field(object_, "mir_analyzed")
            )
        )
    elif kind == "mirOptimized":
        return ArtifactPayloadMirOptimized(
            mir_optimized=destack._generated.artifact.mir.from_json_mir_optimized(
                json_field(object_, "mir_optimized")
            )
        )
    elif kind == "moduleQueryIndex":
        return ArtifactPayloadModuleQueryIndex(
            module_query_index=destack._generated.artifact.query.from_json_module_query_index(
                json_field(object_, "module_query_index")
            )
        )
    elif kind == "workspaceQueryIndex":
        return ArtifactPayloadWorkspaceQueryIndex(
            workspace_query_index=destack._generated.artifact.query.from_json_workspace_query_index(
                json_field(object_, "workspace_query_index")
            )
        )
    elif kind == "script":
        return ArtifactPayloadScript(
            script=destack._generated.artifact.script.from_json_script(
                json_field(object_, "script")
            )
        )
    elif kind == "object":
        return ArtifactPayloadObject(
            object=destack._generated.artifact.object.from_json_object(
                json_field(object_, "object")
            )
        )
    elif kind == "asset":
        return ArtifactPayloadAsset(
            asset=destack._generated.artifact.asset.from_json_asset(
                json_field(object_, "asset")
            )
        )
    elif kind == "build":
        return ArtifactPayloadBuild(
            build=destack._generated.artifact.build.from_json_build(
                json_field(object_, "build")
            )
        )
    elif kind == "bundle":
        return ArtifactPayloadBundle(
            bundle=destack._generated.artifact.bundle.from_json_bundle(
                json_field(object_, "bundle")
            )
        )
    elif kind == "program":
        return ArtifactPayloadProgram(
            program=destack._generated.program.program.from_json_program(
                json_field(object_, "program")
            )
        )
    elif kind == "product":
        return ArtifactPayloadProduct(
            product=destack._generated.artifact.product.from_json_product(
                json_field(object_, "product")
            )
        )
    elif kind == "moduleLinted":
        return ArtifactPayloadModuleLinted(
            module_linted=destack._generated.artifact.lint.from_json_module_linted(
                json_field(object_, "module_linted")
            )
        )
    elif kind == "packageLinted":
        return ArtifactPayloadPackageLinted(
            package_linted=destack._generated.artifact.lint.from_json_package_linted(
                json_field(object_, "package_linted")
            )
        )
    elif kind == "workspaceLinted":
        return ArtifactPayloadWorkspaceLinted(
            workspace_linted=destack._generated.artifact.lint.from_json_workspace_linted(
                json_field(object_, "workspace_linted")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
    "ArtifactPayloadDirElaborated",
    "ArtifactPayloadMirLowered",
    "ArtifactPayloadMirVerified",
    "ArtifactPayloadMirAnalyzed",
    "ArtifactPayloadMirOptimized",
    "ArtifactPayloadModuleQueryIndex",
    "ArtifactPayloadWorkspaceQueryIndex",
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
