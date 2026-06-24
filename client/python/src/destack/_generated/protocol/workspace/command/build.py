# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.artifact.reference
import destack._generated.protocol.workspace.command.common
import destack._generated.repository.provider.trace


@dataclass(frozen=True, slots=True)
class BuildInput:
    """Request to build target artifacts."""

    # revision selected for this build
    revision: destack._generated.protocol.workspace.command.common.CommandRevision
    # input sources for the command
    inputs: Sequence[destack._generated.protocol.workspace.command.common.CommandInput]
    # whether destack.json should resolve inputs when none are provided
    config_inputs: bool
    # optional working directory for this command
    cwd: str | None
    # optional Destack manifest path override
    manifest: str | None
    # optional target name override
    target: str | None
    # optional target overrides
    target_overrides: (
        destack._generated.protocol.workspace.command.common.CommandTargetOverrides
        | None
    )
    # optional profile name override
    profile: str | None
    # optional environment overrides
    env: Sequence[destack._generated.protocol.workspace.command.common.CommandEnvVar]
    # optional manifest overrides
    overrides: Sequence[
        destack._generated.protocol.workspace.command.common.ManifestOverride
    ]
    # whether the command should watch for changes
    watch: bool
    # whether the command should skip writes
    dry_run: bool
    # trace detail returned in the response
    trace: destack._generated.repository.provider.trace.TraceView
    # product name selected for this build
    product: str | None
    # build outputs requested by the caller
    outputs: BuildOutputs

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_build_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BuildInput:
        """Decode one BuildInput."""
        return decode_build_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_build_input(self)

    @classmethod
    def from_json(cls, value: Json) -> BuildInput:
        """Return one BuildInput from one JSON value."""
        return from_json_build_input(value)


def encode_build_input(writer: BinaryWriter, value: BuildInput) -> None:
    """Encode one BuildInput."""
    destack._generated.protocol.workspace.command.common.encode_command_revision(
        writer, value.revision
    )
    writer.write_unsigned(len(value.inputs))
    for item_value_inputs_0 in value.inputs:
        destack._generated.protocol.workspace.command.common.encode_command_input(
            writer, item_value_inputs_0
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
    for item_value_env_0 in value.env:
        destack._generated.protocol.workspace.command.common.encode_command_env_var(
            writer, item_value_env_0
        )
    writer.write_unsigned(len(value.overrides))
    for item_value_overrides_0 in value.overrides:
        destack._generated.protocol.workspace.command.common.encode_manifest_override(
            writer, item_value_overrides_0
        )
    writer.write_bool(value.watch)
    writer.write_bool(value.dry_run)
    destack._generated.repository.provider.trace.encode_trace_view(writer, value.trace)
    if value.product is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.product)
    encode_build_outputs(writer, value.outputs)


def decode_build_input(reader: BinaryReader) -> BuildInput:
    """Decode one BuildInput."""
    revision = (
        destack._generated.protocol.workspace.command.common.decode_command_revision(
            reader
        )
    )
    inputs = [
        destack._generated.protocol.workspace.command.common.decode_command_input(
            reader
        )
        for _ in range(reader.read_number())
    ]
    config_inputs = reader.read_bool()
    cwd = reader.read_option(lambda: reader.read_string())
    manifest = reader.read_option(lambda: reader.read_string())
    target = reader.read_option(lambda: reader.read_string())
    target_overrides = reader.read_option(
        lambda: (
            destack._generated.protocol.workspace.command.common.decode_command_target_overrides(
                reader
            )
        )
    )
    profile = reader.read_option(lambda: reader.read_string())
    env = [
        destack._generated.protocol.workspace.command.common.decode_command_env_var(
            reader
        )
        for _ in range(reader.read_number())
    ]
    overrides = [
        destack._generated.protocol.workspace.command.common.decode_manifest_override(
            reader
        )
        for _ in range(reader.read_number())
    ]
    watch = reader.read_bool()
    dry_run = reader.read_bool()
    trace = destack._generated.repository.provider.trace.decode_trace_view(reader)
    product = reader.read_option(lambda: reader.read_string())
    outputs = decode_build_outputs(reader)

    return BuildInput(
        revision=revision,
        inputs=inputs,
        config_inputs=config_inputs,
        cwd=cwd,
        manifest=manifest,
        target=target,
        target_overrides=target_overrides,
        profile=profile,
        env=env,
        overrides=overrides,
        watch=watch,
        dry_run=dry_run,
        trace=trace,
        product=product,
        outputs=outputs,
    )


