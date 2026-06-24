# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.tree.node

"""The style of a match expression."""
MatchForm: typing.TypeAlias = typing.Literal["match"] | typing.Literal["switch"]

def encode_match_form(writer: BinaryWriter, value: MatchForm) -> None: ...
def decode_match_form(reader: BinaryReader) -> MatchForm: ...
def to_json_match_form(value: MatchForm) -> Json: ...
def from_json_match_form(value: Json) -> MatchForm: ...

@dataclass(frozen=True, slots=True)
class MatchCaseExpression:
    """A match case with an expression body."""

    selector: MatchSelector
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MatchCaseBlock:
    """A match case with a block body."""

    selector: MatchSelector
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["block"] = "block"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A MatchCase is a match case inside a Match expression."""
MatchCase: typing.TypeAlias = MatchCaseExpression | MatchCaseBlock

def encode_match_case(writer: BinaryWriter, value: MatchCase) -> None: ...
def decode_match_case(reader: BinaryReader) -> MatchCase: ...
def to_json_match_case(value: MatchCase) -> Json: ...
def from_json_match_case(value: Json) -> MatchCase: ...

@dataclass(frozen=True, slots=True)
class MatchSelectorPattern:
    """A pattern with an optional guard (e.g., `x if x > 0`)."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    guard: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["pattern"] = "pattern"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MatchSelectorDefault:
    """The default case in a switch statement (`default:`)."""

    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A MatchSelector determines which case is selected in a match/switch expression."""
MatchSelector: typing.TypeAlias = MatchSelectorPattern | MatchSelectorDefault

def encode_match_selector(writer: BinaryWriter, value: MatchSelector) -> None: ...
def decode_match_selector(reader: BinaryReader) -> MatchSelector: ...
def to_json_match_selector(value: MatchSelector) -> Json: ...
def from_json_match_selector(value: Json) -> MatchSelector: ...

__all__ = [
    "MatchForm",
    "encode_match_form",
    "decode_match_form",
    "to_json_match_form",
    "from_json_match_form",
    "MatchCase",
    "encode_match_case",
    "decode_match_case",
    "to_json_match_case",
    "from_json_match_case",
    "MatchCaseExpression",
    "MatchCaseBlock",
    "MatchSelector",
    "encode_match_selector",
    "decode_match_selector",
    "to_json_match_selector",
    "from_json_match_selector",
    "MatchSelectorPattern",
    "MatchSelectorDefault",
]
