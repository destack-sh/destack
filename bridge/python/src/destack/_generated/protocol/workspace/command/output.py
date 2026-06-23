# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.repository.revision
import destack._generated.protocol.source.diagnostic.diagnostic
import destack._generated.protocol.workspace.command.build
import destack._generated.protocol.workspace.command.cache
import destack._generated.protocol.workspace.command.check
import destack._generated.protocol.workspace.command.clean
import destack._generated.protocol.workspace.command.common
import destack._generated.protocol.workspace.command.doctor
import destack._generated.protocol.workspace.command.format
import destack._generated.protocol.workspace.command.info
import destack._generated.protocol.workspace.command.run
import destack._generated.protocol.workspace.command.settings
import destack._generated.protocol.workspace.command.targets
import destack._generated.protocol.workspace.command.task
import destack._generated.protocol.workspace.file.image
import destack._generated.protocol.workspace.message

if TYPE_CHECKING:
    from destack._generated.protocol.repository.revision import (
        Revision,
    )

    from destack._generated.protocol.source.diagnostic.diagnostic import (
        Diagnostic,
    )

    from destack._generated.protocol.workspace.command.build import (
        BuildPayload,
    )

    from destack._generated.protocol.workspace.command.cache import (
        CachePayload,
    )

    from destack._generated.protocol.workspace.command.check import (
        CheckPayload,
        LintPayload,
    )

    from destack._generated.protocol.workspace.command.clean import (
        CleanPayload,
    )

    from destack._generated.protocol.workspace.command.common import (
        CommandMessagePayload,
        CommandOutputChunk,
        CommandOutputFile,
    )

    from destack._generated.protocol.workspace.command.doctor import (
        DoctorPayload,
    )

    from destack._generated.protocol.workspace.command.format import (
        FormatPayload,
    )

    from destack._generated.protocol.workspace.command.info import (
        InfoPayload,
    )

    from destack._generated.protocol.workspace.command.run import (
        RunPayload,
    )

    from destack._generated.protocol.workspace.command.settings import (
        SettingsPayload,
    )

    from destack._generated.protocol.workspace.command.targets import (
        TargetsPayload,
    )

    from destack._generated.protocol.workspace.command.task import (
        TaskPayload,
    )

    from destack._generated.protocol.workspace.file.image import (
        FileImage,
    )

    from destack._generated.protocol.workspace.message import (
        Message,
    )


