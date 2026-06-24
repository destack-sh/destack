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
import destack._generated.source.file.model.file
import destack._generated.source.file.model.type


@dataclass(frozen=True, slots=True)
class FormatInput:
    """Request to format source files or content."""

    # revision selected for this format request
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
    # formatting source
    source: FormatSource
    # formatting mode
    mode: FormatMode

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_format_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FormatInput:
        """Decode one FormatInput."""
        return decode_format_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_format_input(self)

    @classmethod
    def from_json(cls, value: Json) -> FormatInput:
        """Return one FormatInput from one JSON value."""
        return from_json_format_input(value)


def encode_format_input(writer: BinaryWriter, value: FormatInput) -> None:
    """Encode one FormatInput."""
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
    encode_format_source(writer, value.source)
    encode_format_mode(writer, value.mode)


def decode_format_input(reader: BinaryReader) -> FormatInput:
    """Decode one FormatInput."""
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
    source = decode_format_source(reader)
    mode = decode_format_mode(reader)

    return FormatInput(
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
        source=source,
        mode=mode,
    )


def to_json_format_input(value: FormatInput) -> Json:
    """Return one JSON value for one FormatInput."""
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
        "source": to_json_format_source(value.source),
        "mode": to_json_format_mode(value.mode),
    }


def from_json_format_input(value: Json) -> FormatInput:
    """Return one FormatInput from one JSON value."""
    object_ = json_object(value)

    return FormatInput(
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
        source=from_json_format_source(json_field(object_, "source")),
        mode=from_json_format_mode(json_field(object_, "mode")),
    )


