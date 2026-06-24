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
    json_int,
    json_object,
    json_optional,
)

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
import destack._generated.repository.revision
import destack._generated.source.diagnostic.diagnostic


@dataclass(frozen=True, slots=True)
class CheckOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.check.CheckPayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CheckOutput:
        """Decode one CheckOutput."""
        return decode_check_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_output(self)

    @classmethod
    def from_json(cls, value: Json) -> CheckOutput:
        """Return one CheckOutput from one JSON value."""
        return from_json_check_output(value)


def encode_check_output(writer: BinaryWriter, value: CheckOutput) -> None:
    """Encode one CheckOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.check.encode_check_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_check_output(reader: BinaryReader) -> CheckOutput:
    """Decode one CheckOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.check.decode_check_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return CheckOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_check_output(value: CheckOutput) -> Json:
    """Return one JSON value for one CheckOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.check.to_json_check_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_check_output(value: Json) -> CheckOutput:
    """Return one CheckOutput from one JSON value."""
    object_ = json_object(value)

    return CheckOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.check.from_json_check_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class LintOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.check.LintPayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lint_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LintOutput:
        """Decode one LintOutput."""
        return decode_lint_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lint_output(self)

    @classmethod
    def from_json(cls, value: Json) -> LintOutput:
        """Return one LintOutput from one JSON value."""
        return from_json_lint_output(value)


def encode_lint_output(writer: BinaryWriter, value: LintOutput) -> None:
    """Encode one LintOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.check.encode_lint_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_lint_output(reader: BinaryReader) -> LintOutput:
    """Decode one LintOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.check.decode_lint_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return LintOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_lint_output(value: LintOutput) -> Json:
    """Return one JSON value for one LintOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.check.to_json_lint_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_lint_output(value: Json) -> LintOutput:
    """Return one LintOutput from one JSON value."""
    object_ = json_object(value)

    return LintOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.check.from_json_lint_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class FormatOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.format.FormatPayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_format_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FormatOutput:
        """Decode one FormatOutput."""
        return decode_format_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_format_output(self)

    @classmethod
    def from_json(cls, value: Json) -> FormatOutput:
        """Return one FormatOutput from one JSON value."""
        return from_json_format_output(value)


def encode_format_output(writer: BinaryWriter, value: FormatOutput) -> None:
    """Encode one FormatOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.format.encode_format_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_format_output(reader: BinaryReader) -> FormatOutput:
    """Decode one FormatOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.format.decode_format_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return FormatOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_format_output(value: FormatOutput) -> Json:
    """Return one JSON value for one FormatOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.format.to_json_format_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_format_output(value: Json) -> FormatOutput:
    """Return one FormatOutput from one JSON value."""
    object_ = json_object(value)

    return FormatOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.format.from_json_format_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class BuildOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.build.BuildPayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_build_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BuildOutput:
        """Decode one BuildOutput."""
        return decode_build_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_build_output(self)

    @classmethod
    def from_json(cls, value: Json) -> BuildOutput:
        """Return one BuildOutput from one JSON value."""
        return from_json_build_output(value)


def encode_build_output(writer: BinaryWriter, value: BuildOutput) -> None:
    """Encode one BuildOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.build.encode_build_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_build_output(reader: BinaryReader) -> BuildOutput:
    """Decode one BuildOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.build.decode_build_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return BuildOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_build_output(value: BuildOutput) -> Json:
    """Return one JSON value for one BuildOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.build.to_json_build_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_build_output(value: Json) -> BuildOutput:
    """Return one BuildOutput from one JSON value."""
    object_ = json_object(value)

    return BuildOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.build.from_json_build_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class RunOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.run.RunPayload | None
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_run_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RunOutput:
        """Decode one RunOutput."""
        return decode_run_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_run_output(self)

    @classmethod
    def from_json(cls, value: Json) -> RunOutput:
        """Return one RunOutput from one JSON value."""
        return from_json_run_output(value)


