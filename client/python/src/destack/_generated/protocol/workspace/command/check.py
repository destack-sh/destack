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
import destack._generated.repository.provider.trace


@dataclass(frozen=True, slots=True)
class CheckInput:
    """Request to check source state."""

    # revision selected for this check
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
    # whether linting should run when supported
    lint: bool
    # apply lint fixes
    fix: bool
    # include unsafe lint fixes
    unsafe_fixes: bool
    # show lint diff instead of applying fixes
    diff: bool
    # trace detail returned in the response
    trace: destack._generated.repository.provider.trace.TraceView

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CheckInput:
        """Decode one CheckInput."""
        return decode_check_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_input(self)

    @classmethod
    def from_json(cls, value: Json) -> CheckInput:
        """Return one CheckInput from one JSON value."""
        return from_json_check_input(value)


def encode_check_input(writer: BinaryWriter, value: CheckInput) -> None:
    """Encode one CheckInput."""
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
    writer.write_bool(value.lint)
    writer.write_bool(value.fix)
    writer.write_bool(value.unsafe_fixes)
    writer.write_bool(value.diff)
    destack._generated.repository.provider.trace.encode_trace_view(writer, value.trace)


def decode_check_input(reader: BinaryReader) -> CheckInput:
    """Decode one CheckInput."""
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
    lint = reader.read_bool()
    fix = reader.read_bool()
    unsafe_fixes = reader.read_bool()
    diff = reader.read_bool()
    trace = destack._generated.repository.provider.trace.decode_trace_view(reader)

    return CheckInput(
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
        lint=lint,
        fix=fix,
        unsafe_fixes=unsafe_fixes,
        diff=diff,
        trace=trace,
    )


def to_json_check_input(value: CheckInput) -> Json:
    """Return one JSON value for one CheckInput."""
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
        "lint": value.lint,
        "fix": value.fix,
        "unsafeFixes": value.unsafe_fixes,
        "diff": value.diff,
        "trace": destack._generated.repository.provider.trace.to_json_trace_view(
            value.trace
        ),
    }


def from_json_check_input(value: Json) -> CheckInput:
    """Return one CheckInput from one JSON value."""
    object_ = json_object(value)

    return CheckInput(
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
        lint=json_bool(json_field(object_, "lint")),
        fix=json_bool(json_field(object_, "fix")),
        unsafe_fixes=json_bool(json_field(object_, "unsafeFixes")),
        diff=json_bool(json_field(object_, "diff")),
        trace=destack._generated.repository.provider.trace.from_json_trace_view(
            json_field(object_, "trace")
        ),
    )


@dataclass(frozen=True, slots=True)
class LintInput:
    """Request to lint source state."""

    # revision selected for this lint run
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
    # apply fixes
    fix: bool
    # include unsafe fixes
    unsafe_fixes: bool
    # show diff instead of applying fixes
    diff: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lint_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LintInput:
        """Decode one LintInput."""
        return decode_lint_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lint_input(self)

    @classmethod
    def from_json(cls, value: Json) -> LintInput:
        """Return one LintInput from one JSON value."""
        return from_json_lint_input(value)


def encode_lint_input(writer: BinaryWriter, value: LintInput) -> None:
    """Encode one LintInput."""
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
    writer.write_bool(value.fix)
    writer.write_bool(value.unsafe_fixes)
    writer.write_bool(value.diff)


def decode_lint_input(reader: BinaryReader) -> LintInput:
    """Decode one LintInput."""
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
    fix = reader.read_bool()
    unsafe_fixes = reader.read_bool()
    diff = reader.read_bool()

    return LintInput(
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
        fix=fix,
        unsafe_fixes=unsafe_fixes,
        diff=diff,
    )


def to_json_lint_input(value: LintInput) -> Json:
    """Return one JSON value for one LintInput."""
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
        "fix": value.fix,
        "unsafeFixes": value.unsafe_fixes,
        "diff": value.diff,
    }


def from_json_lint_input(value: Json) -> LintInput:
    """Return one LintInput from one JSON value."""
    object_ = json_object(value)

    return LintInput(
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
        fix=json_bool(json_field(object_, "fix")),
        unsafe_fixes=json_bool(json_field(object_, "unsafeFixes")),
        diff=json_bool(json_field(object_, "diff")),
    )


@dataclass(frozen=True, slots=True)
class CheckPayload:
    """Payload for check command output."""

    # trace payload for this check
    trace: destack._generated.repository.provider.trace.TraceSnapshot

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CheckPayload:
        """Decode one CheckPayload."""
        return decode_check_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> CheckPayload:
        """Return one CheckPayload from one JSON value."""
        return from_json_check_payload(value)


def encode_check_payload(writer: BinaryWriter, value: CheckPayload) -> None:
    """Encode one CheckPayload."""
    destack._generated.repository.provider.trace.encode_trace_snapshot(
        writer, value.trace
    )


def decode_check_payload(reader: BinaryReader) -> CheckPayload:
    """Decode one CheckPayload."""
    trace = destack._generated.repository.provider.trace.decode_trace_snapshot(reader)

    return CheckPayload(
        trace=trace,
    )


def to_json_check_payload(value: CheckPayload) -> Json:
    """Return one JSON value for one CheckPayload."""
    return {
        "trace": destack._generated.repository.provider.trace.to_json_trace_snapshot(
            value.trace
        ),
    }


def from_json_check_payload(value: Json) -> CheckPayload:
    """Return one CheckPayload from one JSON value."""
    object_ = json_object(value)

    return CheckPayload(
        trace=destack._generated.repository.provider.trace.from_json_trace_snapshot(
            json_field(object_, "trace")
        ),
    )


@dataclass(frozen=True, slots=True)
class LintPayload:
    """Payload for lint command output."""

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lint_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LintPayload:
        """Decode one LintPayload."""
        return decode_lint_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lint_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> LintPayload:
        """Return one LintPayload from one JSON value."""
        return from_json_lint_payload(value)


def encode_lint_payload(writer: BinaryWriter, value: LintPayload) -> None:
    """Encode one LintPayload."""
    pass


def decode_lint_payload(reader: BinaryReader) -> LintPayload:
    """Decode one LintPayload."""
    return LintPayload()


def to_json_lint_payload(value: LintPayload) -> Json:
    """Return one JSON value for one LintPayload."""
    return {}


def from_json_lint_payload(value: Json) -> LintPayload:
    """Return one LintPayload from one JSON value."""
    object_ = json_object(value)
    return LintPayload()


__all__ = [
    "CheckInput",
    "encode_check_input",
    "decode_check_input",
    "to_json_check_input",
    "from_json_check_input",
    "LintInput",
    "encode_lint_input",
    "decode_lint_input",
    "to_json_lint_input",
    "from_json_lint_input",
    "CheckPayload",
    "encode_check_payload",
    "decode_check_payload",
    "to_json_check_payload",
    "from_json_check_payload",
    "LintPayload",
    "encode_lint_payload",
    "decode_lint_payload",
    "to_json_lint_payload",
    "from_json_lint_payload",
]
