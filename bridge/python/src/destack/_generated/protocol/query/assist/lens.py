# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.source.file.model.span

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryModule,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class CodeLensesRequest:
    """Request code lenses for a document."""

    """The queried module."""
    module: QueryModule


def encode_code_lenses_request(writer: Writer, value: CodeLensesRequest) -> None:
    destack._generated.protocol.query.core.target.encode_query_module(
        writer, value.module
    )


def decode_code_lenses_request(reader: Reader) -> CodeLensesRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_module(reader)

    return CodeLensesRequest(
        module=field_0,
    )


@dataclass(frozen=True, slots=True)
class ResolveCodeLensRequest:
    """Request to resolve a code lens."""

    """The code lens to resolve."""
    lens: CodeLens


def encode_resolve_code_lens_request(
    writer: Writer, value: ResolveCodeLensRequest
) -> None:
    encode_code_lens(writer, value.lens)


def decode_resolve_code_lens_request(reader: Reader) -> ResolveCodeLensRequest:
    field_0 = decode_code_lens(reader)

    return ResolveCodeLensRequest(
        lens=field_0,
    )


@dataclass(frozen=True, slots=True)
class CodeLens:
    """A code lens (inline annotation with optional command)."""

    """The range this lens applies to."""
    range: Span
    """The lens action."""
    action: CodeLensAction


def encode_code_lens(writer: Writer, value: CodeLens) -> None:
    destack._generated.protocol.source.file.model.span.encode_span(writer, value.range)
    encode_code_lens_action(writer, value.action)


def decode_code_lens(reader: Reader) -> CodeLens:
    field_0 = destack._generated.protocol.source.file.model.span.decode_span(reader)
    field_1 = decode_code_lens_action(reader)

    return CodeLens(
        range=field_0,
        action=field_1,
    )


@dataclass(frozen=True, slots=True)
class CodeLensActionReferences:
    """Show reference count."""

    """Number of references (excluding declaration)."""
    count: int
    kind: Literal["references"] = "references"


@dataclass(frozen=True, slots=True)
class CodeLensActionImplementations:
    """Show implementation count."""

    """Number of implementations."""
    count: int
    kind: Literal["implementations"] = "implementations"


@dataclass(frozen=True, slots=True)
class CodeLensActionRunTest:
    """Run test action."""

    """The test name."""
    test_name: str
    kind: Literal["runTest"] = "runTest"


@dataclass(frozen=True, slots=True)
class CodeLensActionDebugTest:
    """Debug test action."""

    """The test name."""
    test_name: str
    kind: Literal["debugTest"] = "debugTest"


@dataclass(frozen=True, slots=True)
class CodeLensActionCustom:
    """Custom lens with title and command."""

    """The title to display."""
    title: str
    """The command identifier."""
    command: str
    """Command arguments (JSON-serializable)."""
    arguments: Sequence[str]
    kind: Literal["custom"] = "custom"


"""The action for a code lens."""
CodeLensAction: TypeAlias = (
    CodeLensActionReferences
    | CodeLensActionImplementations
    | CodeLensActionRunTest
    | CodeLensActionDebugTest
    | CodeLensActionCustom
)


def encode_code_lens_action(writer: Writer, value: CodeLensAction) -> None:
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
        for item_0 in value.arguments:
            writer.write_string(item_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_code_lens_action(reader: Reader) -> CodeLensAction:
    variant = reader.read_number()

    if variant == 0:
        field_0 = reader.read_number()

        return CodeLensActionReferences(
            count=field_0,
        )
    elif variant == 1:
        field_0 = reader.read_number()

        return CodeLensActionImplementations(
            count=field_0,
        )
    elif variant == 2:
        field_0 = reader.read_string()

        return CodeLensActionRunTest(
            test_name=field_0,
        )
    elif variant == 3:
        field_0 = reader.read_string()

        return CodeLensActionDebugTest(
            test_name=field_0,
        )
    elif variant == 4:
        field_0 = reader.read_string()
        field_1 = reader.read_string()
        field_2 = [reader.read_string() for _ in range(reader.read_number())]

        return CodeLensActionCustom(
            title=field_0,
            command=field_1,
            arguments=field_2,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class CodeLensesResponse:
    """Response payload for code lenses queries."""

    """Code lenses."""
    lenses: Sequence[CodeLens]


def encode_code_lenses_response(writer: Writer, value: CodeLensesResponse) -> None:
    writer.write_unsigned(len(value.lenses))
    for item_0 in value.lenses:
        encode_code_lens(writer, item_0)


def decode_code_lenses_response(reader: Reader) -> CodeLensesResponse:
    field_0 = [decode_code_lens(reader) for _ in range(reader.read_number())]

    return CodeLensesResponse(
        lenses=field_0,
    )


@dataclass(frozen=True, slots=True)
class ResolveCodeLensResponse:
    """Response payload for code lens resolve queries."""

    """The resolved code lens."""
    lens: CodeLens


def encode_resolve_code_lens_response(
    writer: Writer, value: ResolveCodeLensResponse
) -> None:
    encode_code_lens(writer, value.lens)


def decode_resolve_code_lens_response(reader: Reader) -> ResolveCodeLensResponse:
    field_0 = decode_code_lens(reader)

    return ResolveCodeLensResponse(
        lens=field_0,
    )


__all__ = [
    "CodeLensesRequest",
    "encode_code_lenses_request",
    "decode_code_lenses_request",
    "ResolveCodeLensRequest",
    "encode_resolve_code_lens_request",
    "decode_resolve_code_lens_request",
    "CodeLens",
    "encode_code_lens",
    "decode_code_lens",
    "CodeLensAction",
    "encode_code_lens_action",
    "decode_code_lens_action",
    "CodeLensActionReferences",
    "CodeLensActionImplementations",
    "CodeLensActionRunTest",
    "CodeLensActionDebugTest",
    "CodeLensActionCustom",
    "CodeLensesResponse",
    "encode_code_lenses_response",
    "decode_code_lenses_response",
    "ResolveCodeLensResponse",
    "encode_resolve_code_lens_response",
    "decode_resolve_code_lens_response",
]
