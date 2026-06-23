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
class CleanInput:
    """Request to clean generated state."""

    """Revision selected for this clean request."""
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
    """Optional directory override."""
    dir: str | None
    """Remove build output directories."""
    dist: bool
    """Remove cache directories."""
    cache: bool
    """Remove all build outputs and caches."""
    all: bool
    """Clean all packages in the workspace."""
    all_packages: bool


def encode_clean_input(writer: Writer, value: CleanInput) -> None:
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
    if value.dir is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.dir)
    writer.write_bool(value.dist)
    writer.write_bool(value.cache)
    writer.write_bool(value.all)
    writer.write_bool(value.all_packages)


def decode_clean_input(reader: Reader) -> CleanInput:
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
    field_12 = reader.read_option(lambda: reader.read_string())
    field_13 = reader.read_bool()
    field_14 = reader.read_bool()
    field_15 = reader.read_bool()
    field_16 = reader.read_bool()

    return CleanInput(
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
        dir=field_12,
        dist=field_13,
        cache=field_14,
        all=field_15,
        all_packages=field_16,
    )


@dataclass(frozen=True, slots=True)
class CleanPayload:
    """Payload for clean command output."""

    """Removed paths or candidate paths for dry runs."""
    removed: Sequence[str]
    """Errors encountered during removal."""
    errors: Sequence[str]
    """Whether this was a dry run."""
    dry_run: bool


def encode_clean_payload(writer: Writer, value: CleanPayload) -> None:
    writer.write_unsigned(len(value.removed))
    for item_0 in value.removed:
        writer.write_string(item_0)
    writer.write_unsigned(len(value.errors))
    for item_0 in value.errors:
        writer.write_string(item_0)
    writer.write_bool(value.dry_run)


def decode_clean_payload(reader: Reader) -> CleanPayload:
    field_0 = [reader.read_string() for _ in range(reader.read_number())]
    field_1 = [reader.read_string() for _ in range(reader.read_number())]
    field_2 = reader.read_bool()

    return CleanPayload(
        removed=field_0,
        errors=field_1,
        dry_run=field_2,
    )


__all__ = [
    "CleanInput",
    "encode_clean_input",
    "decode_clean_input",
    "CleanPayload",
    "encode_clean_payload",
    "decode_clean_payload",
]
