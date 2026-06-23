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
class DoctorInput:
    """Request to return workspace health information."""

    """Revision selected for this doctor request."""
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
    """Whether to run extended checks."""
    full: bool


def encode_doctor_input(writer: Writer, value: DoctorInput) -> None:
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
    writer.write_bool(value.full)


def decode_doctor_input(reader: Reader) -> DoctorInput:
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

    return DoctorInput(
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
        full=field_12,
    )


@dataclass(frozen=True, slots=True)
class DoctorPayload:
    """Payload for doctor command output."""

    """CLI version string."""
    cli_version: str
    """Working directory."""
    cwd: str
    """Host operating system."""
    os: str
    """Host architecture."""
    arch: str
    """Worker count configured."""
    workers: int
    """Detected parallelism."""
    available_parallelism: int
    """Workspace metadata."""
    workspace: DoctorWorkspace
    """Resolved destack.json path."""
    manifest: str | None
    """Manifest extends entries."""
    extends: Sequence[str] | None
    """Default target name."""
    default_target: str | None
    """Target count."""
    target_count: int
    """Target names when requested."""
    targets: Sequence[str] | None
    """Tool checks when requested."""
    tools: Sequence[DoctorTool] | None
    """Warning list."""
    warnings: Sequence[str] | None


def encode_doctor_payload(writer: Writer, value: DoctorPayload) -> None:
    writer.write_string(value.cli_version)
    writer.write_string(value.cwd)
    writer.write_string(value.os)
    writer.write_string(value.arch)
    writer.write_unsigned(value.workers)
    writer.write_unsigned(value.available_parallelism)
    encode_doctor_workspace(writer, value.workspace)
    if value.manifest is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.manifest)
    if value.extends is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.extends))
        for item_1 in value.extends:
            writer.write_string(item_1)
    if value.default_target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.default_target)
    writer.write_unsigned(value.target_count)
    if value.targets is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.targets))
        for item_1 in value.targets:
            writer.write_string(item_1)
    if value.tools is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.tools))
        for item_1 in value.tools:
            encode_doctor_tool(writer, item_1)
    if value.warnings is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.warnings))
        for item_1 in value.warnings:
            writer.write_string(item_1)


def decode_doctor_payload(reader: Reader) -> DoctorPayload:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = reader.read_string()
    field_3 = reader.read_string()
    field_4 = reader.read_number()
    field_5 = reader.read_number()
    field_6 = decode_doctor_workspace(reader)
    field_7 = reader.read_option(lambda: reader.read_string())
    field_8 = reader.read_option(
        lambda: [reader.read_string() for _ in range(reader.read_number())]
    )
    field_9 = reader.read_option(lambda: reader.read_string())
    field_10 = reader.read_number()
    field_11 = reader.read_option(
        lambda: [reader.read_string() for _ in range(reader.read_number())]
    )
    field_12 = reader.read_option(
        lambda: [decode_doctor_tool(reader) for _ in range(reader.read_number())]
    )
    field_13 = reader.read_option(
        lambda: [reader.read_string() for _ in range(reader.read_number())]
    )

    return DoctorPayload(
        cli_version=field_0,
        cwd=field_1,
        os=field_2,
        arch=field_3,
        workers=field_4,
        available_parallelism=field_5,
        workspace=field_6,
        manifest=field_7,
        extends=field_8,
        default_target=field_9,
        target_count=field_10,
        targets=field_11,
        tools=field_12,
        warnings=field_13,
    )


@dataclass(frozen=True, slots=True)
class DoctorWorkspace:
    """Workspace details for doctor output."""

    """Workspace root path."""
    root: str
    """Workspace kind label."""
    kind: str
    """Number of packages."""
    package_count: int
    """Package paths when requested."""
    packages: Sequence[str] | None


def encode_doctor_workspace(writer: Writer, value: DoctorWorkspace) -> None:
    writer.write_string(value.root)
    writer.write_string(value.kind)
    writer.write_unsigned(value.package_count)
    if value.packages is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.packages))
        for item_1 in value.packages:
            writer.write_string(item_1)


def decode_doctor_workspace(reader: Reader) -> DoctorWorkspace:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = reader.read_number()
    field_3 = reader.read_option(
        lambda: [reader.read_string() for _ in range(reader.read_number())]
    )

    return DoctorWorkspace(
        root=field_0,
        kind=field_1,
        package_count=field_2,
        packages=field_3,
    )


@dataclass(frozen=True, slots=True)
class DoctorTool:
    """Tool probe entry for doctor output."""

    """Tool name."""
    name: str
    """Tool version string."""
    version: str | None
    """Tool probe status."""
    status: DoctorToolStatus


def encode_doctor_tool(writer: Writer, value: DoctorTool) -> None:
    writer.write_string(value.name)
    if value.version is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.version)
    encode_doctor_tool_status(writer, value.status)


def decode_doctor_tool(reader: Reader) -> DoctorTool:
    field_0 = reader.read_string()
    field_1 = reader.read_option(lambda: reader.read_string())
    field_2 = decode_doctor_tool_status(reader)

    return DoctorTool(
        name=field_0,
        version=field_1,
        status=field_2,
    )


"""Status values for tool detection."""
DoctorToolStatus: TypeAlias = (
    Literal["available"] | Literal["missing"] | Literal["error"]
)


def encode_doctor_tool_status(writer: Writer, value: DoctorToolStatus) -> None:
    if value == "available":
        writer.write_unsigned(0)
    elif value == "missing":
        writer.write_unsigned(1)
    elif value == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_doctor_tool_status(reader: Reader) -> DoctorToolStatus:
    variant = reader.read_number()

    if variant == 0:
        return "available"
    elif variant == 1:
        return "missing"
    elif variant == 2:
        return "error"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "DoctorInput",
    "encode_doctor_input",
    "decode_doctor_input",
    "DoctorPayload",
    "encode_doctor_payload",
    "decode_doctor_payload",
    "DoctorWorkspace",
    "encode_doctor_workspace",
    "decode_doctor_workspace",
    "DoctorTool",
    "encode_doctor_tool",
    "decode_doctor_tool",
    "DoctorToolStatus",
    "encode_doctor_tool_status",
    "decode_doctor_tool_status",
]
