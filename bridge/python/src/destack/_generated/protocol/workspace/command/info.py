# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.workspace.command.common
import destack._generated.protocol.workspace.command.targets

if TYPE_CHECKING:
    from destack._generated.protocol.workspace.command.common import (
        CommandEnvVar,
        CommandInput,
        CommandRevision,
        CommandTargetOverrides,
        ManifestOverride,
    )

    from destack._generated.protocol.workspace.command.targets import (
        TargetEntry,
    )


@dataclass(frozen=True, slots=True)
class InfoInput:
    """Request to return workspace information."""

    """Revision selected for this info request."""
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
    """Whether to include all workspace packages."""
    all: bool


def encode_info_input(writer: Writer, value: InfoInput) -> None:
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


def decode_info_input(reader: Reader) -> InfoInput:
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

    return InfoInput(
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
class InfoPayload:
    """Payload for info command output."""

    """Workspace metadata."""
    workspace: InfoWorkspace
    """Resolved destack.json path."""
    manifest: str | None
    """Targets for the active package."""
    targets: Sequence[TargetEntry] | None
    """Targets for all workspace packages."""
    workspace_targets: Sequence[TargetEntry] | None


def encode_info_payload(writer: Writer, value: InfoPayload) -> None:
    encode_info_workspace(writer, value.workspace)
    if value.manifest is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.manifest)
    if value.targets is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.targets))
        for item_1 in value.targets:
            destack._generated.protocol.workspace.command.targets.encode_target_entry(
                writer, item_1
            )
    if value.workspace_targets is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.workspace_targets))
        for item_1 in value.workspace_targets:
            destack._generated.protocol.workspace.command.targets.encode_target_entry(
                writer, item_1
            )


def decode_info_payload(reader: Reader) -> InfoPayload:
    field_0 = decode_info_workspace(reader)
    field_1 = reader.read_option(lambda: reader.read_string())
    field_2 = reader.read_option(
        lambda: [
            destack._generated.protocol.workspace.command.targets.decode_target_entry(
                reader
            )
            for _ in range(reader.read_number())
        ]
    )
    field_3 = reader.read_option(
        lambda: [
            destack._generated.protocol.workspace.command.targets.decode_target_entry(
                reader
            )
            for _ in range(reader.read_number())
        ]
    )

    return InfoPayload(
        workspace=field_0,
        manifest=field_1,
        targets=field_2,
        workspace_targets=field_3,
    )


@dataclass(frozen=True, slots=True)
class InfoWorkspace:
    """Workspace info for info command output."""

    """Workspace root path."""
    root: str
    """Workspace kind label."""
    kind: str
    """Workspace package paths."""
    packages: Sequence[str]


def encode_info_workspace(writer: Writer, value: InfoWorkspace) -> None:
    writer.write_string(value.root)
    writer.write_string(value.kind)
    writer.write_unsigned(len(value.packages))
    for item_0 in value.packages:
        writer.write_string(item_0)


def decode_info_workspace(reader: Reader) -> InfoWorkspace:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = [reader.read_string() for _ in range(reader.read_number())]

    return InfoWorkspace(
        root=field_0,
        kind=field_1,
        packages=field_2,
    )


__all__ = [
    "InfoInput",
    "encode_info_input",
    "decode_info_input",
    "InfoPayload",
    "encode_info_payload",
    "decode_info_payload",
    "InfoWorkspace",
    "encode_info_workspace",
    "decode_info_workspace",
]
