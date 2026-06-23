# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.artifact.reference
import destack._generated.protocol.repository.provider.trace
import destack._generated.protocol.workspace.command.common

if TYPE_CHECKING:
    from destack._generated.protocol.artifact.reference import (
        ArtifactReference,
    )

    from destack._generated.protocol.repository.provider.trace import (
        TraceSnapshot,
        TraceView,
    )

    from destack._generated.protocol.workspace.command.common import (
        CommandEnvVar,
        CommandInput,
        CommandRevision,
        CommandTargetOverrides,
        ManifestOverride,
    )


@dataclass(frozen=True, slots=True)
class BuildInput:
    """Request to build target artifacts."""

    """Revision selected for this build."""
    revision: CommandRevision
    """Input sources for the command."""
    inputs: Sequence[CommandInput]
    """Whether destack.json should resolve inputs when none are provided."""
    config_inputs: bool
    """Optional working directory for this command."""
    cwd: str | None
    """Optional Destack manifest path override."""
    manifest: str | None
    """Optional target name override."""
    target: str | None
    """Optional target overrides."""
    target_overrides: CommandTargetOverrides | None
    """Optional profile name override."""
    profile: str | None
    """Optional environment overrides."""
    env: Sequence[CommandEnvVar]
    """Optional manifest overrides."""
    overrides: Sequence[ManifestOverride]
    """Whether the command should watch for changes."""
    watch: bool
    """Whether the command should skip writes."""
    dry_run: bool
    """Trace detail returned in the response."""
    trace: TraceView
    """Product name selected for this build."""
    product: str | None
    """Build outputs requested by the caller."""
    outputs: BuildOutputs


def encode_build_input(writer: Writer, value: BuildInput) -> None:
    destack._generated.protocol.workspace.command.common.encode_command_revision(
        writer, value.revision
    )
    writer.write_unsigned(len(value.inputs))
    for item_0 in value.inputs:
        destack._generated.protocol.workspace.command.common.encode_command_input(
            writer, item_0
        )
    writer.write_bool(value.config_inputs)
    if value.cwd is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.cwd)
    if value.manifest is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.manifest)
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.target)
    if value.target_overrides is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.workspace.command.common.encode_command_target_overrides(
            writer, value.target_overrides
        )
    if value.profile is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.profile)
    writer.write_unsigned(len(value.env))
    for item_0 in value.env:
        destack._generated.protocol.workspace.command.common.encode_command_env_var(
            writer, item_0
        )
    writer.write_unsigned(len(value.overrides))
    for item_0 in value.overrides:
        destack._generated.protocol.workspace.command.common.encode_manifest_override(
            writer, item_0
        )
    writer.write_bool(value.watch)
    writer.write_bool(value.dry_run)
    destack._generated.protocol.repository.provider.trace.encode_trace_view(
        writer, value.trace
    )
    if value.product is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.product)
    encode_build_outputs(writer, value.outputs)


