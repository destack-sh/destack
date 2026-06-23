# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.repository.provider.trace
import destack._generated.protocol.workspace.command.common

if TYPE_CHECKING:
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
class CheckInput:
    """Request to check source state."""

    """Revision selected for this check."""
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
    """Whether linting should run when supported."""
    lint: bool
    """Apply lint fixes."""
    fix: bool
    """Include unsafe lint fixes."""
    unsafe_fixes: bool
    """Show lint diff instead of applying fixes."""
    diff: bool
    """Trace detail returned in the response."""
    trace: TraceView


def encode_check_input(writer: Writer, value: CheckInput) -> None:
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
    writer.write_bool(value.lint)
    writer.write_bool(value.fix)
    writer.write_bool(value.unsafe_fixes)
    writer.write_bool(value.diff)
    destack._generated.protocol.repository.provider.trace.encode_trace_view(
        writer, value.trace
    )


def decode_check_input(reader: Reader) -> CheckInput:
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
    field_13 = reader.read_bool()
    field_14 = reader.read_bool()
    field_15 = reader.read_bool()
    field_16 = destack._generated.protocol.repository.provider.trace.decode_trace_view(
        reader
    )

    return CheckInput(
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
        lint=field_12,
        fix=field_13,
        unsafe_fixes=field_14,
        diff=field_15,
        trace=field_16,
    )


@dataclass(frozen=True, slots=True)
class LintInput:
    """Request to lint source state."""

    """Revision selected for this lint run."""
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
    """Apply fixes."""
    fix: bool
    """Include unsafe fixes."""
    unsafe_fixes: bool
    """Show diff instead of applying fixes."""
    diff: bool


def encode_lint_input(writer: Writer, value: LintInput) -> None:
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
    writer.write_bool(value.fix)
    writer.write_bool(value.unsafe_fixes)
    writer.write_bool(value.diff)


def decode_lint_input(reader: Reader) -> LintInput:
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
    field_13 = reader.read_bool()
    field_14 = reader.read_bool()

    return LintInput(
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
        fix=field_12,
        unsafe_fixes=field_13,
        diff=field_14,
    )


@dataclass(frozen=True, slots=True)
class CheckPayload:
    """Payload for check command output."""

    """Trace payload for this check."""
    trace: TraceSnapshot


def encode_check_payload(writer: Writer, value: CheckPayload) -> None:
    destack._generated.protocol.repository.provider.trace.encode_trace_snapshot(
        writer, value.trace
    )


def decode_check_payload(reader: Reader) -> CheckPayload:
    field_0 = (
        destack._generated.protocol.repository.provider.trace.decode_trace_snapshot(
            reader
        )
    )

    return CheckPayload(
        trace=field_0,
    )


@dataclass(frozen=True, slots=True)
class LintPayload:
    """Payload for lint command output."""

    pass


def encode_lint_payload(writer: Writer, value: LintPayload) -> None:
    pass


def decode_lint_payload(reader: Reader) -> LintPayload:
    return LintPayload()


__all__ = [
    "CheckInput",
    "encode_check_input",
    "decode_check_input",
    "LintInput",
    "encode_lint_input",
    "decode_lint_input",
    "CheckPayload",
    "encode_check_payload",
    "decode_check_payload",
    "LintPayload",
    "encode_lint_payload",
    "decode_lint_payload",
]