@dataclass(frozen=True, slots=True)
class FormatSourceFiles:
    """Format files or directories."""

    files: Sequence[str]
    kind: typing.Literal["files"] = "files"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_format_source(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_format_source(self)


@dataclass(frozen=True, slots=True)
class FormatSourceOpenFile:
    """Format one open file."""

    open_file: str
    kind: typing.Literal["openFile"] = "openFile"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_format_source(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_format_source(self)


@dataclass(frozen=True, slots=True)
class FormatSourceContent:
    """Format explicit content."""

    # input label
    name: str
    # input file type
    file_type: destack._generated.source.file.model.type.FileType
    # content to format
    content: destack._generated.source.file.model.file.ContentId
    kind: typing.Literal["content"] = "content"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_format_source(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_format_source(self)


"""Formatting source."""
FormatSource: typing.TypeAlias = (
    FormatSourceFiles | FormatSourceOpenFile | FormatSourceContent
)


def encode_format_source(writer: BinaryWriter, value: FormatSource) -> None:
    """Encode one FormatSource."""
    if value.kind == "files":
        writer.write_unsigned(0)
        writer.write_unsigned(len(value.files))
        for item_value_files_0 in value.files:
            writer.write_string(item_value_files_0)
    elif value.kind == "openFile":
        writer.write_unsigned(1)
        writer.write_string(value.open_file)
    elif value.kind == "content":
        writer.write_unsigned(2)
        writer.write_string(value.name)
        destack._generated.source.file.model.type.encode_file_type(
            writer, value.file_type
        )
        destack._generated.source.file.model.file.encode_content_id(
            writer, value.content
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_format_source(reader: BinaryReader) -> FormatSource:
    """Decode one FormatSource."""
    variant = reader.read_number()

    if variant == 0:
        files = [reader.read_string() for _ in range(reader.read_number())]

        return FormatSourceFiles(files=files)
    elif variant == 1:
        open_file = reader.read_string()

        return FormatSourceOpenFile(open_file=open_file)
    elif variant == 2:
        name = reader.read_string()
        file_type = destack._generated.source.file.model.type.decode_file_type(reader)
        content = destack._generated.source.file.model.file.decode_content_id(reader)

        return FormatSourceContent(
            name=name,
            file_type=file_type,
            content=content,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_format_source(value: FormatSource) -> Json:
    """Return one JSON value for one FormatSource."""
    if value.kind == "files":
        return {
            "kind": "files",
            "files": [item_0 for item_0 in value.files],
        }
    elif value.kind == "openFile":
        return {
            "kind": "openFile",
            "open_file": value.open_file,
        }
    elif value.kind == "content":
        return {
            "kind": "content",
            "name": value.name,
            "fileType": destack._generated.source.file.model.type.to_json_file_type(
                value.file_type
            ),
            "content": destack._generated.source.file.model.file.to_json_content_id(
                value.content
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_format_source(value: Json) -> FormatSource:
    """Return one FormatSource from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "files":
        return FormatSourceFiles(
            files=[
                json_string(item_0)
                for item_0 in json_array(json_field(object_, "files"))
            ]
        )
    elif kind == "openFile":
        return FormatSourceOpenFile(
            open_file=json_string(json_field(object_, "open_file"))
        )
    elif kind == "content":
        return FormatSourceContent(
            name=json_string(json_field(object_, "name")),
            file_type=destack._generated.source.file.model.type.from_json_file_type(
                json_field(object_, "fileType")
            ),
            content=destack._generated.source.file.model.file.from_json_content_id(
                json_field(object_, "content")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Formatting mode."""
FormatMode: typing.TypeAlias = (
    typing.Literal["preview"] | typing.Literal["check"] | typing.Literal["write"]
)


def encode_format_mode(writer: BinaryWriter, value: FormatMode) -> None:
    """Encode one FormatMode."""
    if value == "preview":
        writer.write_unsigned(0)
    elif value == "check":
        writer.write_unsigned(1)
    elif value == "write":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_format_mode(reader: BinaryReader) -> FormatMode:
    """Decode one FormatMode."""
    variant = reader.read_number()

    if variant == 0:
        return "preview"
    elif variant == 1:
        return "check"
    elif variant == 2:
        return "write"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_format_mode(value: FormatMode) -> Json:
    """Return one JSON value for one FormatMode."""
    return value


def from_json_format_mode(value: Json) -> FormatMode:
    """Return one FormatMode from one JSON value."""
    variant = json_string(value)

    if variant == "preview":
        return "preview"
    elif variant == "check":
        return "check"
    elif variant == "write":
        return "write"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class FormatPayload:
    """Payload for format command output."""

    # the number of files inspected
    files: int
    # the number of files that would change
    changed: int
    # paths for files that changed or would change in check mode
    changed_files: Sequence[str]
    # the number of errors encountered
    errors: int
    # paths for files that failed formatting
    error_files: Sequence[str]
    # whether this was a check-only run
    check: bool
    # formatted output for eval mode
    formatted: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_format_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FormatPayload:
        """Decode one FormatPayload."""
        return decode_format_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_format_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> FormatPayload:
        """Return one FormatPayload from one JSON value."""
        return from_json_format_payload(value)


def encode_format_payload(writer: BinaryWriter, value: FormatPayload) -> None:
    """Encode one FormatPayload."""
    writer.write_unsigned(value.files)
    writer.write_unsigned(value.changed)
    writer.write_unsigned(len(value.changed_files))
    for item_value_changed_files_0 in value.changed_files:
        writer.write_string(item_value_changed_files_0)
    writer.write_unsigned(value.errors)
    writer.write_unsigned(len(value.error_files))
    for item_value_error_files_0 in value.error_files:
        writer.write_string(item_value_error_files_0)
    writer.write_bool(value.check)
    if value.formatted is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.formatted)


def decode_format_payload(reader: BinaryReader) -> FormatPayload:
    """Decode one FormatPayload."""
    files = reader.read_number()
    changed = reader.read_number()
    changed_files = [reader.read_string() for _ in range(reader.read_number())]
    errors = reader.read_number()
    error_files = [reader.read_string() for _ in range(reader.read_number())]
    check = reader.read_bool()
    formatted = reader.read_option(lambda: reader.read_string())

    return FormatPayload(
        files=files,
        changed=changed,
        changed_files=changed_files,
        errors=errors,
        error_files=error_files,
        check=check,
        formatted=formatted,
    )


def to_json_format_payload(value: FormatPayload) -> Json:
    """Return one JSON value for one FormatPayload."""
    return {
        "files": value.files,
        "changed": value.changed,
        "changedFiles": [item_0 for item_0 in value.changed_files],
        "errors": value.errors,
        "errorFiles": [item_0 for item_0 in value.error_files],
        "check": value.check,
        **({} if value.formatted is None else {"formatted": value.formatted}),
    }


def from_json_format_payload(value: Json) -> FormatPayload:
    """Return one FormatPayload from one JSON value."""
    object_ = json_object(value)

    return FormatPayload(
        files=json_int(json_field(object_, "files")),
        changed=json_int(json_field(object_, "changed")),
        changed_files=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "changedFiles"))
        ],
        errors=json_int(json_field(object_, "errors")),
        error_files=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "errorFiles"))
        ],
        check=json_bool(json_field(object_, "check")),
        formatted=json_optional(object_, "formatted", lambda value: json_string(value)),
    )


__all__ = [
    "FormatInput",
    "encode_format_input",
    "decode_format_input",
    "to_json_format_input",
    "from_json_format_input",
    "FormatSource",
    "encode_format_source",
    "decode_format_source",
    "to_json_format_source",
    "from_json_format_source",
    "FormatSourceFiles",
    "FormatSourceOpenFile",
    "FormatSourceContent",
    "FormatMode",
    "encode_format_mode",
    "decode_format_mode",
    "to_json_format_mode",
    "from_json_format_mode",
    "FormatPayload",
    "encode_format_payload",
    "decode_format_payload",
    "to_json_format_payload",
    "from_json_format_payload",
]
