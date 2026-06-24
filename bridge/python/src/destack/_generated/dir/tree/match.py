# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.tree.node

"""The style of a match expression."""
MatchForm: typing.TypeAlias = typing.Literal["match"] | typing.Literal["switch"]


def encode_match_form(writer: BinaryWriter, value: MatchForm) -> None:
    """Encode one MatchForm."""
    if value == "match":
        writer.write_unsigned(0)
    elif value == "switch":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_match_form(reader: BinaryReader) -> MatchForm:
    """Decode one MatchForm."""
    variant = reader.read_number()

    if variant == 0:
        return "match"
    elif variant == 1:
        return "switch"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_match_form(value: MatchForm) -> Json:
    """Return one JSON value for one MatchForm."""
    return value


def from_json_match_form(value: Json) -> MatchForm:
    """Return one MatchForm from one JSON value."""
    variant = json_string(value)

    if variant == "match":
        return "match"
    elif variant == "switch":
        return "switch"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class MatchCaseExpression:
    """A match case with an expression body."""

    selector: MatchSelector
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_match_case(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_match_case(self)


@dataclass(frozen=True, slots=True)
class MatchCaseBlock:
    """A match case with a block body."""

    selector: MatchSelector
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["block"] = "block"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_match_case(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_match_case(self)


"""A MatchCase is a match case inside a Match expression."""
MatchCase: typing.TypeAlias = MatchCaseExpression | MatchCaseBlock


def encode_match_case(writer: BinaryWriter, value: MatchCase) -> None:
    """Encode one MatchCase."""
    if value.kind == "expression":
        writer.write_unsigned(0)
        encode_match_selector(writer, value.selector)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "block":
        writer.write_unsigned(1)
        encode_match_selector(writer, value.selector)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    else:
        raise SerdeError("unknown enum variant")


def decode_match_case(reader: BinaryReader) -> MatchCase:
    """Decode one MatchCase."""
    variant = reader.read_number()

    if variant == 0:
        selector = decode_match_selector(reader)
        body = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return MatchCaseExpression(
            selector=selector,
            body=body,
        )
    elif variant == 1:
        selector = decode_match_selector(reader)
        body = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return MatchCaseBlock(
            selector=selector,
            body=body,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_match_case(value: MatchCase) -> Json:
    """Return one JSON value for one MatchCase."""
    if value.kind == "expression":
        return {
            "kind": "expression",
            "selector": to_json_match_selector(value.selector),
            "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "block":
        return {
            "kind": "block",
            "selector": to_json_match_selector(value.selector),
            "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_match_case(value: Json) -> MatchCase:
    """Return one MatchCase from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "expression":
        return MatchCaseExpression(
            selector=from_json_match_selector(json_field(object_, "selector")),
            body=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "block":
        return MatchCaseBlock(
            selector=from_json_match_selector(json_field(object_, "selector")),
            body=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class MatchSelectorPattern:
    """A pattern with an optional guard (e.g., `x if x > 0`)."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    guard: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["pattern"] = "pattern"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_match_selector(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_match_selector(self)


@dataclass(frozen=True, slots=True)
class MatchSelectorDefault:
    """The default case in a switch statement (`default:`)."""

    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_match_selector(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_match_selector(self)


"""A MatchSelector determines which case is selected in a match/switch expression."""
MatchSelector: typing.TypeAlias = MatchSelectorPattern | MatchSelectorDefault


def encode_match_selector(writer: BinaryWriter, value: MatchSelector) -> None:
    """Encode one MatchSelector."""
    if value.kind == "pattern":
        writer.write_unsigned(0)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
        if value.guard is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.guard)
    elif value.kind == "default":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_match_selector(reader: BinaryReader) -> MatchSelector:
    """Decode one MatchSelector."""
    variant = reader.read_number()

    if variant == 0:
        pattern = destack._generated.dir.tree.node.decode_local_node_id(reader)
        guard = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return MatchSelectorPattern(
            pattern=pattern,
            guard=guard,
        )
    elif variant == 1:
        return MatchSelectorDefault()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_match_selector(value: MatchSelector) -> Json:
    """Return one JSON value for one MatchSelector."""
    if value.kind == "pattern":
        return {
            "kind": "pattern",
            "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                value.pattern
            ),
            **(
                {}
                if value.guard is None
                else {
                    "guard": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.guard
                    )
                }
            ),
        }
    elif value.kind == "default":
        return {
            "kind": "default",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_match_selector(value: Json) -> MatchSelector:
    """Return one MatchSelector from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "pattern":
        return MatchSelectorPattern(
            pattern=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
            guard=json_optional(
                object_,
                "guard",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "default":
        return MatchSelectorDefault()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