def to_json_build_input(value: BuildInput) -> Json:
    """Return one JSON value for one BuildInput."""
    return {
        "revision": destack._generated.protocol.workspace.command.common.to_json_command_revision(
            value.revision
        ),
        "inputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_input(
                item_0
            )
            for item_0 in value.inputs
        ],
        "configInputs": value.config_inputs,
        **({} if value.cwd is None else {"cwd": value.cwd}),
        **({} if value.manifest is None else {"manifest": value.manifest}),
        **({} if value.target is None else {"target": value.target}),
        **(
            {}
            if value.target_overrides is None
            else {
                "targetOverrides": destack._generated.protocol.workspace.command.common.to_json_command_target_overrides(
                    value.target_overrides
                )
            }
        ),
        **({} if value.profile is None else {"profile": value.profile}),
        "env": [
            destack._generated.protocol.workspace.command.common.to_json_command_env_var(
                item_0
            )
            for item_0 in value.env
        ],
        "overrides": [
            destack._generated.protocol.workspace.command.common.to_json_manifest_override(
                item_0
            )
            for item_0 in value.overrides
        ],
        "watch": value.watch,
        "dryRun": value.dry_run,
        "trace": destack._generated.repository.provider.trace.to_json_trace_view(
            value.trace
        ),
        **({} if value.product is None else {"product": value.product}),
        "outputs": to_json_build_outputs(value.outputs),
    }


def from_json_build_input(value: Json) -> BuildInput:
    """Return one BuildInput from one JSON value."""
    object_ = json_object(value)

    return BuildInput(
        revision=destack._generated.protocol.workspace.command.common.from_json_command_revision(
            json_field(object_, "revision")
        ),
        inputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_input(
                item_0
            )
            for item_0 in json_array(json_field(object_, "inputs"))
        ],
        config_inputs=json_bool(json_field(object_, "configInputs")),
        cwd=json_optional(object_, "cwd", lambda value: json_string(value)),
        manifest=json_optional(object_, "manifest", lambda value: json_string(value)),
        target=json_optional(object_, "target", lambda value: json_string(value)),
        target_overrides=json_optional(
            object_,
            "targetOverrides",
            lambda value: (
                destack._generated.protocol.workspace.command.common.from_json_command_target_overrides(
                    value
                )
            ),
        ),
        profile=json_optional(object_, "profile", lambda value: json_string(value)),
        env=[
            destack._generated.protocol.workspace.command.common.from_json_command_env_var(
                item_0
            )
            for item_0 in json_array(json_field(object_, "env"))
        ],
        overrides=[
            destack._generated.protocol.workspace.command.common.from_json_manifest_override(
                item_0
            )
            for item_0 in json_array(json_field(object_, "overrides"))
        ],
        watch=json_bool(json_field(object_, "watch")),
        dry_run=json_bool(json_field(object_, "dryRun")),
        trace=destack._generated.repository.provider.trace.from_json_trace_view(
            json_field(object_, "trace")
        ),
        product=json_optional(object_, "product", lambda value: json_string(value)),
        outputs=from_json_build_outputs(json_field(object_, "outputs")),
    )


