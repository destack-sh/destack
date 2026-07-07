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
    json_object,
    json_optional,
    json_string,
)

import destack._generated.query.protocol.target
import destack._generated.source.edit.edit


@dataclass(frozen=True, slots=True)
class CodeActionsRequest:
    """Request code actions for a range in a document."""

    # the queried range
    range: destack._generated.query.protocol.target.Range
    # the code action context
    context: CodeActionContext

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_actions_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeActionsRequest:
        """Decode one CodeActionsRequest."""
        return decode_code_actions_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_actions_request(self)

    @classmethod
    def from_json(cls, value: Json) -> CodeActionsRequest:
        """Return one CodeActionsRequest from one JSON value."""
        return from_json_code_actions_request(value)


def encode_code_actions_request(
    writer: BinaryWriter, value: CodeActionsRequest
) -> None:
    """Encode one CodeActionsRequest."""
    destack._generated.query.protocol.target.encode_range(writer, value.range)
    encode_code_action_context(writer, value.context)


def decode_code_actions_request(reader: BinaryReader) -> CodeActionsRequest:
    """Decode one CodeActionsRequest."""
    range_ = destack._generated.query.protocol.target.decode_range(reader)
    context = decode_code_action_context(reader)

    return CodeActionsRequest(
        range=range_,
        context=context,
    )


def to_json_code_actions_request(value: CodeActionsRequest) -> Json:
    """Return one JSON value for one CodeActionsRequest."""
    return {
        "range": destack._generated.query.protocol.target.to_json_range(value.range),
        "context": to_json_code_action_context(value.context),
    }


def from_json_code_actions_request(value: Json) -> CodeActionsRequest:
    """Return one CodeActionsRequest from one JSON value."""
    object_ = json_object(value)

    return CodeActionsRequest(
        range=destack._generated.query.protocol.target.from_json_range(
            json_field(object_, "range")
        ),
        context=from_json_code_action_context(json_field(object_, "context")),
    )