def decode_build_input(reader: Reader) -> BuildInput:
    field_0 = (
        destack._generated.protocol.workspace.command.common.decode_command_revision(
            reader
        )
    )
    field_1 = [
        destack._generated.protocol.workspace.command.common.decode_command_input(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_2 = reader.read_bool()
    field_3 = reader.read_option(lambda: reader.read_string())
    field_4 = reader.read_option(lambda: reader.read_string())
    field_5 = reader.read_option(lambda: reader.read_string())
    field_6 = reader.read_option(
        lambda: (
            destack._generated.protocol.workspace.command.common.decode_command_target_overrides(
                reader
            )
        )
    )
    field_7 = reader.read_option(lambda: reader.read_string())
    field_8 = [
        destack._generated.protocol.workspace.command.common.decode_command_env_var(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_9 = [
        destack._generated.protocol.workspace.command.common.decode_manifest_override(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_10 = reader.read_bool()
    field_11 = reader.read_bool()
    field_12 = destack._generated.protocol.repository.provider.trace.decode_trace_view(
        reader
    )
    field_13 = reader.read_option(lambda: reader.read_string())
    field_14 = decode_build_outputs(reader)

    return BuildInput(
        revision=field_0,
        inputs=field_1,
        config_inputs=field_2,
        cwd=field_3,
        manifest=field_4,
        target=field_5,
        target_overrides=field_6,
        profile=field_7,
        env=field_8,
        overrides=field_9,
        watch=field_10,
        dry_run=field_11,
        trace=field_12,
        product=field_13,
        outputs=field_14,
    )


@dataclass(frozen=True, slots=True)
class BuildOutputs:
    """Build output families requested by a caller."""

    """Return product artifact refs."""
    products: bool
    """Return bundle artifact refs."""
    bundles: bool
    """Return program artifact refs."""
    programs: bool
    """Return per-module asset artifact refs."""
    assets: bool


def encode_build_outputs(writer: Writer, value: BuildOutputs) -> None:
    writer.write_bool(value.products)
    writer.write_bool(value.bundles)
    writer.write_bool(value.programs)
    writer.write_bool(value.assets)


def decode_build_outputs(reader: Reader) -> BuildOutputs:
    field_0 = reader.read_bool()
    field_1 = reader.read_bool()
    field_2 = reader.read_bool()
    field_3 = reader.read_bool()

    return BuildOutputs(
        products=field_0,
        bundles=field_1,
        programs=field_2,
        assets=field_3,
    )


@dataclass(frozen=True, slots=True)
class BuildPayload:
    """Payload for build command output."""

    """Product artifacts produced by this build."""
    products: Sequence[ArtifactReference]
    """Bundle artifacts produced by this build."""
    bundles: Sequence[ArtifactReference]
    """Program artifacts produced by this build."""
    programs: Sequence[ArtifactReference]
    """Per-module asset artifacts produced by this build."""
    assets: Sequence[ArtifactReference]
    """Trace payload for this build."""
    trace: TraceSnapshot


def encode_build_payload(writer: Writer, value: BuildPayload) -> None:
    writer.write_unsigned(len(value.products))
    for item_0 in value.products:
        destack._generated.protocol.artifact.reference.encode_artifact_reference(
            writer, item_0
        )
    writer.write_unsigned(len(value.bundles))
    for item_0 in value.bundles:
        destack._generated.protocol.artifact.reference.encode_artifact_reference(
            writer, item_0
        )
    writer.write_unsigned(len(value.programs))
    for item_0 in value.programs:
        destack._generated.protocol.artifact.reference.encode_artifact_reference(
            writer, item_0
        )
    writer.write_unsigned(len(value.assets))
    for item_0 in value.assets:
        destack._generated.protocol.artifact.reference.encode_artifact_reference(
            writer, item_0
        )
    destack._generated.protocol.repository.provider.trace.encode_trace_snapshot(
        writer, value.trace
    )


def decode_build_payload(reader: Reader) -> BuildPayload:
    field_0 = [
        destack._generated.protocol.artifact.reference.decode_artifact_reference(reader)
        for _ in range(reader.read_number())
    ]
    field_1 = [
        destack._generated.protocol.artifact.reference.decode_artifact_reference(reader)
        for _ in range(reader.read_number())
    ]
    field_2 = [
        destack._generated.protocol.artifact.reference.decode_artifact_reference(reader)
        for _ in range(reader.read_number())
    ]
    field_3 = [
        destack._generated.protocol.artifact.reference.decode_artifact_reference(reader)
        for _ in range(reader.read_number())
    ]
    field_4 = (
        destack._generated.protocol.repository.provider.trace.decode_trace_snapshot(
            reader
        )
    )

    return BuildPayload(
        products=field_0,
        bundles=field_1,
        programs=field_2,
        assets=field_3,
        trace=field_4,
    )


__all__ = [
    "BuildInput",
    "encode_build_input",
    "decode_build_input",
    "BuildOutputs",
    "encode_build_outputs",
    "decode_build_outputs",
    "BuildPayload",
    "encode_build_payload",
    "decode_build_payload",
]