@dataclass(frozen=True, slots=True)
class CheckOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: CheckPayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_check_output(writer: Writer, value: CheckOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.check.encode_check_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_check_output(reader: Reader) -> CheckOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = destack._generated.protocol.workspace.command.check.decode_check_payload(
        reader
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return CheckOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class LintOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: LintPayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_lint_output(writer: Writer, value: LintOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.check.encode_lint_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_lint_output(reader: Reader) -> LintOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = destack._generated.protocol.workspace.command.check.decode_lint_payload(
        reader
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return LintOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class FormatOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: FormatPayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_format_output(writer: Writer, value: FormatOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.format.encode_format_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_format_output(reader: Reader) -> FormatOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = (
        destack._generated.protocol.workspace.command.format.decode_format_payload(
            reader
        )
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return FormatOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class BuildOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: BuildPayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_build_output(writer: Writer, value: BuildOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.build.encode_build_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_build_output(reader: Reader) -> BuildOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = destack._generated.protocol.workspace.command.build.decode_build_payload(
        reader
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return BuildOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class RunOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: RunPayload | None
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_run_output(writer: Writer, value: RunOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    if value.data is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.workspace.command.run.encode_run_payload(
            writer, value.data
        )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_run_output(reader: Reader) -> RunOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = reader.read_option(
        lambda: destack._generated.protocol.workspace.command.run.decode_run_payload(
            reader
        )
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return RunOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class TestOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: CommandMessagePayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_test_output(writer: Writer, value: TestOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.common.encode_command_message_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_test_output(reader: Reader) -> TestOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = destack._generated.protocol.workspace.command.common.decode_command_message_payload(
        reader
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return TestOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class DocOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: CommandMessagePayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_doc_output(writer: Writer, value: DocOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.common.encode_command_message_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_doc_output(reader: Reader) -> DocOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = destack._generated.protocol.workspace.command.common.decode_command_message_payload(
        reader
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return DocOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class BenchOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: CommandMessagePayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_bench_output(writer: Writer, value: BenchOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.common.encode_command_message_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_bench_output(reader: Reader) -> BenchOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = destack._generated.protocol.workspace.command.common.decode_command_message_payload(
        reader
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return BenchOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class InfoOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: InfoPayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_info_output(writer: Writer, value: InfoOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.info.encode_info_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_info_output(reader: Reader) -> InfoOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = destack._generated.protocol.workspace.command.info.decode_info_payload(
        reader
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return InfoOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class TargetsOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: TargetsPayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_targets_output(writer: Writer, value: TargetsOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.targets.encode_targets_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_targets_output(reader: Reader) -> TargetsOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = (
        destack._generated.protocol.workspace.command.targets.decode_targets_payload(
            reader
        )
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return TargetsOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class CacheOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: CachePayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_cache_output(writer: Writer, value: CacheOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.cache.encode_cache_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_cache_output(reader: Reader) -> CacheOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = destack._generated.protocol.workspace.command.cache.decode_cache_payload(
        reader
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return CacheOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class SettingsOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: SettingsPayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_settings_output(writer: Writer, value: SettingsOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.settings.encode_settings_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_settings_output(reader: Reader) -> SettingsOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = (
        destack._generated.protocol.workspace.command.settings.decode_settings_payload(
            reader
        )
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return SettingsOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class DoctorOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: DoctorPayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_doctor_output(writer: Writer, value: DoctorOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.doctor.encode_doctor_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_doctor_output(reader: Reader) -> DoctorOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = (
        destack._generated.protocol.workspace.command.doctor.decode_doctor_payload(
            reader
        )
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return DoctorOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class TaskOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: TaskPayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_task_output(writer: Writer, value: TaskOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.task.encode_task_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_task_output(reader: Reader) -> TaskOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = destack._generated.protocol.workspace.command.task.decode_task_payload(
        reader
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return TaskOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


@dataclass(frozen=True, slots=True)
class CleanOutput:
    """Output produced by one workspace command."""

    """Revision used for this operation."""
    revision: Revision
    """Whether the operation succeeded."""
    success: bool
    """Exit code for the operation."""
    exit_code: int
    """Diagnostics produced by the operation."""
    diagnostics: Sequence[Diagnostic]
    """File images needed to render diagnostics."""
    files: Sequence[FileImage]
    """Messages produced by operation execution."""
    messages: Sequence[Message]
    """Stream output collected during execution."""
    output: Sequence[CommandOutputChunk]
    """Generated output files."""
    outputs: Sequence[CommandOutputFile]
    """Operation payload."""
    data: CleanPayload
    """Count of modules involved."""
    module_count: int
    """Count of profiles involved."""
    profile_count: int
    """Count of targets involved."""
    target_count: int


def encode_clean_output(writer: Writer, value: CleanOutput) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)
    writer.write_unsigned(len(value.output))
    for item_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_0
        )
    destack._generated.protocol.workspace.command.clean.encode_clean_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_clean_output(reader: Reader) -> CleanOutput:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = reader.read_bool()
    field_2 = reader.read_signed_number()
    field_3 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_4 = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = destack._generated.protocol.workspace.command.clean.decode_clean_payload(
        reader
    )
    field_9 = reader.read_number()
    field_10 = reader.read_number()
    field_11 = reader.read_number()

    return CleanOutput(
        revision=field_0,
        success=field_1,
        exit_code=field_2,
        diagnostics=field_3,
        files=field_4,
        messages=field_5,
        output=field_6,
        outputs=field_7,
        data=field_8,
        module_count=field_9,
        profile_count=field_10,
        target_count=field_11,
    )


__all__ = [
    "CheckOutput",
    "encode_check_output",
    "decode_check_output",
    "LintOutput",
    "encode_lint_output",
    "decode_lint_output",
    "FormatOutput",
    "encode_format_output",
    "decode_format_output",
    "BuildOutput",
    "encode_build_output",
    "decode_build_output",
    "RunOutput",
    "encode_run_output",
    "decode_run_output",
    "TestOutput",
    "encode_test_output",
    "decode_test_output",
    "DocOutput",
    "encode_doc_output",
    "decode_doc_output",
    "BenchOutput",
    "encode_bench_output",
    "decode_bench_output",
    "InfoOutput",
    "encode_info_output",
    "decode_info_output",
    "TargetsOutput",
    "encode_targets_output",
    "decode_targets_output",
    "CacheOutput",
    "encode_cache_output",
    "decode_cache_output",
    "SettingsOutput",
    "encode_settings_output",
    "decode_settings_output",
    "DoctorOutput",
    "encode_doctor_output",
    "decode_doctor_output",
    "TaskOutput",
    "encode_task_output",
    "decode_task_output",
    "CleanOutput",
    "encode_clean_output",
    "decode_clean_output",
]
