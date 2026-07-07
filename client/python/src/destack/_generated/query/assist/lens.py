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
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class CodeLensesRequest:
    """Request code lenses for a document."""

    # the queried module
    module: destack._generated.query.protocol.target.Module

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_lenses_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeLensesRequest:
        """Decode one CodeLensesRequest."""
        return decode_code_lenses_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_lenses_request(self)

    @classmethod
    def from_json(cls, value: Json) -> CodeLensesRequest:
        """Return one CodeLensesRequest from one JSON value."""
        return from_json_code_lenses_request(value)


def encode_code_lenses_request(writer: BinaryWriter, value: CodeLensesRequest) -> None:
    """Encode one CodeLensesRequest."""
    destack._generated.query.protocol.target.encode_module(writer, value.module)


def decode_code_lenses_request(reader: BinaryReader) -> CodeLensesRequest:
    """Decode one CodeLensesRequest."""
    module = destack._generated.query.protocol.target.decode_module(reader)

    return CodeLensesRequest(
        module=module,
    )


def to_json_code_lenses_request(value: CodeLensesRequest) -> Json:
    """Return one JSON value for one CodeLensesRequest."""
    return {
        "module": destack._generated.query.protocol.target.to_json_module(value.module),
    }


def from_json_code_lenses_request(value: Json) -> CodeLensesRequest:
    """Return one CodeLensesRequest from one JSON value."""
    object_ = json_object(value)

    return CodeLensesRequest(
        module=destack._generated.query.protocol.target.from_json_module(
            json_field(object_, "module")
        ),
    )


@dataclass(frozen=True, slots=True)
class ResolveCodeLensRequest:
    """Request to resolve a code lens."""

    # the code lens to resolve
    lens: CodeLens

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_resolve_code_lens_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ResolveCodeLensRequest:
        """Decode one ResolveCodeLensRequest."""
        return decode_resolve_code_lens_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_resolve_code_lens_request(self)

    @classmethod
    def from_json(cls, value: Json) -> ResolveCodeLensRequest:
        """Return one ResolveCodeLensRequest from one JSON value."""
        return from_json_resolve_code_lens_request(value)


def encode_resolve_code_lens_request(
    writer: BinaryWriter, value: ResolveCodeLensRequest
) -> None:
    """Encode one ResolveCodeLensRequest."""
    encode_code_lens(writer, value.lens)


def decode_resolve_code_lens_request(reader: BinaryReader) -> ResolveCodeLensRequest:
    """Decode one ResolveCodeLensRequest."""
    lens = decode_code_lens(reader)

    return ResolveCodeLensRequest(
        lens=lens,
    )


def to_json_resolve_code_lens_request(value: ResolveCodeLensRequest) -> Json:
    """Return one JSON value for one ResolveCodeLensRequest."""
    return {
        "lens": to_json_code_lens(value.lens),
    }


def from_json_resolve_code_lens_request(value: Json) -> ResolveCodeLensRequest:
    """Return one ResolveCodeLensRequest from one JSON value."""
    object_ = json_object(value)

    return ResolveCodeLensRequest(
        lens=from_json_code_lens(json_field(object_, "lens")),
    )


@dataclass(frozen=True, slots=True)
class CodeLens:
    """A code lens (inline annotation with optional command)."""

    # the range this lens applies to
    range: destack._generated.source.file.model.span.Span
    # the lens action
    action: CodeLensAction

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_lens(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeLens:
        """Decode one CodeLens."""
        return decode_code_lens(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_lens(self)

    @classmethod
    def from_json(cls, value: Json) -> CodeLens:
        """Return one CodeLens from one JSON value."""
        return from_json_code_lens(value)


def encode_code_lens(writer: BinaryWriter, value: CodeLens) -> None:
    """Encode one CodeLens."""
    destack._generated.source.file.model.span.encode_span(writer, value.range)
    encode_code_lens_action(writer, value.action)


def decode_code_lens(reader: BinaryReader) -> CodeLens:
    """Decode one CodeLens."""
    range_ = destack._generated.source.file.model.span.decode_span(reader)
    action = decode_code_lens_action(reader)

    return CodeLens(
        range=range_,
        action=action,
    )


def to_json_code_lens(value: CodeLens) -> Json:
    """Return one JSON value for one CodeLens."""
    return {
        "range": destack._generated.source.file.model.span.to_json_span(value.range),
        "action": to_json_code_lens_action(value.action),
    }


def from_json_code_lens(value: Json) -> CodeLens:
    """Return one CodeLens from one JSON value."""
    object_ = json_object(value)

    return CodeLens(
        range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "range")
        ),
        action=from_json_code_lens_action(json_field(object_, "action")),
    )


