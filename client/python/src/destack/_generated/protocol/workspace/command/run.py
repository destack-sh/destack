# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.protocol.workspace.command.common


@dataclass(frozen=True, slots=True)
class RunInput:
    """Request to run a workspace target."""

    # revision selected for this run
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
    # optional run entry function name
    entry: str | None
    # optional command arguments
    args: Sequence[str]
    # optional run mode override
    run_mode: RunMode

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_run_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RunInput:
        """Decode one RunInput."""
        return decode_run_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_run_input(self)

    @classmethod
    def from_json(cls, value: Json) -> RunInput:
        """Return one RunInput from one JSON value."""
        return from_json_run_input(value)


def encode_run_input(writer: BinaryWriter, value: RunInput) -> None:
    """Encode one RunInput."""
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
    if value.entry is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.entry)
    writer.write_unsigned(len(value.args))
    for item_value_args_0 in value.args:
        writer.write_string(item_value_args_0)
    encode_run_mode(writer, value.run_mode)


def decode_run_input(reader: BinaryReader) -> RunInput:
    """Decode one RunInput."""
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
    entry = reader.read_option(lambda: reader.read_string())
    args = [reader.read_string() for _ in range(reader.read_number())]
    run_mode = decode_run_mode(reader)

    return RunInput(
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
        entry=entry,
        args=args,
        run_mode=run_mode,
    )


def to_json_run_input(value: RunInput) -> Json:
    """Return one JSON value for one RunInput."""
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
        **({} if value.entry is None else {"entry": value.entry}),
        "args": [item_0 for item_0 in value.args],
        "runMode": to_json_run_mode(value.run_mode),
    }


def from_json_run_input(value: Json) -> RunInput:
    """Return one RunInput from one JSON value."""
    object_ = json_object(value)

    return RunInput(
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
        entry=json_optional(object_, "entry", lambda value: json_string(value)),
        args=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "args"))
        ],
        run_mode=from_json_run_mode(json_field(object_, "runMode")),
    )


@dataclass(frozen=True, slots=True)
class RunModeProgram:
    """Execute an entry module."""

    kind: typing.Literal["program"] = "program"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_run_mode(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_run_mode(self)


@dataclass(frozen=True, slots=True)
class RunModeEval:
    """Evaluate inline input and optionally print the result."""

    print: bool
    kind: typing.Literal["eval"] = "eval"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_run_mode(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_run_mode(self)


"""Run mode for the run command."""
RunMode: typing.TypeAlias = RunModeProgram | RunModeEval


def encode_run_mode(writer: BinaryWriter, value: RunMode) -> None:
    """Encode one RunMode."""
    if value.kind == "program":
        writer.write_unsigned(0)
    elif value.kind == "eval":
        writer.write_unsigned(1)
        writer.write_bool(value.print)
    else:
        raise SerdeError("unknown enum variant")


def decode_run_mode(reader: BinaryReader) -> RunMode:
    """Decode one RunMode."""
    variant = reader.read_number()

    if variant == 0:
        return RunModeProgram()
    elif variant == 1:
        print = reader.read_bool()

        return RunModeEval(
            print=print,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_run_mode(value: RunMode) -> Json:
    """Return one JSON value for one RunMode."""
    if value.kind == "program":
        return {
            "kind": "program",
        }
    elif value.kind == "eval":
        return {
            "kind": "eval",
            "print": value.print,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_run_mode(value: Json) -> RunMode:
    """Return one RunMode from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "program":
        return RunModeProgram()
    elif kind == "eval":
        return RunModeEval(
            print=json_bool(json_field(object_, "print")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class RunPayloadValue:
    """Run completed successfully with a return value."""

    # returned value from the entry function
    value: typing.Any
    kind: typing.Literal["value"] = "value"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_run_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_run_payload(self)


@dataclass(frozen=True, slots=True)
class RunPayloadRuntimeError:
    """Run failed with a runtime error."""

    # runtime error string
    message: str
    kind: typing.Literal["runtimeError"] = "runtimeError"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_run_payload(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_run_payload(self)


"""Payload for run command output."""
RunPayload: typing.TypeAlias = RunPayloadValue | RunPayloadRuntimeError


def encode_run_payload(writer: BinaryWriter, value: RunPayload) -> None:
    """Encode one RunPayload."""
    if value.kind == "value":
        writer.write_unsigned(0)
        writer.write_json(value.value)
    elif value.kind == "runtimeError":
        writer.write_unsigned(1)
        writer.write_string(value.message)
    else:
        raise SerdeError("unknown enum variant")


def decode_run_payload(reader: BinaryReader) -> RunPayload:
    """Decode one RunPayload."""
    variant = reader.read_number()

    if variant == 0:
        value_ = reader.read_json()

        return RunPayloadValue(
            value=value_,
        )
    elif variant == 1:
        message = reader.read_string()

        return RunPayloadRuntimeError(
            message=message,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_run_payload(value: RunPayload) -> Json:
    """Return one JSON value for one RunPayload."""
    if value.kind == "value":
        return {
            "kind": "value",
            "value": value.value,
        }
    elif value.kind == "runtimeError":
        return {
            "kind": "runtimeError",
            "message": value.message,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_run_payload(value: Json) -> RunPayload:
    """Return one RunPayload from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "value":
        return RunPayloadValue(
            value=json_field(object_, "value"),
        )
    elif kind == "runtimeError":
        return RunPayloadRuntimeError(
            message=json_string(json_field(object_, "message")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "RunInput",
    "encode_run_input",
    "decode_run_input",
    "to_json_run_input",
    "from_json_run_input",
    "RunMode",
    "encode_run_mode",
    "decode_run_mode",
    "to_json_run_mode",
    "from_json_run_mode",
    "RunModeProgram",
    "RunModeEval",
    "RunPayload",
    "encode_run_payload",
    "decode_run_payload",
    "to_json_run_payload",
    "from_json_run_payload",
    "RunPayloadValue",
    "RunPayloadRuntimeError",
]
