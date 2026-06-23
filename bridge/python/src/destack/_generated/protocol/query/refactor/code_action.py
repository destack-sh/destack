# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.source.edit.edit

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryRange,
    )

    from destack._generated.protocol.source.edit.edit import (
        PatchSet,
    )


@dataclass(frozen=True, slots=True)
class CodeActionsRequest:
    """Request code actions for a range in a document."""

    """The queried range."""
    range: QueryRange
    """The code action context."""
    context: CodeActionContext


def encode_code_actions_request(writer: Writer, value: CodeActionsRequest) -> None:
    destack._generated.protocol.query.core.target.encode_query_range(
        writer, value.range
    )
    encode_code_action_context(writer, value.context)


def decode_code_actions_request(reader: Reader) -> CodeActionsRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_range(reader)
    field_1 = decode_code_action_context(reader)

    return CodeActionsRequest(
        range=field_0,
        context=field_1,
    )


@dataclass(frozen=True, slots=True)
class CodeActionContext:
    """Context for code action requests."""

    """Requested action kinds (empty = all)."""
    only: Sequence[CodeActionKind]
    """Whether to include disabled actions."""
    include_disabled: bool


def encode_code_action_context(writer: Writer, value: CodeActionContext) -> None:
    writer.write_unsigned(len(value.only))
    for item_0 in value.only:
        encode_code_action_kind(writer, item_0)
    writer.write_bool(value.include_disabled)


def decode_code_action_context(reader: Reader) -> CodeActionContext:
    field_0 = [decode_code_action_kind(reader) for _ in range(reader.read_number())]
    field_1 = reader.read_bool()

    return CodeActionContext(
        only=field_0,
        include_disabled=field_1,
    )


"""Kind of code action."""
CodeActionKind: TypeAlias = (
    Literal["quickFix"]
    | Literal["refactor"]
    | Literal["refactorExtract"]
    | Literal["refactorInline"]
    | Literal["refactorRewrite"]
    | Literal["source"]
    | Literal["sourceFixAll"]
)


def encode_code_action_kind(writer: Writer, value: CodeActionKind) -> None:
    if value == "quickFix":
        writer.write_unsigned(0)
    elif value == "refactor":
        writer.write_unsigned(1)
    elif value == "refactorExtract":
        writer.write_unsigned(2)
    elif value == "refactorInline":
        writer.write_unsigned(3)
    elif value == "refactorRewrite":
        writer.write_unsigned(4)
    elif value == "source":
        writer.write_unsigned(5)
    elif value == "sourceFixAll":
        writer.write_unsigned(6)
    else:
        raise SerdeError("unknown enum variant")


def decode_code_action_kind(reader: Reader) -> CodeActionKind:
    variant = reader.read_number()

    if variant == 0:
        return "quickFix"
    elif variant == 1:
        return "refactor"
    elif variant == 2:
        return "refactorExtract"
    elif variant == 3:
        return "refactorInline"
    elif variant == 4:
        return "refactorRewrite"
    elif variant == 5:
        return "source"
    elif variant == 6:
        return "sourceFixAll"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class CodeActionsResponse:
    """Response payload for code actions queries."""

    """Code actions."""
    actions: Sequence[CodeAction]


def encode_code_actions_response(writer: Writer, value: CodeActionsResponse) -> None:
    writer.write_unsigned(len(value.actions))
    for item_0 in value.actions:
        encode_code_action(writer, item_0)


def decode_code_actions_response(reader: Reader) -> CodeActionsResponse:
    field_0 = [decode_code_action(reader) for _ in range(reader.read_number())]

    return CodeActionsResponse(
        actions=field_0,
    )


@dataclass(frozen=True, slots=True)
class CodeAction:
    """A code action (quick fix or refactoring)."""

    """The title shown in the UI."""
    title: str
    """The kind of action."""
    kind: CodeActionKind
    """Edits to apply."""
    patches: PatchSet
    """Whether this is the preferred action for its diagnostics."""
    is_preferred: bool
    """Whether this action is disabled (with reason)."""
    disabled_reason: str | None
    """The diagnostic code this action fixes (if from a diagnostic)."""
    diagnostic_code: str | None


def encode_code_action(writer: Writer, value: CodeAction) -> None:
    writer.write_string(value.title)
    encode_code_action_kind(writer, value.kind)
    destack._generated.protocol.source.edit.edit.encode_patch_set(writer, value.patches)
    writer.write_bool(value.is_preferred)
    if value.disabled_reason is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.disabled_reason)
    if value.diagnostic_code is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.diagnostic_code)


def decode_code_action(reader: Reader) -> CodeAction:
    field_0 = reader.read_string()
    field_1 = decode_code_action_kind(reader)
    field_2 = destack._generated.protocol.source.edit.edit.decode_patch_set(reader)
    field_3 = reader.read_bool()
    field_4 = reader.read_option(lambda: reader.read_string())
    field_5 = reader.read_option(lambda: reader.read_string())

    return CodeAction(
        title=field_0,
        kind=field_1,
        patches=field_2,
        is_preferred=field_3,
        disabled_reason=field_4,
        diagnostic_code=field_5,
    )


__all__ = [
    "CodeActionsRequest",
    "encode_code_actions_request",
    "decode_code_actions_request",
    "CodeActionContext",
    "encode_code_action_context",
    "decode_code_action_context",
    "CodeActionKind",
    "encode_code_action_kind",
    "decode_code_action_kind",
    "CodeActionsResponse",
    "encode_code_actions_response",
    "decode_code_actions_response",
    "CodeAction",
    "encode_code_action",
    "decode_code_action",
]
