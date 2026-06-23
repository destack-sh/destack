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
class TaskInput:
    """Request to run workspace tasks."""

    """Revision selected for this task request."""
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
    """Task action to run."""
    action: TaskAction
    """Selected projects."""
    projects: Sequence[str]
    """Selected workspace groups."""
    groups: Sequence[str]


def encode_task_input(writer: Writer, value: TaskInput) -> None:
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
    encode_task_action(writer, value.action)
    writer.write_unsigned(len(value.projects))
    for item_0 in value.projects:
        writer.write_string(item_0)
    writer.write_unsigned(len(value.groups))
    for item_0 in value.groups:
        writer.write_string(item_0)


def decode_task_input(reader: Reader) -> TaskInput:
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
    field_12 = decode_task_action(reader)
    field_13 = [reader.read_string() for _ in range(reader.read_number())]
    field_14 = [reader.read_string() for _ in range(reader.read_number())]

    return TaskInput(
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
        action=field_12,
        projects=field_13,
        groups=field_14,
    )


@dataclass(frozen=True, slots=True)
class TaskActionList:
    """List available tasks."""

    kind: Literal["list"] = "list"


@dataclass(frozen=True, slots=True)
class TaskActionRun:
    """Run a task by name."""

    """Task name."""
    name: str
    """Task arguments."""
    args: Sequence[str]
    """Whether to only print the command."""
    dry_run: bool
    kind: Literal["run"] = "run"


"""Task selection for the task command."""
TaskAction: TypeAlias = TaskActionList | TaskActionRun


def encode_task_action(writer: Writer, value: TaskAction) -> None:
    if value.kind == "list":
        writer.write_unsigned(0)
    elif value.kind == "run":
        writer.write_unsigned(1)
        writer.write_string(value.name)
        writer.write_unsigned(len(value.args))
        for item_0 in value.args:
            writer.write_string(item_0)
        writer.write_bool(value.dry_run)
    else:
        raise SerdeError("unknown enum variant")


def decode_task_action(reader: Reader) -> TaskAction:
    variant = reader.read_number()

    if variant == 0:
        return TaskActionList()
    elif variant == 1:
        field_0 = reader.read_string()
        field_1 = [reader.read_string() for _ in range(reader.read_number())]
        field_2 = reader.read_bool()

        return TaskActionRun(
            name=field_0,
            args=field_1,
            dry_run=field_2,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class TaskPayload:
    """Payload for task command output."""

    """Task list entries."""
    tasks: Sequence[TaskEntry] | None
    """Per-project task execution results."""
    results: Sequence[TaskResult] | None
    """Exit code when executed."""
    exit_code: int | None


def encode_task_payload(writer: Writer, value: TaskPayload) -> None:
    if value.tasks is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.tasks))
        for item_1 in value.tasks:
            encode_task_entry(writer, item_1)
    if value.results is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.results))
        for item_1 in value.results:
            encode_task_result(writer, item_1)
    if value.exit_code is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_signed(value.exit_code)


def decode_task_payload(reader: Reader) -> TaskPayload:
    field_0 = reader.read_option(
        lambda: [decode_task_entry(reader) for _ in range(reader.read_number())]
    )
    field_1 = reader.read_option(
        lambda: [decode_task_result(reader) for _ in range(reader.read_number())]
    )
    field_2 = reader.read_option(lambda: reader.read_signed_number())

    return TaskPayload(
        tasks=field_0,
        results=field_1,
        exit_code=field_2,
    )


@dataclass(frozen=True, slots=True)
class TaskEntry:
    """Task entry for task list output."""

    """Project identifier."""
    project: str
    """Task name."""
    name: str
    """Task description."""
    description: str | None
    """Task source."""
    source: str | None


def encode_task_entry(writer: Writer, value: TaskEntry) -> None:
    writer.write_string(value.project)
    writer.write_string(value.name)
    if value.description is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.description)
    if value.source is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.source)


def decode_task_entry(reader: Reader) -> TaskEntry:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = reader.read_option(lambda: reader.read_string())
    field_3 = reader.read_option(lambda: reader.read_string())

    return TaskEntry(
        project=field_0,
        name=field_1,
        description=field_2,
        source=field_3,
    )


@dataclass(frozen=True, slots=True)
class TaskResult:
    """One per-project task execution result."""

    """Project identifier."""
    project: str
    """Task name."""
    task: str
    """Task command string."""
    command: str
    """Task working directory."""
    cwd: str
    """Whether this was a dry run."""
    dry_run: bool
    """Task source."""
    source: str
    """Exit code when executed."""
    exit_code: int | None


def encode_task_result(writer: Writer, value: TaskResult) -> None:
    writer.write_string(value.project)
    writer.write_string(value.task)
    writer.write_string(value.command)
    writer.write_string(value.cwd)
    writer.write_bool(value.dry_run)
    writer.write_string(value.source)
    if value.exit_code is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_signed(value.exit_code)


def decode_task_result(reader: Reader) -> TaskResult:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = reader.read_string()
    field_3 = reader.read_string()
    field_4 = reader.read_bool()
    field_5 = reader.read_string()
    field_6 = reader.read_option(lambda: reader.read_signed_number())

    return TaskResult(
        project=field_0,
        task=field_1,
        command=field_2,
        cwd=field_3,
        dry_run=field_4,
        source=field_5,
        exit_code=field_6,
    )


__all__ = [
    "TaskInput",
    "encode_task_input",
    "decode_task_input",
    "TaskAction",
    "encode_task_action",
    "decode_task_action",
    "TaskActionList",
    "TaskActionRun",
    "TaskPayload",
    "encode_task_payload",
    "decode_task_payload",
    "TaskEntry",
    "encode_task_entry",
    "decode_task_entry",
    "TaskResult",
    "encode_task_result",
    "decode_task_result",
]