def encode_run_output(writer: BinaryWriter, value: RunOutput) -> None:
    """Encode one RunOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
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


def decode_run_output(reader: BinaryReader) -> RunOutput:
    """Decode one RunOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = reader.read_option(
        lambda: destack._generated.protocol.workspace.command.run.decode_run_payload(
            reader
        )
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return RunOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_run_output(value: RunOutput) -> Json:
    """Return one JSON value for one RunOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        **(
            {}
            if value.data is None
            else {
                "data": destack._generated.protocol.workspace.command.run.to_json_run_payload(
                    value.data
                )
            }
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_run_output(value: Json) -> RunOutput:
    """Return one RunOutput from one JSON value."""
    object_ = json_object(value)

    return RunOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=json_optional(
            object_,
            "data",
            lambda value: (
                destack._generated.protocol.workspace.command.run.from_json_run_payload(
                    value
                )
            ),
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class TestOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.common.CommandMessagePayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_test_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TestOutput:
        """Decode one TestOutput."""
        return decode_test_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_test_output(self)

    @classmethod
    def from_json(cls, value: Json) -> TestOutput:
        """Return one TestOutput from one JSON value."""
        return from_json_test_output(value)


def encode_test_output(writer: BinaryWriter, value: TestOutput) -> None:
    """Encode one TestOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.common.encode_command_message_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_test_output(reader: BinaryReader) -> TestOutput:
    """Decode one TestOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.common.decode_command_message_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return TestOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_test_output(value: TestOutput) -> Json:
    """Return one JSON value for one TestOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.common.to_json_command_message_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_test_output(value: Json) -> TestOutput:
    """Return one TestOutput from one JSON value."""
    object_ = json_object(value)

    return TestOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.common.from_json_command_message_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class DocOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.common.CommandMessagePayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_doc_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DocOutput:
        """Decode one DocOutput."""
        return decode_doc_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_doc_output(self)

    @classmethod
    def from_json(cls, value: Json) -> DocOutput:
        """Return one DocOutput from one JSON value."""
        return from_json_doc_output(value)


def encode_doc_output(writer: BinaryWriter, value: DocOutput) -> None:
    """Encode one DocOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.common.encode_command_message_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_doc_output(reader: BinaryReader) -> DocOutput:
    """Decode one DocOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.common.decode_command_message_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return DocOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_doc_output(value: DocOutput) -> Json:
    """Return one JSON value for one DocOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.common.to_json_command_message_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_doc_output(value: Json) -> DocOutput:
    """Return one DocOutput from one JSON value."""
    object_ = json_object(value)

    return DocOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.common.from_json_command_message_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class BenchOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.common.CommandMessagePayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_bench_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BenchOutput:
        """Decode one BenchOutput."""
        return decode_bench_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_bench_output(self)

    @classmethod
    def from_json(cls, value: Json) -> BenchOutput:
        """Return one BenchOutput from one JSON value."""
        return from_json_bench_output(value)


def encode_bench_output(writer: BinaryWriter, value: BenchOutput) -> None:
    """Encode one BenchOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.common.encode_command_message_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_bench_output(reader: BinaryReader) -> BenchOutput:
    """Decode one BenchOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.common.decode_command_message_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return BenchOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_bench_output(value: BenchOutput) -> Json:
    """Return one JSON value for one BenchOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.common.to_json_command_message_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_bench_output(value: Json) -> BenchOutput:
    """Return one BenchOutput from one JSON value."""
    object_ = json_object(value)

    return BenchOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.common.from_json_command_message_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class InfoOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.info.InfoPayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_info_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InfoOutput:
        """Decode one InfoOutput."""
        return decode_info_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_info_output(self)

    @classmethod
    def from_json(cls, value: Json) -> InfoOutput:
        """Return one InfoOutput from one JSON value."""
        return from_json_info_output(value)


def encode_info_output(writer: BinaryWriter, value: InfoOutput) -> None:
    """Encode one InfoOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.info.encode_info_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_info_output(reader: BinaryReader) -> InfoOutput:
    """Decode one InfoOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.info.decode_info_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return InfoOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_info_output(value: InfoOutput) -> Json:
    """Return one JSON value for one InfoOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.info.to_json_info_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_info_output(value: Json) -> InfoOutput:
    """Return one InfoOutput from one JSON value."""
    object_ = json_object(value)

    return InfoOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.info.from_json_info_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class TargetsOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.targets.TargetsPayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_targets_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetsOutput:
        """Decode one TargetsOutput."""
        return decode_targets_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_targets_output(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetsOutput:
        """Return one TargetsOutput from one JSON value."""
        return from_json_targets_output(value)


def encode_targets_output(writer: BinaryWriter, value: TargetsOutput) -> None:
    """Encode one TargetsOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.targets.encode_targets_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_targets_output(reader: BinaryReader) -> TargetsOutput:
    """Decode one TargetsOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.targets.decode_targets_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return TargetsOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_targets_output(value: TargetsOutput) -> Json:
    """Return one JSON value for one TargetsOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.targets.to_json_targets_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_targets_output(value: Json) -> TargetsOutput:
    """Return one TargetsOutput from one JSON value."""
    object_ = json_object(value)

    return TargetsOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.targets.from_json_targets_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class CacheOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.cache.CachePayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cache_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CacheOutput:
        """Decode one CacheOutput."""
        return decode_cache_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cache_output(self)

    @classmethod
    def from_json(cls, value: Json) -> CacheOutput:
        """Return one CacheOutput from one JSON value."""
        return from_json_cache_output(value)


def encode_cache_output(writer: BinaryWriter, value: CacheOutput) -> None:
    """Encode one CacheOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.cache.encode_cache_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_cache_output(reader: BinaryReader) -> CacheOutput:
    """Decode one CacheOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.cache.decode_cache_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return CacheOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_cache_output(value: CacheOutput) -> Json:
    """Return one JSON value for one CacheOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.cache.to_json_cache_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_cache_output(value: Json) -> CacheOutput:
    """Return one CacheOutput from one JSON value."""
    object_ = json_object(value)

    return CacheOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.cache.from_json_cache_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class SettingsOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.settings.SettingsPayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_settings_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SettingsOutput:
        """Decode one SettingsOutput."""
        return decode_settings_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_settings_output(self)

    @classmethod
    def from_json(cls, value: Json) -> SettingsOutput:
        """Return one SettingsOutput from one JSON value."""
        return from_json_settings_output(value)


def encode_settings_output(writer: BinaryWriter, value: SettingsOutput) -> None:
    """Encode one SettingsOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.settings.encode_settings_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_settings_output(reader: BinaryReader) -> SettingsOutput:
    """Decode one SettingsOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = (
        destack._generated.protocol.workspace.command.settings.decode_settings_payload(
            reader
        )
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return SettingsOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_settings_output(value: SettingsOutput) -> Json:
    """Return one JSON value for one SettingsOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.settings.to_json_settings_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_settings_output(value: Json) -> SettingsOutput:
    """Return one SettingsOutput from one JSON value."""
    object_ = json_object(value)

    return SettingsOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.settings.from_json_settings_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class DoctorOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.doctor.DoctorPayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_doctor_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DoctorOutput:
        """Decode one DoctorOutput."""
        return decode_doctor_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_doctor_output(self)

    @classmethod
    def from_json(cls, value: Json) -> DoctorOutput:
        """Return one DoctorOutput from one JSON value."""
        return from_json_doctor_output(value)


def encode_doctor_output(writer: BinaryWriter, value: DoctorOutput) -> None:
    """Encode one DoctorOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.doctor.encode_doctor_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_doctor_output(reader: BinaryReader) -> DoctorOutput:
    """Decode one DoctorOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.doctor.decode_doctor_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return DoctorOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_doctor_output(value: DoctorOutput) -> Json:
    """Return one JSON value for one DoctorOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.doctor.to_json_doctor_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_doctor_output(value: Json) -> DoctorOutput:
    """Return one DoctorOutput from one JSON value."""
    object_ = json_object(value)

    return DoctorOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.doctor.from_json_doctor_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class TaskOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.task.TaskPayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_task_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TaskOutput:
        """Decode one TaskOutput."""
        return decode_task_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_task_output(self)

    @classmethod
    def from_json(cls, value: Json) -> TaskOutput:
        """Return one TaskOutput from one JSON value."""
        return from_json_task_output(value)


def encode_task_output(writer: BinaryWriter, value: TaskOutput) -> None:
    """Encode one TaskOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.task.encode_task_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_task_output(reader: BinaryReader) -> TaskOutput:
    """Decode one TaskOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.task.decode_task_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return TaskOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_task_output(value: TaskOutput) -> Json:
    """Return one JSON value for one TaskOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.task.to_json_task_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_task_output(value: Json) -> TaskOutput:
    """Return one TaskOutput from one JSON value."""
    object_ = json_object(value)

    return TaskOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.task.from_json_task_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


@dataclass(frozen=True, slots=True)
class CleanOutput:
    """Output produced by one workspace command."""

    # revision used for this operation
    revision: destack._generated.repository.revision.Revision
    # whether the operation succeeded
    success: bool
    # exit code for the operation
    exit_code: int
    # diagnostics produced by the operation
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]
    # file images needed to render diagnostics
    files: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    # messages produced by operation execution
    messages: Sequence[destack._generated.protocol.workspace.message.Message]
    # stream output collected during execution
    output: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputChunk
    ]
    # generated output files
    outputs: Sequence[
        destack._generated.protocol.workspace.command.common.CommandOutputFile
    ]
    # operation payload
    data: destack._generated.protocol.workspace.command.clean.CleanPayload
    # count of modules involved
    module_count: int
    # count of profiles involved
    profile_count: int
    # count of targets involved
    target_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_clean_output(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CleanOutput:
        """Decode one CleanOutput."""
        return decode_clean_output(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_clean_output(self)

    @classmethod
    def from_json(cls, value: Json) -> CleanOutput:
        """Return one CleanOutput from one JSON value."""
        return from_json_clean_output(value)


def encode_clean_output(writer: BinaryWriter, value: CleanOutput) -> None:
    """Encode one CleanOutput."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_bool(value.success)
    writer.write_signed(value.exit_code)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        destack._generated.protocol.workspace.file.image.encode_file_image(
            writer, item_value_files_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )
    writer.write_unsigned(len(value.output))
    for item_value_output_0 in value.output:
        destack._generated.protocol.workspace.command.common.encode_command_output_chunk(
            writer, item_value_output_0
        )
    writer.write_unsigned(len(value.outputs))
    for item_value_outputs_0 in value.outputs:
        destack._generated.protocol.workspace.command.common.encode_command_output_file(
            writer, item_value_outputs_0
        )
    destack._generated.protocol.workspace.command.clean.encode_clean_payload(
        writer, value.data
    )
    writer.write_unsigned(value.module_count)
    writer.write_unsigned(value.profile_count)
    writer.write_unsigned(value.target_count)


def decode_clean_output(reader: BinaryReader) -> CleanOutput:
    """Decode one CleanOutput."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    success = reader.read_bool()
    exit_code = reader.read_signed_number()
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    files = [
        destack._generated.protocol.workspace.file.image.decode_file_image(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]
    output = [
        destack._generated.protocol.workspace.command.common.decode_command_output_chunk(
            reader
        )
        for _ in range(reader.read_number())
    ]
    outputs = [
        destack._generated.protocol.workspace.command.common.decode_command_output_file(
            reader
        )
        for _ in range(reader.read_number())
    ]
    data = destack._generated.protocol.workspace.command.clean.decode_clean_payload(
        reader
    )
    module_count = reader.read_number()
    profile_count = reader.read_number()
    target_count = reader.read_number()

    return CleanOutput(
        revision=revision,
        success=success,
        exit_code=exit_code,
        diagnostics=diagnostics,
        files=files,
        messages=messages,
        output=output,
        outputs=outputs,
        data=data,
        module_count=module_count,
        profile_count=profile_count,
        target_count=target_count,
    )


def to_json_clean_output(value: CleanOutput) -> Json:
    """Return one JSON value for one CleanOutput."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "success": value.success,
        "exitCode": value.exit_code,
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
        "files": [
            destack._generated.protocol.workspace.file.image.to_json_file_image(item_0)
            for item_0 in value.files
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
        "output": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_chunk(
                item_0
            )
            for item_0 in value.output
        ],
        "outputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_output_file(
                item_0
            )
            for item_0 in value.outputs
        ],
        "data": destack._generated.protocol.workspace.command.clean.to_json_clean_payload(
            value.data
        ),
        "moduleCount": value.module_count,
        "profileCount": value.profile_count,
        "targetCount": value.target_count,
    }


def from_json_clean_output(value: Json) -> CleanOutput:
    """Return one CleanOutput from one JSON value."""
    object_ = json_object(value)

    return CleanOutput(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        success=json_bool(json_field(object_, "success")),
        exit_code=json_int(json_field(object_, "exitCode")),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
        files=[
            destack._generated.protocol.workspace.file.image.from_json_file_image(
                item_0
            )
            for item_0 in json_array(json_field(object_, "files"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
        output=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_chunk(
                item_0
            )
            for item_0 in json_array(json_field(object_, "output"))
        ],
        outputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_output_file(
                item_0
            )
            for item_0 in json_array(json_field(object_, "outputs"))
        ],
        data=destack._generated.protocol.workspace.command.clean.from_json_clean_payload(
            json_field(object_, "data")
        ),
        module_count=json_int(json_field(object_, "moduleCount")),
        profile_count=json_int(json_field(object_, "profileCount")),
        target_count=json_int(json_field(object_, "targetCount")),
    )


__all__ = [
    "CheckOutput",
    "encode_check_output",
    "decode_check_output",
    "to_json_check_output",
    "from_json_check_output",
    "LintOutput",
    "encode_lint_output",
    "decode_lint_output",
    "to_json_lint_output",
    "from_json_lint_output",
    "FormatOutput",
    "encode_format_output",
    "decode_format_output",
    "to_json_format_output",
    "from_json_format_output",
    "BuildOutput",
    "encode_build_output",
    "decode_build_output",
    "to_json_build_output",
    "from_json_build_output",
    "RunOutput",
    "encode_run_output",
    "decode_run_output",
    "to_json_run_output",
    "from_json_run_output",
    "TestOutput",
    "encode_test_output",
    "decode_test_output",
    "to_json_test_output",
    "from_json_test_output",
    "DocOutput",
    "encode_doc_output",
    "decode_doc_output",
    "to_json_doc_output",
    "from_json_doc_output",
    "BenchOutput",
    "encode_bench_output",
    "decode_bench_output",
    "to_json_bench_output",
    "from_json_bench_output",
    "InfoOutput",
    "encode_info_output",
    "decode_info_output",
    "to_json_info_output",
    "from_json_info_output",
    "TargetsOutput",
    "encode_targets_output",
    "decode_targets_output",
    "to_json_targets_output",
    "from_json_targets_output",
    "CacheOutput",
    "encode_cache_output",
    "decode_cache_output",
    "to_json_cache_output",
    "from_json_cache_output",
    "SettingsOutput",
    "encode_settings_output",
    "decode_settings_output",
    "to_json_settings_output",
    "from_json_settings_output",
    "DoctorOutput",
    "encode_doctor_output",
    "decode_doctor_output",
    "to_json_doctor_output",
    "from_json_doctor_output",
    "TaskOutput",
    "encode_task_output",
    "decode_task_output",
    "to_json_task_output",
    "from_json_task_output",
    "CleanOutput",
    "encode_clean_output",
    "decode_clean_output",
    "to_json_clean_output",
    "from_json_clean_output",
]