@dataclass(frozen=True, slots=True)
class BuildOutputs:
    """Build output families requested by a caller."""

    # return product artifact refs
    products: bool
    # return bundle artifact refs
    bundles: bool
    # return program artifact refs
    programs: bool
    # return per-module asset artifact refs
    assets: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_build_outputs(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BuildOutputs:
        """Decode one BuildOutputs."""
        return decode_build_outputs(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_build_outputs(self)

    @classmethod
    def from_json(cls, value: Json) -> BuildOutputs:
        """Return one BuildOutputs from one JSON value."""
        return from_json_build_outputs(value)


def encode_build_outputs(writer: BinaryWriter, value: BuildOutputs) -> None:
    """Encode one BuildOutputs."""
    writer.write_bool(value.products)
    writer.write_bool(value.bundles)
    writer.write_bool(value.programs)
    writer.write_bool(value.assets)


def decode_build_outputs(reader: BinaryReader) -> BuildOutputs:
    """Decode one BuildOutputs."""
    products = reader.read_bool()
    bundles = reader.read_bool()
    programs = reader.read_bool()
    assets = reader.read_bool()

    return BuildOutputs(
        products=products,
        bundles=bundles,
        programs=programs,
        assets=assets,
    )


def to_json_build_outputs(value: BuildOutputs) -> Json:
    """Return one JSON value for one BuildOutputs."""
    return {
        "products": value.products,
        "bundles": value.bundles,
        "programs": value.programs,
        "assets": value.assets,
    }


def from_json_build_outputs(value: Json) -> BuildOutputs:
    """Return one BuildOutputs from one JSON value."""
    object_ = json_object(value)

    return BuildOutputs(
        products=json_bool(json_field(object_, "products")),
        bundles=json_bool(json_field(object_, "bundles")),
        programs=json_bool(json_field(object_, "programs")),
        assets=json_bool(json_field(object_, "assets")),
    )


@dataclass(frozen=True, slots=True)
class BuildPayload:
    """Payload for build command output."""

    # product artifacts produced by this build
    products: Sequence[destack._generated.artifact.reference.ArtifactReference]
    # bundle artifacts produced by this build
    bundles: Sequence[destack._generated.artifact.reference.ArtifactReference]
    # program artifacts produced by this build
    programs: Sequence[destack._generated.artifact.reference.ArtifactReference]
    # per-module asset artifacts produced by this build
    assets: Sequence[destack._generated.artifact.reference.ArtifactReference]
    # trace payload for this build
    trace: destack._generated.repository.provider.trace.TraceSnapshot

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_build_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BuildPayload:
        """Decode one BuildPayload."""
        return decode_build_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_build_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> BuildPayload:
        """Return one BuildPayload from one JSON value."""
        return from_json_build_payload(value)


def encode_build_payload(writer: BinaryWriter, value: BuildPayload) -> None:
    """Encode one BuildPayload."""
    writer.write_unsigned(len(value.products))
    for item_value_products_0 in value.products:
        destack._generated.artifact.reference.encode_artifact_reference(
            writer, item_value_products_0
        )
    writer.write_unsigned(len(value.bundles))
    for item_value_bundles_0 in value.bundles:
        destack._generated.artifact.reference.encode_artifact_reference(
            writer, item_value_bundles_0
        )
    writer.write_unsigned(len(value.programs))
    for item_value_programs_0 in value.programs:
        destack._generated.artifact.reference.encode_artifact_reference(
            writer, item_value_programs_0
        )
    writer.write_unsigned(len(value.assets))
    for item_value_assets_0 in value.assets:
        destack._generated.artifact.reference.encode_artifact_reference(
            writer, item_value_assets_0
        )
    destack._generated.repository.provider.trace.encode_trace_snapshot(
        writer, value.trace
    )


def decode_build_payload(reader: BinaryReader) -> BuildPayload:
    """Decode one BuildPayload."""
    products = [
        destack._generated.artifact.reference.decode_artifact_reference(reader)
        for _ in range(reader.read_number())
    ]
    bundles = [
        destack._generated.artifact.reference.decode_artifact_reference(reader)
        for _ in range(reader.read_number())
    ]
    programs = [
        destack._generated.artifact.reference.decode_artifact_reference(reader)
        for _ in range(reader.read_number())
    ]
    assets = [
        destack._generated.artifact.reference.decode_artifact_reference(reader)
        for _ in range(reader.read_number())
    ]
    trace = destack._generated.repository.provider.trace.decode_trace_snapshot(reader)

    return BuildPayload(
        products=products,
        bundles=bundles,
        programs=programs,
        assets=assets,
        trace=trace,
    )


def to_json_build_payload(value: BuildPayload) -> Json:
    """Return one JSON value for one BuildPayload."""
    return {
        "products": [
            destack._generated.artifact.reference.to_json_artifact_reference(item_0)
            for item_0 in value.products
        ],
        "bundles": [
            destack._generated.artifact.reference.to_json_artifact_reference(item_0)
            for item_0 in value.bundles
        ],
        "programs": [
            destack._generated.artifact.reference.to_json_artifact_reference(item_0)
            for item_0 in value.programs
        ],
        "assets": [
            destack._generated.artifact.reference.to_json_artifact_reference(item_0)
            for item_0 in value.assets
        ],
        "trace": destack._generated.repository.provider.trace.to_json_trace_snapshot(
            value.trace
        ),
    }


def from_json_build_payload(value: Json) -> BuildPayload:
    """Return one BuildPayload from one JSON value."""
    object_ = json_object(value)

    return BuildPayload(
        products=[
            destack._generated.artifact.reference.from_json_artifact_reference(item_0)
            for item_0 in json_array(json_field(object_, "products"))
        ],
        bundles=[
            destack._generated.artifact.reference.from_json_artifact_reference(item_0)
            for item_0 in json_array(json_field(object_, "bundles"))
        ],
        programs=[
            destack._generated.artifact.reference.from_json_artifact_reference(item_0)
            for item_0 in json_array(json_field(object_, "programs"))
        ],
        assets=[
            destack._generated.artifact.reference.from_json_artifact_reference(item_0)
            for item_0 in json_array(json_field(object_, "assets"))
        ],
        trace=destack._generated.repository.provider.trace.from_json_trace_snapshot(
            json_field(object_, "trace")
        ),
    )


__all__ = [
    "BuildInput",
    "encode_build_input",
    "decode_build_input",
    "to_json_build_input",
    "from_json_build_input",
    "BuildOutputs",
    "encode_build_outputs",
    "decode_build_outputs",
    "to_json_build_outputs",
    "from_json_build_outputs",
    "BuildPayload",
    "encode_build_payload",
    "decode_build_payload",
    "to_json_build_payload",
    "from_json_build_payload",
]
