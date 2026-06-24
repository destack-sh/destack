# generated bridge target, do not edit

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
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.protocol.workspace.command.common


@dataclass(frozen=True, slots=True)
class DoctorInput:
    """Request to return workspace health information."""

    # revision selected for this doctor request
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
    # whether to run extended checks
    full: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_doctor_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DoctorInput:
        """Decode one DoctorInput."""
        return decode_doctor_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_doctor_input(self)

    @classmethod
    def from_json(cls, value: Json) -> DoctorInput:
        """Return one DoctorInput from one JSON value."""
        return from_json_doctor_input(value)


def encode_doctor_input(writer: BinaryWriter, value: DoctorInput) -> None:
    """Encode one DoctorInput."""
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
    writer.write_bool(value.full)


def decode_doctor_input(reader: BinaryReader) -> DoctorInput:
    """Decode one DoctorInput."""
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
    full = reader.read_bool()

    return DoctorInput(
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
        full=full,
    )


def to_json_doctor_input(value: DoctorInput) -> Json:
    """Return one JSON value for one DoctorInput."""
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
        "full": value.full,
    }


def from_json_doctor_input(value: Json) -> DoctorInput:
    """Return one DoctorInput from one JSON value."""
    object_ = json_object(value)

    return DoctorInput(
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
        full=json_bool(json_field(object_, "full")),
    )


@dataclass(frozen=True, slots=True)
class DoctorPayload:
    """Payload for doctor command output."""

    # CLI version string
    cli_version: str
    # working directory
    cwd: str
    # host operating system
    os: str
    # host architecture
    arch: str
    # worker count configured
    workers: int
    # detected parallelism
    available_parallelism: int
    # workspace metadata
    workspace: DoctorWorkspace
    # resolved destack.json path
    manifest: str | None
    # manifest extends entries
    extends: Sequence[str] | None
    # default target name
    default_target: str | None
    # target count
    target_count: int
    # target names when requested
    targets: Sequence[str] | None
    # tool checks when requested
    tools: Sequence[DoctorTool] | None
    # warning list
    warnings: Sequence[str] | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_doctor_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DoctorPayload:
        """Decode one DoctorPayload."""
        return decode_doctor_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_doctor_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> DoctorPayload:
        """Return one DoctorPayload from one JSON value."""
        return from_json_doctor_payload(value)


def encode_doctor_payload(writer: BinaryWriter, value: DoctorPayload) -> None:
    """Encode one DoctorPayload."""
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
        for item_value_extends_1 in value.extends:
            writer.write_string(item_value_extends_1)
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
        for item_value_targets_1 in value.targets:
            writer.write_string(item_value_targets_1)
    if value.tools is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.tools))
        for item_value_tools_1 in value.tools:
            encode_doctor_tool(writer, item_value_tools_1)
    if value.warnings is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.warnings))
        for item_value_warnings_1 in value.warnings:
            writer.write_string(item_value_warnings_1)


def decode_doctor_payload(reader: BinaryReader) -> DoctorPayload:
    """Decode one DoctorPayload."""
    cli_version = reader.read_string()
    cwd = reader.read_string()
    os = reader.read_string()
    arch = reader.read_string()
    workers = reader.read_number()
    available_parallelism = reader.read_number()
    workspace = decode_doctor_workspace(reader)
    manifest = reader.read_option(lambda: reader.read_string())
    extends = reader.read_option(
        lambda: [reader.read_string() for _ in range(reader.read_number())]
    )
    default_target = reader.read_option(lambda: reader.read_string())
    target_count = reader.read_number()
    targets = reader.read_option(
        lambda: [reader.read_string() for _ in range(reader.read_number())]
    )
    tools = reader.read_option(
        lambda: [decode_doctor_tool(reader) for _ in range(reader.read_number())]
    )
    warnings = reader.read_option(
        lambda: [reader.read_string() for _ in range(reader.read_number())]
    )

    return DoctorPayload(
        cli_version=cli_version,
        cwd=cwd,
        os=os,
        arch=arch,
        workers=workers,
        available_parallelism=available_parallelism,
        workspace=workspace,
        manifest=manifest,
        extends=extends,
        default_target=default_target,
        target_count=target_count,
        targets=targets,
        tools=tools,
        warnings=warnings,
    )


