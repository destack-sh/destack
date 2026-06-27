# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.mir.tree.node


@dataclass(frozen=True, slots=True)
class Edge:
    """Control-flow edge selected by a terminator successor."""

    # the source block
    source: destack._generated.mir.tree.node.LocalNodeId
    # the successor field selected from the source terminator
    successor: Successor
    # the target block
    target: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_edge(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Edge:
        """Decode one Edge."""
        return decode_edge(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_edge(self)

    @classmethod
    def from_json(cls, value: Json) -> Edge:
        """Return one Edge from one JSON value."""
        return from_json_edge(value)


def encode_edge(writer: BinaryWriter, value: Edge) -> None:
    """Encode one Edge."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.source)
    encode_successor(writer, value.successor)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.target)


def decode_edge(reader: BinaryReader) -> Edge:
    """Decode one Edge."""
    source = destack._generated.mir.tree.node.decode_local_node_id(reader)
    successor = decode_successor(reader)
    target = destack._generated.mir.tree.node.decode_local_node_id(reader)

    return Edge(
        source=source,
        successor=successor,
        target=target,
    )


def to_json_edge(value: Edge) -> Json:
    """Return one JSON value for one Edge."""
    return {
        "source": destack._generated.mir.tree.node.to_json_local_node_id(value.source),
        "successor": to_json_successor(value.successor),
        "target": destack._generated.mir.tree.node.to_json_local_node_id(value.target),
    }


def from_json_edge(value: Json) -> Edge:
    """Return one Edge from one JSON value."""
    object_ = json_object(value)

    return Edge(
        source=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "source")
        ),
        successor=from_json_successor(json_field(object_, "successor")),
        target=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "target")
        ),
    )


@dataclass(frozen=True, slots=True)
class SuccessorJump:
    """The target of an unconditional jump."""

    kind: typing.Literal["jump"] = "jump"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorCallReturn:
    """The return continuation of a call terminator."""

    kind: typing.Literal["callReturn"] = "callReturn"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorCallUnwind:
    """The unwind continuation of a call terminator."""

    kind: typing.Literal["callUnwind"] = "callUnwind"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorBranchThen:
    """The then target of a branch terminator."""

    kind: typing.Literal["branchThen"] = "branchThen"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorBranchElse:
    """The else target of a branch terminator."""

    kind: typing.Literal["branchElse"] = "branchElse"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorCheckSuccess:
    """The success target of a check terminator."""

    kind: typing.Literal["checkSuccess"] = "checkSuccess"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorCheckFailure:
    """The failure target of a check terminator."""

    kind: typing.Literal["checkFailure"] = "checkFailure"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorTrySuccess:
    """The success target of a fallible terminator."""

    kind: typing.Literal["trySuccess"] = "trySuccess"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorTryFailure:
    """The failure target of a fallible terminator."""

    kind: typing.Literal["tryFailure"] = "tryFailure"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorSwitchCase:
    """One switch case target."""

    # the matched case value
    value: int
    kind: typing.Literal["switchCase"] = "switchCase"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorSwitchDefault:
    """The default target of a switch terminator."""

    kind: typing.Literal["switchDefault"] = "switchDefault"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorYieldResume:
    """The resume target of a yield terminator."""

    kind: typing.Literal["yieldResume"] = "yieldResume"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


@dataclass(frozen=True, slots=True)
class SuccessorYieldUnwind:
    """The unwind target of a yield terminator."""

    kind: typing.Literal["yieldUnwind"] = "yieldUnwind"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_successor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_successor(self)


"""Successor selected by one MIR terminator."""
Successor: typing.TypeAlias = (
    SuccessorJump
    | SuccessorCallReturn
    | SuccessorCallUnwind
    | SuccessorBranchThen
    | SuccessorBranchElse
    | SuccessorCheckSuccess
    | SuccessorCheckFailure
    | SuccessorTrySuccess
    | SuccessorTryFailure
    | SuccessorSwitchCase
    | SuccessorSwitchDefault
    | SuccessorYieldResume
    | SuccessorYieldUnwind
)


def encode_successor(writer: BinaryWriter, value: Successor) -> None:
    """Encode one Successor."""
    if value.kind == "jump":
        writer.write_unsigned(0)
    elif value.kind == "callReturn":
        writer.write_unsigned(1)
    elif value.kind == "callUnwind":
        writer.write_unsigned(2)
    elif value.kind == "branchThen":
        writer.write_unsigned(3)
    elif value.kind == "branchElse":
        writer.write_unsigned(4)
    elif value.kind == "checkSuccess":
        writer.write_unsigned(5)
    elif value.kind == "checkFailure":
        writer.write_unsigned(6)
    elif value.kind == "trySuccess":
        writer.write_unsigned(7)
    elif value.kind == "tryFailure":
        writer.write_unsigned(8)
    elif value.kind == "switchCase":
        writer.write_unsigned(9)
        writer.write_signed(value.value)
    elif value.kind == "switchDefault":
        writer.write_unsigned(10)
    elif value.kind == "yieldResume":
        writer.write_unsigned(11)
    elif value.kind == "yieldUnwind":
        writer.write_unsigned(12)
    else:
        raise SerdeError("unknown enum variant")


def decode_successor(reader: BinaryReader) -> Successor:
    """Decode one Successor."""
    variant = reader.read_number()

    if variant == 0:
        return SuccessorJump()
    elif variant == 1:
        return SuccessorCallReturn()
    elif variant == 2:
        return SuccessorCallUnwind()
    elif variant == 3:
        return SuccessorBranchThen()
    elif variant == 4:
        return SuccessorBranchElse()
    elif variant == 5:
        return SuccessorCheckSuccess()
    elif variant == 6:
        return SuccessorCheckFailure()
    elif variant == 7:
        return SuccessorTrySuccess()
    elif variant == 8:
        return SuccessorTryFailure()
    elif variant == 9:
        value_ = reader.read_signed_number()

        return SuccessorSwitchCase(
            value=value_,
        )
    elif variant == 10:
        return SuccessorSwitchDefault()
    elif variant == 11:
        return SuccessorYieldResume()
    elif variant == 12:
        return SuccessorYieldUnwind()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_successor(value: Successor) -> Json:
    """Return one JSON value for one Successor."""
    if value.kind == "jump":
        return {
            "kind": "jump",
        }
    elif value.kind == "callReturn":
        return {
            "kind": "callReturn",
        }
    elif value.kind == "callUnwind":
        return {
            "kind": "callUnwind",
        }
    elif value.kind == "branchThen":
        return {
            "kind": "branchThen",
        }
    elif value.kind == "branchElse":
        return {
            "kind": "branchElse",
        }
    elif value.kind == "checkSuccess":
        return {
            "kind": "checkSuccess",
        }
    elif value.kind == "checkFailure":
        return {
            "kind": "checkFailure",
        }
    elif value.kind == "trySuccess":
        return {
            "kind": "trySuccess",
        }
    elif value.kind == "tryFailure":
        return {
            "kind": "tryFailure",
        }
    elif value.kind == "switchCase":
        return {
            "kind": "switchCase",
            "value": value.value,
        }
    elif value.kind == "switchDefault":
        return {
            "kind": "switchDefault",
        }
    elif value.kind == "yieldResume":
        return {
            "kind": "yieldResume",
        }
    elif value.kind == "yieldUnwind":
        return {
            "kind": "yieldUnwind",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_successor(value: Json) -> Successor:
    """Return one Successor from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "jump":
        return SuccessorJump()
    elif kind == "callReturn":
        return SuccessorCallReturn()
    elif kind == "callUnwind":
        return SuccessorCallUnwind()
    elif kind == "branchThen":
        return SuccessorBranchThen()
    elif kind == "branchElse":
        return SuccessorBranchElse()
    elif kind == "checkSuccess":
        return SuccessorCheckSuccess()
    elif kind == "checkFailure":
        return SuccessorCheckFailure()
    elif kind == "trySuccess":
        return SuccessorTrySuccess()
    elif kind == "tryFailure":
        return SuccessorTryFailure()
    elif kind == "switchCase":
        return SuccessorSwitchCase(
            value=json_int(json_field(object_, "value")),
        )
    elif kind == "switchDefault":
        return SuccessorSwitchDefault()
    elif kind == "yieldResume":
        return SuccessorYieldResume()
    elif kind == "yieldUnwind":
        return SuccessorYieldUnwind()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Edge",
    "encode_edge",
    "decode_edge",
    "to_json_edge",
    "from_json_edge",
    "Successor",
    "encode_successor",
    "decode_successor",
    "to_json_successor",
    "from_json_successor",
    "SuccessorJump",
    "SuccessorCallReturn",
    "SuccessorCallUnwind",
    "SuccessorBranchThen",
    "SuccessorBranchElse",
    "SuccessorCheckSuccess",
    "SuccessorCheckFailure",
    "SuccessorTrySuccess",
    "SuccessorTryFailure",
    "SuccessorSwitchCase",
    "SuccessorSwitchDefault",
    "SuccessorYieldResume",
    "SuccessorYieldUnwind",
]