@dataclass(frozen=True, slots=True)
class CodeActionContext:
    """Context for code action requests."""

    # requested action kinds (empty = all)
    only: Sequence[CodeActionKind]
    # whether to include disabled actions
    include_disabled: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_action_context(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeActionContext:
        """Decode one CodeActionContext."""
        return decode_code_action_context(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_action_context(self)

    @classmethod
    def from_json(cls, value: Json) -> CodeActionContext:
        """Return one CodeActionContext from one JSON value."""
        return from_json_code_action_context(value)


def encode_code_action_context(writer: BinaryWriter, value: CodeActionContext) -> None:
    """Encode one CodeActionContext."""
    writer.write_unsigned(len(value.only))
    for item_value_only_0 in value.only:
        encode_code_action_kind(writer, item_value_only_0)
    writer.write_bool(value.include_disabled)


def decode_code_action_context(reader: BinaryReader) -> CodeActionContext:
    """Decode one CodeActionContext."""
    only = [decode_code_action_kind(reader) for _ in range(reader.read_number())]
    include_disabled = reader.read_bool()

    return CodeActionContext(
        only=only,
        include_disabled=include_disabled,
    )


def to_json_code_action_context(value: CodeActionContext) -> Json:
    """Return one JSON value for one CodeActionContext."""
    return {
        "only": [to_json_code_action_kind(item_0) for item_0 in value.only],
        "includeDisabled": value.include_disabled,
    }


def from_json_code_action_context(value: Json) -> CodeActionContext:
    """Return one CodeActionContext from one JSON value."""
    object_ = json_object(value)

    return CodeActionContext(
        only=[
            from_json_code_action_kind(item_0)
            for item_0 in json_array(json_field(object_, "only"))
        ],
        include_disabled=json_bool(json_field(object_, "includeDisabled")),
    )


"""Kind of code action."""
CodeActionKind: typing.TypeAlias = (
    typing.Literal["quickFix"]
    | typing.Literal["refactor"]
    | typing.Literal["refactorExtract"]
    | typing.Literal["refactorInline"]
    | typing.Literal["refactorRewrite"]
    | typing.Literal["source"]
    | typing.Literal["sourceFixAll"]
)


def encode_code_action_kind(writer: BinaryWriter, value: CodeActionKind) -> None:
    """Encode one CodeActionKind."""
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


def decode_code_action_kind(reader: BinaryReader) -> CodeActionKind:
    """Decode one CodeActionKind."""
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


def to_json_code_action_kind(value: CodeActionKind) -> Json:
    """Return one JSON value for one CodeActionKind."""
    return value


def from_json_code_action_kind(value: Json) -> CodeActionKind:
    """Return one CodeActionKind from one JSON value."""
    variant = json_string(value)

    if variant == "quickFix":
        return "quickFix"
    elif variant == "refactor":
        return "refactor"
    elif variant == "refactorExtract":
        return "refactorExtract"
    elif variant == "refactorInline":
        return "refactorInline"
    elif variant == "refactorRewrite":
        return "refactorRewrite"
    elif variant == "source":
        return "source"
    elif variant == "sourceFixAll":
        return "sourceFixAll"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class CodeActionsResponse:
    """Response payload for code actions queries."""

    # code actions
    actions: Sequence[CodeAction]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_actions_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeActionsResponse:
        """Decode one CodeActionsResponse."""
        return decode_code_actions_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_actions_response(self)

    @classmethod
    def from_json(cls, value: Json) -> CodeActionsResponse:
        """Return one CodeActionsResponse from one JSON value."""
        return from_json_code_actions_response(value)


def encode_code_actions_response(
    writer: BinaryWriter, value: CodeActionsResponse
) -> None:
    """Encode one CodeActionsResponse."""
    writer.write_unsigned(len(value.actions))
    for item_value_actions_0 in value.actions:
        encode_code_action(writer, item_value_actions_0)


def decode_code_actions_response(reader: BinaryReader) -> CodeActionsResponse:
    """Decode one CodeActionsResponse."""
    actions = [decode_code_action(reader) for _ in range(reader.read_number())]

    return CodeActionsResponse(
        actions=actions,
    )


def to_json_code_actions_response(value: CodeActionsResponse) -> Json:
    """Return one JSON value for one CodeActionsResponse."""
    return {
        "actions": [to_json_code_action(item_0) for item_0 in value.actions],
    }


def from_json_code_actions_response(value: Json) -> CodeActionsResponse:
    """Return one CodeActionsResponse from one JSON value."""
    object_ = json_object(value)

    return CodeActionsResponse(
        actions=[
            from_json_code_action(item_0)
            for item_0 in json_array(json_field(object_, "actions"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CodeAction:
    """A code action (quick fix or refactoring)."""

    # the title shown in the UI
    title: str
    # the kind of action
    kind: CodeActionKind
    # edits to apply
    patches: destack._generated.source.edit.edit.PatchSet
    # whether this is the preferred action for its diagnostics
    is_preferred: bool
    # whether this action is disabled (with reason)
    disabled_reason: str | None
    # the diagnostic code this action fixes (if from a diagnostic)
    diagnostic_code: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_action(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeAction:
        """Decode one CodeAction."""
        return decode_code_action(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_action(self)

    @classmethod
    def from_json(cls, value: Json) -> CodeAction:
        """Return one CodeAction from one JSON value."""
        return from_json_code_action(value)


def encode_code_action(writer: BinaryWriter, value: CodeAction) -> None:
    """Encode one CodeAction."""
    writer.write_string(value.title)
    encode_code_action_kind(writer, value.kind)
    destack._generated.source.edit.edit.encode_patch_set(writer, value.patches)
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


def decode_code_action(reader: BinaryReader) -> CodeAction:
    """Decode one CodeAction."""
    title = reader.read_string()
    kind = decode_code_action_kind(reader)
    patches = destack._generated.source.edit.edit.decode_patch_set(reader)
    is_preferred = reader.read_bool()
    disabled_reason = reader.read_option(lambda: reader.read_string())
    diagnostic_code = reader.read_option(lambda: reader.read_string())

    return CodeAction(
        title=title,
        kind=kind,
        patches=patches,
        is_preferred=is_preferred,
        disabled_reason=disabled_reason,
        diagnostic_code=diagnostic_code,
    )


def to_json_code_action(value: CodeAction) -> Json:
    """Return one JSON value for one CodeAction."""
    return {
        "title": value.title,
        "kind": to_json_code_action_kind(value.kind),
        "patches": destack._generated.source.edit.edit.to_json_patch_set(value.patches),
        "isPreferred": value.is_preferred,
        **(
            {}
            if value.disabled_reason is None
            else {"disabledReason": value.disabled_reason}
        ),
        **(
            {}
            if value.diagnostic_code is None
            else {"diagnosticCode": value.diagnostic_code}
        ),
    }


def from_json_code_action(value: Json) -> CodeAction:
    """Return one CodeAction from one JSON value."""
    object_ = json_object(value)

    return CodeAction(
        title=json_string(json_field(object_, "title")),
        kind=from_json_code_action_kind(json_field(object_, "kind")),
        patches=destack._generated.source.edit.edit.from_json_patch_set(
            json_field(object_, "patches")
        ),
        is_preferred=json_bool(json_field(object_, "isPreferred")),
        disabled_reason=json_optional(
            object_, "disabledReason", lambda value: json_string(value)
        ),
        diagnostic_code=json_optional(
            object_, "diagnosticCode", lambda value: json_string(value)
        ),
    )


__all__ = [
    "CodeActionsRequest",
    "encode_code_actions_request",
    "decode_code_actions_request",
    "to_json_code_actions_request",
    "from_json_code_actions_request",
    "CodeActionContext",
    "encode_code_action_context",
    "decode_code_action_context",
    "to_json_code_action_context",
    "from_json_code_action_context",
    "CodeActionKind",
    "encode_code_action_kind",
    "decode_code_action_kind",
    "to_json_code_action_kind",
    "from_json_code_action_kind",
    "CodeActionsResponse",
    "encode_code_actions_response",
    "decode_code_actions_response",
    "to_json_code_actions_response",
    "from_json_code_actions_response",
    "CodeAction",
    "encode_code_action",
    "decode_code_action",
    "to_json_code_action",
    "from_json_code_action",
]