def to_json_doctor_payload(value: DoctorPayload) -> Json:
    """Return one JSON value for one DoctorPayload."""
    return {
        "cliVersion": value.cli_version,
        "cwd": value.cwd,
        "os": value.os,
        "arch": value.arch,
        "workers": value.workers,
        "availableParallelism": value.available_parallelism,
        "workspace": to_json_doctor_workspace(value.workspace),
        **({} if value.manifest is None else {"manifest": value.manifest}),
        **(
            {}
            if value.extends is None
            else {"extends": [item_0 for item_0 in value.extends]}
        ),
        **(
            {}
            if value.default_target is None
            else {"defaultTarget": value.default_target}
        ),
        "targetCount": value.target_count,
        **(
            {}
            if value.targets is None
            else {"targets": [item_0 for item_0 in value.targets]}
        ),
        **(
            {}
            if value.tools is None
            else {"tools": [to_json_doctor_tool(item_0) for item_0 in value.tools]}
        ),
        **(
            {}
            if value.warnings is None
            else {"warnings": [item_0 for item_0 in value.warnings]}
        ),
    }


def from_json_doctor_payload(value: Json) -> DoctorPayload:
    """Return one DoctorPayload from one JSON value."""
    object_ = json_object(value)

    return DoctorPayload(
        cli_version=json_string(json_field(object_, "cliVersion")),
        cwd=json_string(json_field(object_, "cwd")),
        os=json_string(json_field(object_, "os")),
        arch=json_string(json_field(object_, "arch")),
        workers=json_int(json_field(object_, "workers")),
        available_parallelism=json_int(json_field(object_, "availableParallelism")),
        workspace=from_json_doctor_workspace(json_field(object_, "workspace")),
        manifest=json_optional(object_, "manifest", lambda value: json_string(value)),
        extends=json_optional(
            object_,
            "extends",
            lambda value: [json_string(item_0) for item_0 in json_array(value)],
        ),
        default_target=json_optional(
            object_, "defaultTarget", lambda value: json_string(value)
        ),
        target_count=json_int(json_field(object_, "targetCount")),
        targets=json_optional(
            object_,
            "targets",
            lambda value: [json_string(item_0) for item_0 in json_array(value)],
        ),
        tools=json_optional(
            object_,
            "tools",
            lambda value: [
                from_json_doctor_tool(item_0) for item_0 in json_array(value)
            ],
        ),
        warnings=json_optional(
            object_,
            "warnings",
            lambda value: [json_string(item_0) for item_0 in json_array(value)],
        ),
    )


@dataclass(frozen=True, slots=True)
class DoctorWorkspace:
    """Workspace details for doctor output."""

    # workspace root path
    root: str
    # workspace kind label
    kind: str
    # number of packages
    package_count: int
    # package paths when requested
    packages: Sequence[str] | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_doctor_workspace(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DoctorWorkspace:
        """Decode one DoctorWorkspace."""
        return decode_doctor_workspace(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_doctor_workspace(self)

    @classmethod
    def from_json(cls, value: Json) -> DoctorWorkspace:
        """Return one DoctorWorkspace from one JSON value."""
        return from_json_doctor_workspace(value)


def encode_doctor_workspace(writer: BinaryWriter, value: DoctorWorkspace) -> None:
    """Encode one DoctorWorkspace."""
    writer.write_string(value.root)
    writer.write_string(value.kind)
    writer.write_unsigned(value.package_count)
    if value.packages is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.packages))
        for item_value_packages_1 in value.packages:
            writer.write_string(item_value_packages_1)


def decode_doctor_workspace(reader: BinaryReader) -> DoctorWorkspace:
    """Decode one DoctorWorkspace."""
    root = reader.read_string()
    kind = reader.read_string()
    package_count = reader.read_number()
    packages = reader.read_option(
        lambda: [reader.read_string() for _ in range(reader.read_number())]
    )

    return DoctorWorkspace(
        root=root,
        kind=kind,
        package_count=package_count,
        packages=packages,
    )


def to_json_doctor_workspace(value: DoctorWorkspace) -> Json:
    """Return one JSON value for one DoctorWorkspace."""
    return {
        "root": value.root,
        "kind": value.kind,
        "packageCount": value.package_count,
        **(
            {}
            if value.packages is None
            else {"packages": [item_0 for item_0 in value.packages]}
        ),
    }


