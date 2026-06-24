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

import destack._generated.protocol.workspace.command.common


@dataclass(frozen=True, slots=True)
class TargetsInput:
    """Request to return configured targets."""

    # revision selected for this targets request
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
    # whether to list targets for all packages
    all: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_targets_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetsInput:
        """Decode one TargetsInput."""
        return decode_targets_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_targets_input(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetsInput:
        """Return one TargetsInput from one JSON value."""
        return from_json_targets_input(value)


def encode_targets_input(writer: BinaryWriter, value: TargetsInput) -> None:
    """Encode one TargetsInput."""
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
    writer.write_bool(value.all)


def decode_targets_input(reader: BinaryReader) -> TargetsInput:
    """Decode one TargetsInput."""
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
    all = reader.read_bool()

    return TargetsInput(
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
        all=all,
    )


def to_json_targets_input(value: TargetsInput) -> Json:
    """Return one JSON value for one TargetsInput."""
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
        "all": value.all,
    }


def from_json_targets_input(value: Json) -> TargetsInput:
    """Return one TargetsInput from one JSON value."""
    object_ = json_object(value)

    return TargetsInput(
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
        all=json_bool(json_field(object_, "all")),
    )


@dataclass(frozen=True, slots=True)
class TargetEntry:
    """Target row shown by workspace discovery commands."""

    # the target name
    name: str
    # the emit format
    emit: str
    # the runtime environment
    runtime: str
    # the target platform
    platform: str
    # the output directory
    out_dir: str
    # the output file path, when applicable
    out_file: str | None
    # whether this is the package default target
    is_default: bool
    # the owning package directory, when workspace-wide output is requested
    package_dir: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetEntry:
        """Decode one TargetEntry."""
        return decode_target_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetEntry:
        """Return one TargetEntry from one JSON value."""
        return from_json_target_entry(value)


def encode_target_entry(writer: BinaryWriter, value: TargetEntry) -> None:
    """Encode one TargetEntry."""
    writer.write_string(value.name)
    writer.write_string(value.emit)
    writer.write_string(value.runtime)
    writer.write_string(value.platform)
    writer.write_string(value.out_dir)
    if value.out_file is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.out_file)
    writer.write_bool(value.is_default)
    if value.package_dir is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.package_dir)


def decode_target_entry(reader: BinaryReader) -> TargetEntry:
    """Decode one TargetEntry."""
    name = reader.read_string()
    emit = reader.read_string()
    runtime = reader.read_string()
    platform = reader.read_string()
    out_dir = reader.read_string()
    out_file = reader.read_option(lambda: reader.read_string())
    is_default = reader.read_bool()
    package_dir = reader.read_option(lambda: reader.read_string())

    return TargetEntry(
        name=name,
        emit=emit,
        runtime=runtime,
        platform=platform,
        out_dir=out_dir,
        out_file=out_file,
        is_default=is_default,
        package_dir=package_dir,
    )


def to_json_target_entry(value: TargetEntry) -> Json:
    """Return one JSON value for one TargetEntry."""
    return {
        "name": value.name,
        "emit": value.emit,
        "runtime": value.runtime,
        "platform": value.platform,
        "outDir": value.out_dir,
        **({} if value.out_file is None else {"outFile": value.out_file}),
        "isDefault": value.is_default,
        **({} if value.package_dir is None else {"packageDir": value.package_dir}),
    }


def from_json_target_entry(value: Json) -> TargetEntry:
    """Return one TargetEntry from one JSON value."""
    object_ = json_object(value)

    return TargetEntry(
        name=json_string(json_field(object_, "name")),
        emit=json_string(json_field(object_, "emit")),
        runtime=json_string(json_field(object_, "runtime")),
        platform=json_string(json_field(object_, "platform")),
        out_dir=json_string(json_field(object_, "outDir")),
        out_file=json_optional(object_, "outFile", lambda value: json_string(value)),
        is_default=json_bool(json_field(object_, "isDefault")),
        package_dir=json_optional(
            object_, "packageDir", lambda value: json_string(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class TargetsPayload:
    """Payload for targets command output."""

    # list of target entries
    targets: Sequence[TargetEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_targets_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetsPayload:
        """Decode one TargetsPayload."""
        return decode_targets_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_targets_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetsPayload:
        """Return one TargetsPayload from one JSON value."""
        return from_json_targets_payload(value)


def encode_targets_payload(writer: BinaryWriter, value: TargetsPayload) -> None:
    """Encode one TargetsPayload."""
    writer.write_unsigned(len(value.targets))
    for item_value_targets_0 in value.targets:
        encode_target_entry(writer, item_value_targets_0)


def decode_targets_payload(reader: BinaryReader) -> TargetsPayload:
    """Decode one TargetsPayload."""
    targets = [decode_target_entry(reader) for _ in range(reader.read_number())]

    return TargetsPayload(
        targets=targets,
    )


def to_json_targets_payload(value: TargetsPayload) -> Json:
    """Return one JSON value for one TargetsPayload."""
    return {
        "targets": [to_json_target_entry(item_0) for item_0 in value.targets],
    }


def from_json_targets_payload(value: Json) -> TargetsPayload:
    """Return one TargetsPayload from one JSON value."""
    object_ = json_object(value)

    return TargetsPayload(
        targets=[
            from_json_target_entry(item_0)
            for item_0 in json_array(json_field(object_, "targets"))
        ],
    )


__all__ = [
    "TargetsInput",
    "encode_targets_input",
    "decode_targets_input",
    "to_json_targets_input",
    "from_json_targets_input",
    "TargetEntry",
    "encode_target_entry",
    "decode_target_entry",
    "to_json_target_entry",
    "from_json_target_entry",
    "TargetsPayload",
    "encode_targets_payload",
    "decode_targets_payload",
    "to_json_targets_payload",
    "from_json_targets_payload",
]
