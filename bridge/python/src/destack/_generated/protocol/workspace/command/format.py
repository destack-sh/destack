# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.file.model.file
import destack._generated.protocol.source.file.model.type
import destack._generated.protocol.workspace.command.common

if TYPE_CHECKING:
    from destack._generated.protocol.source.file.model.file import (
        ContentId,
    )

    from destack._generated.protocol.source.file.model.type import (
        FileType,
    )

    from destack._generated.protocol.workspace.command.common import (
        CommandEnvVar,
        CommandInput,
        CommandRevision,
        CommandTargetOverrides,
        ManifestOverride,
    )


@dataclass(frozen=True, slots=True)
class FormatInput:
    """Request to format source files or content."""

    """Revision selected for this format request."""
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
    """Formatting source."""
    source: FormatSource
    """Formatting mode."""
    mode: FormatMode


def encode_format_input(writer: Writer, value: FormatInput) -> None:
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
    encode_format_source(writer, value.source)
    encode_format_mode(writer, value.mode)


def decode_format_input(reader: Reader) -> FormatInput:
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
    field_12 = decode_format_source(reader)
    field_13 = decode_format_mode(reader)

    return FormatInput(
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
        source=field_12,
        mode=field_13,
    )


@dataclass(frozen=True, slots=True)
class FormatSourceFiles:
    """Format files or directories."""

    files: Sequence[str]
    kind: Literal["files"] = "files"


@dataclass(frozen=True, slots=True)
class FormatSourceOpenFile:
    """Format one open file."""

    open_file: str
    kind: Literal["openFile"] = "openFile"


@dataclass(frozen=True, slots=True)
class FormatSourceContent:
    """Format explicit content."""

    """Input label."""
    name: str
    """Input file type."""
    file_type: FileType
    """Content to format."""
    content: ContentId
    kind: Literal["content"] = "content"


"""Formatting source."""
FormatSource: TypeAlias = FormatSourceFiles | FormatSourceOpenFile | FormatSourceContent


def encode_format_source(writer: Writer, value: FormatSource) -> None:
    if value.kind == "files":
        writer.write_unsigned(0)
        writer.write_unsigned(len(value.files))
        for item_0 in value.files:
            writer.write_string(item_0)
    elif value.kind == "openFile":
        writer.write_unsigned(1)
        writer.write_string(value.open_file)
    elif value.kind == "content":
        writer.write_unsigned(2)
        writer.write_string(value.name)
        destack._generated.protocol.source.file.model.type.encode_file_type(
            writer, value.file_type
        )
        destack._generated.protocol.source.file.model.file.encode_content_id(
            writer, value.content
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_format_source(reader: Reader) -> FormatSource:
    variant = reader.read_number()

    if variant == 0:
        return FormatSourceFiles(
            files=[reader.read_string() for _ in range(reader.read_number())]
        )
    elif variant == 1:
        return FormatSourceOpenFile(open_file=reader.read_string())
    elif variant == 2:
        field_0 = reader.read_string()
        field_1 = destack._generated.protocol.source.file.model.type.decode_file_type(
            reader
        )
        field_2 = destack._generated.protocol.source.file.model.file.decode_content_id(
            reader
        )

        return FormatSourceContent(
            name=field_0,
            file_type=field_1,
            content=field_2,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


"""Formatting mode."""
FormatMode: TypeAlias = Literal["preview"] | Literal["check"] | Literal["write"]


def encode_format_mode(writer: Writer, value: FormatMode) -> None:
    if value == "preview":
        writer.write_unsigned(0)
    elif value == "check":
        writer.write_unsigned(1)
    elif value == "write":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_format_mode(reader: Reader) -> FormatMode:
    variant = reader.read_number()

    if variant == 0:
        return "preview"
    elif variant == 1:
        return "check"
    elif variant == 2:
        return "write"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class FormatPayload:
    """Payload for format command output."""

    """The number of files inspected."""
    files: int
    """The number of files that would change."""
    changed: int
    """Paths for files that changed or would change in check mode."""
    changed_files: Sequence[str]
    """The number of errors encountered."""
    errors: int
    """Paths for files that failed formatting."""
    error_files: Sequence[str]
    """Whether this was a check-only run."""
    check: bool
    """Formatted output for eval mode."""
    formatted: str | None


def encode_format_payload(writer: Writer, value: FormatPayload) -> None:
    writer.write_unsigned(value.files)
    writer.write_unsigned(value.changed)
    writer.write_unsigned(len(value.changed_files))
    for item_0 in value.changed_files:
        writer.write_string(item_0)
    writer.write_unsigned(value.errors)
    writer.write_unsigned(len(value.error_files))
    for item_0 in value.error_files:
        writer.write_string(item_0)
    writer.write_bool(value.check)
    if value.formatted is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.formatted)


def decode_format_payload(reader: Reader) -> FormatPayload:
    field_0 = reader.read_number()
    field_1 = reader.read_number()
    field_2 = [reader.read_string() for _ in range(reader.read_number())]
    field_3 = reader.read_number()
    field_4 = [reader.read_string() for _ in range(reader.read_number())]
    field_5 = reader.read_bool()
    field_6 = reader.read_option(lambda: reader.read_string())

    return FormatPayload(
        files=field_0,
        changed=field_1,
        changed_files=field_2,
        errors=field_3,
        error_files=field_4,
        check=field_5,
        formatted=field_6,
    )


__all__ = [
    "FormatInput",
    "encode_format_input",
    "decode_format_input",
    "FormatSource",
    "encode_format_source",
    "decode_format_source",
    "FormatSourceFiles",
    "FormatSourceOpenFile",
    "FormatSourceContent",
    "FormatMode",
    "encode_format_mode",
    "decode_format_mode",
    "FormatPayload",
    "encode_format_payload",
    "decode_format_payload",
]
