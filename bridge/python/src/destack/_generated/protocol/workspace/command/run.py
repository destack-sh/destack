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
class RunInput:
    """Request to run a workspace target."""

    """Revision selected for this run."""
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
    """Optional run entry function name."""
    entry: str | None
    """Optional command arguments."""
    args: Sequence[str]
    """Optional run mode override."""
    run_mode: RunMode


def encode_run_input(writer: Writer, value: RunInput) -> None:
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
    if value.entry is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.entry)
    writer.write_unsigned(len(value.args))
    for item_0 in value.args:
        writer.write_string(item_0)
    encode_run_mode(writer, value.run_mode)


def decode_run_input(reader: Reader) -> RunInput:
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
    field_13 = [reader.read_string() for _ in range(reader.read_number())]
    field_14 = decode_run_mode(reader)

    return RunInput(
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
        entry=field_12,
        args=field_13,
        run_mode=field_14,
    )


@dataclass(frozen=True, slots=True)
class RunModeProgram:
    """Execute an entry module."""

    kind: Literal["program"] = "program"


@dataclass(frozen=True, slots=True)
class RunModeEval:
    """Evaluate inline input and optionally print the result."""

    print: bool
    kind: Literal["eval"] = "eval"


"""Run mode for the run command."""
RunMode: TypeAlias = RunModeProgram | RunModeEval


def encode_run_mode(writer: Writer, value: RunMode) -> None:
    if value.kind == "program":
        writer.write_unsigned(0)
    elif value.kind == "eval":
        writer.write_unsigned(1)
        writer.write_bool(value.print)
    else:
        raise SerdeError("unknown enum variant")


def decode_run_mode(reader: Reader) -> RunMode:
    variant = reader.read_number()

    if variant == 0:
        return RunModeProgram()
    elif variant == 1:
        field_0 = reader.read_bool()

        return RunModeEval(
            print=field_0,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class RunPayloadValue:
    """Run completed successfully with a return value."""

    """Returned value from the entry function."""
    value: Any
    kind: Literal["value"] = "value"


@dataclass(frozen=True, slots=True)
class RunPayloadRuntimeError:
    """Run failed with a runtime error."""

    """Runtime error string."""
    message: str
    kind: Literal["runtimeError"] = "runtimeError"


"""Payload for run command output."""
RunPayload: TypeAlias = RunPayloadValue | RunPayloadRuntimeError


def encode_run_payload(writer: Writer, value: RunPayload) -> None:
    if value.kind == "value":
        writer.write_unsigned(0)
        writer.write_json(value.value)
    elif value.kind == "runtimeError":
        writer.write_unsigned(1)
        writer.write_string(value.message)
    else:
        raise SerdeError("unknown enum variant")


def decode_run_payload(reader: Reader) -> RunPayload:
    variant = reader.read_number()

    if variant == 0:
        field_0 = reader.read_json()

        return RunPayloadValue(
            value=field_0,
        )
    elif variant == 1:
        field_0 = reader.read_string()

        return RunPayloadRuntimeError(
            message=field_0,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "RunInput",
    "encode_run_input",
    "decode_run_input",
    "RunMode",
    "encode_run_mode",
    "decode_run_mode",
    "RunModeProgram",
    "RunModeEval",
    "RunPayload",
    "encode_run_payload",
    "decode_run_payload",
    "RunPayloadValue",
    "RunPayloadRuntimeError",
]