def from_json_doctor_workspace(value: Json) -> DoctorWorkspace:
    """Return one DoctorWorkspace from one JSON value."""
    object_ = json_object(value)

    return DoctorWorkspace(
        root=json_string(json_field(object_, "root")),
        kind=json_string(json_field(object_, "kind")),
        package_count=json_int(json_field(object_, "packageCount")),
        packages=json_optional(
            object_,
            "packages",
            lambda value: [json_string(item_0) for item_0 in json_array(value)],
        ),
    )


@dataclass(frozen=True, slots=True)
class DoctorTool:
    """Tool probe entry for doctor output."""

    # tool name
    name: str
    # tool version string
    version: str | None
    # tool probe status
    status: DoctorToolStatus

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_doctor_tool(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DoctorTool:
        """Decode one DoctorTool."""
        return decode_doctor_tool(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_doctor_tool(self)

    @classmethod
    def from_json(cls, value: Json) -> DoctorTool:
        """Return one DoctorTool from one JSON value."""
        return from_json_doctor_tool(value)


def encode_doctor_tool(writer: BinaryWriter, value: DoctorTool) -> None:
    """Encode one DoctorTool."""
    writer.write_string(value.name)
    if value.version is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.version)
    encode_doctor_tool_status(writer, value.status)


def decode_doctor_tool(reader: BinaryReader) -> DoctorTool:
    """Decode one DoctorTool."""
    name = reader.read_string()
    version = reader.read_option(lambda: reader.read_string())
    status = decode_doctor_tool_status(reader)

    return DoctorTool(
        name=name,
        version=version,
        status=status,
    )


def to_json_doctor_tool(value: DoctorTool) -> Json:
    """Return one JSON value for one DoctorTool."""
    return {
        "name": value.name,
        **({} if value.version is None else {"version": value.version}),
        "status": to_json_doctor_tool_status(value.status),
    }


def from_json_doctor_tool(value: Json) -> DoctorTool:
    """Return one DoctorTool from one JSON value."""
    object_ = json_object(value)

    return DoctorTool(
        name=json_string(json_field(object_, "name")),
        version=json_optional(object_, "version", lambda value: json_string(value)),
        status=from_json_doctor_tool_status(json_field(object_, "status")),
    )


"""Status values for tool detection."""
DoctorToolStatus: typing.TypeAlias = (
    typing.Literal["available"] | typing.Literal["missing"] | typing.Literal["error"]
)


def encode_doctor_tool_status(writer: BinaryWriter, value: DoctorToolStatus) -> None:
    """Encode one DoctorToolStatus."""
    if value == "available":
        writer.write_unsigned(0)
    elif value == "missing":
        writer.write_unsigned(1)
    elif value == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_doctor_tool_status(reader: BinaryReader) -> DoctorToolStatus:
    """Decode one DoctorToolStatus."""
    variant = reader.read_number()

    if variant == 0:
        return "available"
    elif variant == 1:
        return "missing"
    elif variant == 2:
        return "error"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_doctor_tool_status(value: DoctorToolStatus) -> Json:
    """Return one JSON value for one DoctorToolStatus."""
    return value


def from_json_doctor_tool_status(value: Json) -> DoctorToolStatus:
    """Return one DoctorToolStatus from one JSON value."""
    variant = json_string(value)

    if variant == "available":
        return "available"
    elif variant == "missing":
        return "missing"
    elif variant == "error":
        return "error"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "DoctorInput",
    "encode_doctor_input",
    "decode_doctor_input",
    "to_json_doctor_input",
    "from_json_doctor_input",
    "DoctorPayload",
    "encode_doctor_payload",
    "decode_doctor_payload",
    "to_json_doctor_payload",
    "from_json_doctor_payload",
    "DoctorWorkspace",
    "encode_doctor_workspace",
    "decode_doctor_workspace",
    "to_json_doctor_workspace",
    "from_json_doctor_workspace",
    "DoctorTool",
    "encode_doctor_tool",
    "decode_doctor_tool",
    "to_json_doctor_tool",
    "from_json_doctor_tool",
    "DoctorToolStatus",
    "encode_doctor_tool_status",
    "decode_doctor_tool_status",
    "to_json_doctor_tool_status",
    "from_json_doctor_tool_status",
]
