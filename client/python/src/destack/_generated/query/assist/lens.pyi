# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class CodeLensesRequest:
    """Request code lenses for a document."""

    # the queried module
    module: destack._generated.query.protocol.target.Module

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeLensesRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CodeLensesRequest: ...

def encode_code_lenses_request(
    writer: BinaryWriter, value: CodeLensesRequest
) -> None: ...
def decode_code_lenses_request(reader: BinaryReader) -> CodeLensesRequest: ...
def to_json_code_lenses_request(value: CodeLensesRequest) -> Json: ...
def from_json_code_lenses_request(value: Json) -> CodeLensesRequest: ...

@dataclass(frozen=True, slots=True)
class ResolveCodeLensRequest:
    """Request to resolve a code lens."""

    # the code lens to resolve
    lens: CodeLens

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ResolveCodeLensRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ResolveCodeLensRequest: ...

def encode_resolve_code_lens_request(
    writer: BinaryWriter, value: ResolveCodeLensRequest
) -> None: ...
def decode_resolve_code_lens_request(
    reader: BinaryReader,
) -> ResolveCodeLensRequest: ...
def to_json_resolve_code_lens_request(value: ResolveCodeLensRequest) -> Json: ...
def from_json_resolve_code_lens_request(value: Json) -> ResolveCodeLensRequest: ...

@dataclass(frozen=True, slots=True)
class CodeLens:
    """A code lens (inline annotation with optional command)."""

    # the range this lens applies to
    range: destack._generated.source.file.model.span.Span
    # the lens action
    action: CodeLensAction

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeLens: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CodeLens: ...

def encode_code_lens(writer: BinaryWriter, value: CodeLens) -> None: ...
def decode_code_lens(reader: BinaryReader) -> CodeLens: ...
def to_json_code_lens(value: CodeLens) -> Json: ...
def from_json_code_lens(value: Json) -> CodeLens: ...

@dataclass(frozen=True, slots=True)
class CodeLensActionReferences:
    """Show reference count."""

    # number of references (excluding declaration)
    count: int
    kind: typing.Literal["references"] = "references"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CodeLensActionImplementations:
    """Show implementation count."""

    # number of implementations
    count: int
    kind: typing.Literal["implementations"] = "implementations"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CodeLensActionRunTest:
    """Run test action."""

    # the test name
    test_name: str
    kind: typing.Literal["runTest"] = "runTest"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CodeLensActionDebugTest:
    """Debug test action."""

    # the test name
    test_name: str
    kind: typing.Literal["debugTest"] = "debugTest"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""The action for a code lens."""
CodeLensAction: typing.TypeAlias = (
    CodeLensActionReferences
    | CodeLensActionImplementations
    | CodeLensActionRunTest
    | CodeLensActionDebugTest
    | CodeLensActionCustom
)

def encode_code_lens_action(writer: BinaryWriter, value: CodeLensAction) -> None: ...
def decode_code_lens_action(reader: BinaryReader) -> CodeLensAction: ...
def to_json_code_lens_action(value: CodeLensAction) -> Json: ...
def from_json_code_lens_action(value: Json) -> CodeLensAction: ...

@dataclass(frozen=True, slots=True)
class CodeLensesResponse:
    """Response payload for code lenses queries."""

    # code lenses
    lenses: Sequence[CodeLens]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeLensesResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CodeLensesResponse: ...

def encode_code_lenses_response(
    writer: BinaryWriter, value: CodeLensesResponse
) -> None: ...
def decode_code_lenses_response(reader: BinaryReader) -> CodeLensesResponse: ...
def to_json_code_lenses_response(value: CodeLensesResponse) -> Json: ...
def from_json_code_lenses_response(value: Json) -> CodeLensesResponse: ...

@dataclass(frozen=True, slots=True)
class ResolveCodeLensResponse:
    """Response payload for code lens resolve queries."""

    # the resolved code lens
    lens: CodeLens

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ResolveCodeLensResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ResolveCodeLensResponse: ...

def encode_resolve_code_lens_response(
    writer: BinaryWriter, value: ResolveCodeLensResponse
) -> None: ...
def decode_resolve_code_lens_response(
    reader: BinaryReader,
) -> ResolveCodeLensResponse: ...
def to_json_resolve_code_lens_response(value: ResolveCodeLensResponse) -> Json: ...
def from_json_resolve_code_lens_response(value: Json) -> ResolveCodeLensResponse: ...

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
