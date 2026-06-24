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
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.protocol.workspace.command.common


@dataclass(frozen=True, slots=True)
class TaskInput:
    """Request to run workspace tasks."""

    # revision selected for this task request
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
    # task action to run
    action: TaskAction
    # selected projects
    projects: Sequence[str]
    # selected workspace groups
    groups: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_task_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TaskInput:
        """Decode one TaskInput."""
        return decode_task_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_task_input(self)

    @classmethod
    def from_json(cls, value: Json) -> TaskInput:
        """Return one TaskInput from one JSON value."""
        return from_json_task_input(value)


def encode_task_input(writer: BinaryWriter, value: TaskInput) -> None:
    """Encode one TaskInput."""
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
    encode_task_action(writer, value.action)
    writer.write_unsigned(len(value.projects))
    for item_value_projects_0 in value.projects:
        writer.write_string(item_value_projects_0)
    writer.write_unsigned(len(value.groups))
    for item_value_groups_0 in value.groups:
        writer.write_string(item_value_groups_0)


def decode_task_input(reader: BinaryReader) -> TaskInput:
    """Decode one TaskInput."""
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
    action = decode_task_action(reader)
    projects = [reader.read_string() for _ in range(reader.read_number())]
    groups = [reader.read_string() for _ in range(reader.read_number())]

    return TaskInput(
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
        action=action,
        projects=projects,
        groups=groups,
    )


def to_json_task_input(value: TaskInput) -> Json:
    """Return one JSON value for one TaskInput."""
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
        "action": to_json_task_action(value.action),
        "projects": [item_0 for item_0 in value.projects],
        "groups": [item_0 for item_0 in value.groups],
    }


def from_json_task_input(value: Json) -> TaskInput:
    """Return one TaskInput from one JSON value."""
    object_ = json_object(value)

    return TaskInput(
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
        action=from_json_task_action(json_field(object_, "action")),
        projects=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "projects"))
        ],
        groups=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "groups"))
        ],
    )


