# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.workspace.command.common

if TYPE_CHECKING:
    from destack._generated.protocol.workspace.command.common import (
        CommandEnvVar,
        CommandInput,
        CommandRevision,
        CommandTargetOverrides,
        ManifestOverride,
    )


@dataclass(frozen=True, slots=True)
class TargetsInput:
    """Request to return configured targets."""

    """Revision selected for this targets request."""
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
    """Whether to list targets for all packages."""
    all: bool


def encode_targets_input(writer: Writer, value: TargetsInput) -> None:
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
    writer.write_bool(value.all)


def decode_targets_input(reader: Reader) -> TargetsInput:
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
    field_12 = reader.read_bool()

    return TargetsInput(
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
        all=field_12,
    )


@dataclass(frozen=True, slots=True)
class TargetEntry:
    """Target row shown by workspace discovery commands."""

    """The target name."""
    name: str
    """The emit format."""
    emit: str
    """The runtime environment."""
    runtime: str
    """The target platform."""
    platform: str
    """The output directory."""
    out_dir: str
    """The output file path, when applicable."""
    out_file: str | None
    """Whether this is the package default target."""
    is_default: bool
    """The owning package directory, when workspace-wide output is requested."""
    package_dir: str | None


def encode_target_entry(writer: Writer, value: TargetEntry) -> None:
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


def decode_target_entry(reader: Reader) -> TargetEntry:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = reader.read_string()
    field_3 = reader.read_string()
    field_4 = reader.read_string()
    field_5 = reader.read_option(lambda: reader.read_string())
    field_6 = reader.read_bool()
    field_7 = reader.read_option(lambda: reader.read_string())

    return TargetEntry(
        name=field_0,
        emit=field_1,
        runtime=field_2,
        platform=field_3,
        out_dir=field_4,
        out_file=field_5,
        is_default=field_6,
        package_dir=field_7,
    )


@dataclass(frozen=True, slots=True)
class TargetsPayload:
    """Payload for targets command output."""

    """List of target entries."""
    targets: Sequence[TargetEntry]


def encode_targets_payload(writer: Writer, value: TargetsPayload) -> None:
    writer.write_unsigned(len(value.targets))
    for item_0 in value.targets:
        encode_target_entry(writer, item_0)


def decode_targets_payload(reader: Reader) -> TargetsPayload:
    field_0 = [decode_target_entry(reader) for _ in range(reader.read_number())]

    return TargetsPayload(
        targets=field_0,
    )


__all__ = [
    "TargetsInput",
    "encode_targets_input",
    "decode_targets_input",
    "TargetEntry",
    "encode_target_entry",
    "decode_target_entry",
    "TargetsPayload",
    "encode_targets_payload",
    "decode_targets_payload",
]