@dataclass(frozen=True, slots=True)
class CodeLensActionReferences:
    """Show reference count."""

    # number of references (excluding declaration)
    count: int
    kind: typing.Literal["references"] = "references"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_lens_action(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_lens_action(self)


@dataclass(frozen=True, slots=True)
class CodeLensActionImplementations:
    """Show implementation count."""

    # number of implementations
    count: int
    kind: typing.Literal["implementations"] = "implementations"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_lens_action(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_lens_action(self)


@dataclass(frozen=True, slots=True)
class CodeLensActionRunTest:
    """Run test action."""

    # the test name
    test_name: str
    kind: typing.Literal["runTest"] = "runTest"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_lens_action(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_lens_action(self)


@dataclass(frozen=True, slots=True)
class CodeLensActionDebugTest:
    """Debug test action."""

    # the test name
    test_name: str
    kind: typing.Literal["debugTest"] = "debugTest"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_lens_action(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_lens_action(self)


@dataclass(frozen=True, slots=True)
class CodeLensActionCustom:
    """Custom lens with title and command."""

    # the title to display
    title: str
    # the command identifier
    command: str
    # command arguments (JSON-serializable)
    arguments: Sequence[str]
    kind: typing.Literal["custom"] = "custom"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_lens_action(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_lens_action(self)


"""The action for a code lens."""
CodeLensAction: typing.TypeAlias = (
    CodeLensActionReferences
    | CodeLensActionImplementations
    | CodeLensActionRunTest
    | CodeLensActionDebugTest
    | CodeLensActionCustom
)


def encode_code_lens_action(writer: BinaryWriter, value: CodeLensAction) -> None:
    """Encode one CodeLensAction."""
    if value.kind == "references":
        writer.write_unsigned(0)
        writer.write_unsigned(value.count)
    elif value.kind == "implementations":
        writer.write_unsigned(1)
        writer.write_unsigned(value.count)
    elif value.kind == "runTest":
        writer.write_unsigned(2)
        writer.write_string(value.test_name)
    elif value.kind == "debugTest":
        writer.write_unsigned(3)
        writer.write_string(value.test_name)
    elif value.kind == "custom":
        writer.write_unsigned(4)
        writer.write_string(value.title)
        writer.write_string(value.command)
        writer.write_unsigned(len(value.arguments))
        for item_value_arguments_0 in value.arguments:
            writer.write_string(item_value_arguments_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_code_lens_action(reader: BinaryReader) -> CodeLensAction:
    """Decode one CodeLensAction."""
    variant = reader.read_number()

    if variant == 0:
        count = reader.read_number()

        return CodeLensActionReferences(
            count=count,
        )
    elif variant == 1:
        count = reader.read_number()

        return CodeLensActionImplementations(
            count=count,
        )
    elif variant == 2:
        test_name = reader.read_string()

        return CodeLensActionRunTest(
            test_name=test_name,
        )
    elif variant == 3:
        test_name = reader.read_string()

        return CodeLensActionDebugTest(
            test_name=test_name,
        )
    elif variant == 4:
        title = reader.read_string()
        command = reader.read_string()
        arguments = [reader.read_string() for _ in range(reader.read_number())]

        return CodeLensActionCustom(
            title=title,
            command=command,
            arguments=arguments,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_code_lens_action(value: CodeLensAction) -> Json:
    """Return one JSON value for one CodeLensAction."""
    if value.kind == "references":
        return {
            "kind": "references",
            "count": value.count,
        }
    elif value.kind == "implementations":
        return {
            "kind": "implementations",
            "count": value.count,
        }
    elif value.kind == "runTest":
        return {
            "kind": "runTest",
            "testName": value.test_name,
        }
    elif value.kind == "debugTest":
        return {
            "kind": "debugTest",
            "testName": value.test_name,
        }
    elif value.kind == "custom":
        return {
            "kind": "custom",
            "title": value.title,
            "command": value.command,
            "arguments": [item_0 for item_0 in value.arguments],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_code_lens_action(value: Json) -> CodeLensAction:
    """Return one CodeLensAction from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "references":
        return CodeLensActionReferences(
            count=json_int(json_field(object_, "count")),
        )
    elif kind == "implementations":
        return CodeLensActionImplementations(
            count=json_int(json_field(object_, "count")),
        )
    elif kind == "runTest":
        return CodeLensActionRunTest(
            test_name=json_string(json_field(object_, "testName")),
        )
    elif kind == "debugTest":
        return CodeLensActionDebugTest(
            test_name=json_string(json_field(object_, "testName")),
        )
    elif kind == "custom":
        return CodeLensActionCustom(
            title=json_string(json_field(object_, "title")),
            command=json_string(json_field(object_, "command")),
            arguments=[
                json_string(item_0)
                for item_0 in json_array(json_field(object_, "arguments"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class CodeLensesResponse:
    """Response payload for code lenses queries."""

    # code lenses
    lenses: Sequence[CodeLens]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_lenses_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeLensesResponse:
        """Decode one CodeLensesResponse."""
        return decode_code_lenses_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_lenses_response(self)

    @classmethod
    def from_json(cls, value: Json) -> CodeLensesResponse:
        """Return one CodeLensesResponse from one JSON value."""
        return from_json_code_lenses_response(value)


def encode_code_lenses_response(
    writer: BinaryWriter, value: CodeLensesResponse
) -> None:
    """Encode one CodeLensesResponse."""
    writer.write_unsigned(len(value.lenses))
    for item_value_lenses_0 in value.lenses:
        encode_code_lens(writer, item_value_lenses_0)


def decode_code_lenses_response(reader: BinaryReader) -> CodeLensesResponse:
    """Decode one CodeLensesResponse."""
    lenses = [decode_code_lens(reader) for _ in range(reader.read_number())]

    return CodeLensesResponse(
        lenses=lenses,
    )


def to_json_code_lenses_response(value: CodeLensesResponse) -> Json:
    """Return one JSON value for one CodeLensesResponse."""
    return {
        "lenses": [to_json_code_lens(item_0) for item_0 in value.lenses],
    }


def from_json_code_lenses_response(value: Json) -> CodeLensesResponse:
    """Return one CodeLensesResponse from one JSON value."""
    object_ = json_object(value)

    return CodeLensesResponse(
        lenses=[
            from_json_code_lens(item_0)
            for item_0 in json_array(json_field(object_, "lenses"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ResolveCodeLensResponse:
    """Response payload for code lens resolve queries."""

    # the resolved code lens
    lens: CodeLens

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_resolve_code_lens_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ResolveCodeLensResponse:
        """Decode one ResolveCodeLensResponse."""
        return decode_resolve_code_lens_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_resolve_code_lens_response(self)

    @classmethod
    def from_json(cls, value: Json) -> ResolveCodeLensResponse:
        """Return one ResolveCodeLensResponse from one JSON value."""
        return from_json_resolve_code_lens_response(value)


def encode_resolve_code_lens_response(
    writer: BinaryWriter, value: ResolveCodeLensResponse
) -> None:
    """Encode one ResolveCodeLensResponse."""
    encode_code_lens(writer, value.lens)


def decode_resolve_code_lens_response(reader: BinaryReader) -> ResolveCodeLensResponse:
    """Decode one ResolveCodeLensResponse."""
    lens = decode_code_lens(reader)

    return ResolveCodeLensResponse(
        lens=lens,
    )


def to_json_resolve_code_lens_response(value: ResolveCodeLensResponse) -> Json:
    """Return one JSON value for one ResolveCodeLensResponse."""
    return {
        "lens": to_json_code_lens(value.lens),
    }


def from_json_resolve_code_lens_response(value: Json) -> ResolveCodeLensResponse:
    """Return one ResolveCodeLensResponse from one JSON value."""
    object_ = json_object(value)

    return ResolveCodeLensResponse(
        lens=from_json_code_lens(json_field(object_, "lens")),
    )


__all__ = [
    "CodeLensesRequest",
    "encode_code_lenses_request",
    "decode_code_lenses_request",
    "to_json_code_lenses_request",
    "from_json_code_lenses_request",
    "ResolveCodeLensRequest",
    "encode_resolve_code_lens_request",
    "decode_resolve_code_lens_request",
    "to_json_resolve_code_lens_request",
    "from_json_resolve_code_lens_request",
    "CodeLens",
    "encode_code_lens",
    "decode_code_lens",
    "to_json_code_lens",
    "from_json_code_lens",
    "CodeLensAction",
    "encode_code_lens_action",
    "decode_code_lens_action",
    "to_json_code_lens_action",
    "from_json_code_lens_action",
    "CodeLensActionReferences",
    "CodeLensActionImplementations",
    "CodeLensActionRunTest",
    "CodeLensActionDebugTest",
    "CodeLensActionCustom",
    "CodeLensesResponse",
    "encode_code_lenses_response",
    "decode_code_lenses_response",
    "to_json_code_lenses_response",
    "from_json_code_lenses_response",
    "ResolveCodeLensResponse",
    "encode_resolve_code_lens_response",
    "decode_resolve_code_lens_response",
    "to_json_resolve_code_lens_response",
    "from_json_resolve_code_lens_response",
]