@dataclass(frozen=True, slots=True)
class TaskActionList:
    """List available tasks."""

    kind: typing.Literal["list"] = "list"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_task_action(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_task_action(self)


@dataclass(frozen=True, slots=True)
class TaskActionRun:
    """Run a task by name."""

    # task name
    name: str
    # task arguments
    args: Sequence[str]
    # whether to only print the command
    dry_run: bool
    kind: typing.Literal["run"] = "run"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_task_action(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_task_action(self)


"""Task selection for the task command."""
TaskAction: typing.TypeAlias = TaskActionList | TaskActionRun


def encode_task_action(writer: BinaryWriter, value: TaskAction) -> None:
    """Encode one TaskAction."""
    if value.kind == "list":
        writer.write_unsigned(0)
    elif value.kind == "run":
        writer.write_unsigned(1)
        writer.write_string(value.name)
        writer.write_unsigned(len(value.args))
        for item_value_args_0 in value.args:
            writer.write_string(item_value_args_0)
        writer.write_bool(value.dry_run)
    else:
        raise SerdeError("unknown enum variant")


def decode_task_action(reader: BinaryReader) -> TaskAction:
    """Decode one TaskAction."""
    variant = reader.read_number()

    if variant == 0:
        return TaskActionList()
    elif variant == 1:
        name = reader.read_string()
        args = [reader.read_string() for _ in range(reader.read_number())]
        dry_run = reader.read_bool()

        return TaskActionRun(
            name=name,
            args=args,
            dry_run=dry_run,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_task_action(value: TaskAction) -> Json:
    """Return one JSON value for one TaskAction."""
    if value.kind == "list":
        return {
            "kind": "list",
        }
    elif value.kind == "run":
        return {
            "kind": "run",
            "name": value.name,
            "args": [item_0 for item_0 in value.args],
            "dryRun": value.dry_run,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_task_action(value: Json) -> TaskAction:
    """Return one TaskAction from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "list":
        return TaskActionList()
    elif kind == "run":
        return TaskActionRun(
            name=json_string(json_field(object_, "name")),
            args=[
                json_string(item_0)
                for item_0 in json_array(json_field(object_, "args"))
            ],
            dry_run=json_bool(json_field(object_, "dryRun")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TaskPayload:
    """Payload for task command output."""

    # task list entries
    tasks: Sequence[TaskEntry] | None
    # per-project task execution results
    results: Sequence[TaskResult] | None
    # exit code when executed
    exit_code: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_task_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TaskPayload:
        """Decode one TaskPayload."""
        return decode_task_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_task_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> TaskPayload:
        """Return one TaskPayload from one JSON value."""
        return from_json_task_payload(value)


def encode_task_payload(writer: BinaryWriter, value: TaskPayload) -> None:
    """Encode one TaskPayload."""
    if value.tasks is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.tasks))
        for item_value_tasks_1 in value.tasks:
            encode_task_entry(writer, item_value_tasks_1)
    if value.results is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.results))
        for item_value_results_1 in value.results:
            encode_task_result(writer, item_value_results_1)
    if value.exit_code is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_signed(value.exit_code)


def decode_task_payload(reader: BinaryReader) -> TaskPayload:
    """Decode one TaskPayload."""
    tasks = reader.read_option(
        lambda: [decode_task_entry(reader) for _ in range(reader.read_number())]
    )
    results = reader.read_option(
        lambda: [decode_task_result(reader) for _ in range(reader.read_number())]
    )
    exit_code = reader.read_option(lambda: reader.read_signed_number())

    return TaskPayload(
        tasks=tasks,
        results=results,
        exit_code=exit_code,
    )


def to_json_task_payload(value: TaskPayload) -> Json:
    """Return one JSON value for one TaskPayload."""
    return {
        **(
            {}
            if value.tasks is None
            else {"tasks": [to_json_task_entry(item_0) for item_0 in value.tasks]}
        ),
        **(
            {}
            if value.results is None
            else {"results": [to_json_task_result(item_0) for item_0 in value.results]}
        ),
        **({} if value.exit_code is None else {"exitCode": value.exit_code}),
    }


def from_json_task_payload(value: Json) -> TaskPayload:
    """Return one TaskPayload from one JSON value."""
    object_ = json_object(value)

    return TaskPayload(
        tasks=json_optional(
            object_,
            "tasks",
            lambda value: [
                from_json_task_entry(item_0) for item_0 in json_array(value)
            ],
        ),
        results=json_optional(
            object_,
            "results",
            lambda value: [
                from_json_task_result(item_0) for item_0 in json_array(value)
            ],
        ),
        exit_code=json_optional(object_, "exitCode", lambda value: json_int(value)),
    )


@dataclass(frozen=True, slots=True)
class TaskEntry:
    """Task entry for task list output."""

    # project identifier
    project: str
    # task name
    name: str
    # task description
    description: str | None
    # task source
    source: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_task_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TaskEntry:
        """Decode one TaskEntry."""
        return decode_task_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_task_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> TaskEntry:
        """Return one TaskEntry from one JSON value."""
        return from_json_task_entry(value)


def encode_task_entry(writer: BinaryWriter, value: TaskEntry) -> None:
    """Encode one TaskEntry."""
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


def decode_task_entry(reader: BinaryReader) -> TaskEntry:
    """Decode one TaskEntry."""
    project = reader.read_string()
    name = reader.read_string()
    description = reader.read_option(lambda: reader.read_string())
    source = reader.read_option(lambda: reader.read_string())

    return TaskEntry(
        project=project,
        name=name,
        description=description,
        source=source,
    )


def to_json_task_entry(value: TaskEntry) -> Json:
    """Return one JSON value for one TaskEntry."""
    return {
        "project": value.project,
        "name": value.name,
        **({} if value.description is None else {"description": value.description}),
        **({} if value.source is None else {"source": value.source}),
    }


def from_json_task_entry(value: Json) -> TaskEntry:
    """Return one TaskEntry from one JSON value."""
    object_ = json_object(value)

    return TaskEntry(
        project=json_string(json_field(object_, "project")),
        name=json_string(json_field(object_, "name")),
        description=json_optional(
            object_, "description", lambda value: json_string(value)
        ),
        source=json_optional(object_, "source", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class TaskResult:
    """One per-project task execution result."""

    # project identifier
    project: str
    # task name
    task: str
    # task command string
    command: str
    # task working directory
    cwd: str
    # whether this was a dry run
    dry_run: bool
    # task source
    source: str
    # exit code when executed
    exit_code: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_task_result(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TaskResult:
        """Decode one TaskResult."""
        return decode_task_result(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_task_result(self)

    @classmethod
    def from_json(cls, value: Json) -> TaskResult:
        """Return one TaskResult from one JSON value."""
        return from_json_task_result(value)


def encode_task_result(writer: BinaryWriter, value: TaskResult) -> None:
    """Encode one TaskResult."""
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


def decode_task_result(reader: BinaryReader) -> TaskResult:
    """Decode one TaskResult."""
    project = reader.read_string()
    task = reader.read_string()
    command = reader.read_string()
    cwd = reader.read_string()
    dry_run = reader.read_bool()
    source = reader.read_string()
    exit_code = reader.read_option(lambda: reader.read_signed_number())

    return TaskResult(
        project=project,
        task=task,
        command=command,
        cwd=cwd,
        dry_run=dry_run,
        source=source,
        exit_code=exit_code,
    )


def to_json_task_result(value: TaskResult) -> Json:
    """Return one JSON value for one TaskResult."""
    return {
        "project": value.project,
        "task": value.task,
        "command": value.command,
        "cwd": value.cwd,
        "dryRun": value.dry_run,
        "source": value.source,
        **({} if value.exit_code is None else {"exitCode": value.exit_code}),
    }


def from_json_task_result(value: Json) -> TaskResult:
    """Return one TaskResult from one JSON value."""
    object_ = json_object(value)

    return TaskResult(
        project=json_string(json_field(object_, "project")),
        task=json_string(json_field(object_, "task")),
        command=json_string(json_field(object_, "command")),
        cwd=json_string(json_field(object_, "cwd")),
        dry_run=json_bool(json_field(object_, "dryRun")),
        source=json_string(json_field(object_, "source")),
        exit_code=json_optional(object_, "exitCode", lambda value: json_int(value)),
    )


__all__ = [
    "TaskInput",
    "encode_task_input",
    "decode_task_input",
    "to_json_task_input",
    "from_json_task_input",
    "TaskAction",
    "encode_task_action",
    "decode_task_action",
    "to_json_task_action",
    "from_json_task_action",
    "TaskActionList",
    "TaskActionRun",
    "TaskPayload",
    "encode_task_payload",
    "decode_task_payload",
    "to_json_task_payload",
    "from_json_task_payload",
    "TaskEntry",
    "encode_task_entry",
    "decode_task_entry",
    "to_json_task_entry",
    "from_json_task_entry",
    "TaskResult",
    "encode_task_result",
    "decode_task_result",
    "to_json_task_result",
    "from_json_task_result",
]
