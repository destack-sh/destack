# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.source.edit.edit

@dataclass(frozen=True, slots=True)
class CodeActionsRequest:
    """Request code actions for a range in a document."""

    # the queried range
    range: destack._generated.query.core.target.QueryRange
    # the code action context
    context: CodeActionContext

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeActionsRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CodeActionsRequest: ...

def encode_code_actions_request(
    writer: BinaryWriter, value: CodeActionsRequest
) -> None: ...
def decode_code_actions_request(reader: BinaryReader) -> CodeActionsRequest: ...
def to_json_code_actions_request(value: CodeActionsRequest) -> Json: ...
def from_json_code_actions_request(value: Json) -> CodeActionsRequest: ...

@dataclass(frozen=True, slots=True)
class CodeActionContext:
    """Context for code action requests."""

    # requested action kinds (empty = all)
    only: Sequence[CodeActionKind]
    # whether to include disabled actions
    include_disabled: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeActionContext: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CodeActionContext: ...

def encode_code_action_context(
    writer: BinaryWriter, value: CodeActionContext
) -> None: ...
def decode_code_action_context(reader: BinaryReader) -> CodeActionContext: ...
def to_json_code_action_context(value: CodeActionContext) -> Json: ...
def from_json_code_action_context(value: Json) -> CodeActionContext: ...

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

def encode_code_action_kind(writer: BinaryWriter, value: CodeActionKind) -> None: ...
def decode_code_action_kind(reader: BinaryReader) -> CodeActionKind: ...
def to_json_code_action_kind(value: CodeActionKind) -> Json: ...
def from_json_code_action_kind(value: Json) -> CodeActionKind: ...

@dataclass(frozen=True, slots=True)
class CodeActionsResponse:
    """Response payload for code actions queries."""

    # code actions
    actions: Sequence[CodeAction]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeActionsResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CodeActionsResponse: ...

def encode_code_actions_response(
    writer: BinaryWriter, value: CodeActionsResponse
) -> None: ...
def decode_code_actions_response(reader: BinaryReader) -> CodeActionsResponse: ...
def to_json_code_actions_response(value: CodeActionsResponse) -> Json: ...
def from_json_code_actions_response(value: Json) -> CodeActionsResponse: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeAction: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CodeAction: ...

def encode_code_action(writer: BinaryWriter, value: CodeAction) -> None: ...
def decode_code_action(reader: BinaryReader) -> CodeAction: ...
def to_json_code_action(value: CodeAction) -> Json: ...
def from_json_code_action(value: Json) -> CodeAction: ...

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
