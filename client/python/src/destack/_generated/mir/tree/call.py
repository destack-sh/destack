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
    json_object,
    json_string,
)

import destack._generated.mir.tree.node
import destack._generated.mir.tree.value


@dataclass(frozen=True, slots=True)
class Call:
    """Shared payload for one call-like instruction or terminator."""

    # the call arguments
    arguments: destack._generated.mir.tree.value.ValueSlice
    # the signature type for the callee
    signature: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Call:
        """Decode one Call."""
        return decode_call(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call(self)

    @classmethod
    def from_json(cls, value: Json) -> Call:
        """Return one Call from one JSON value."""
        return from_json_call(value)


def encode_call(writer: BinaryWriter, value: Call) -> None:
    """Encode one Call."""
    destack._generated.mir.tree.value.encode_value_slice(writer, value.arguments)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.signature)


def decode_call(reader: BinaryReader) -> Call:
    """Decode one Call."""
    arguments = destack._generated.mir.tree.value.decode_value_slice(reader)
    signature = destack._generated.mir.tree.node.decode_local_node_id(reader)

    return Call(
        arguments=arguments,
        signature=signature,
    )


def to_json_call(value: Call) -> Json:
    """Return one JSON value for one Call."""
    return {
        "arguments": destack._generated.mir.tree.value.to_json_value_slice(
            value.arguments
        ),
        "signature": destack._generated.mir.tree.node.to_json_local_node_id(
            value.signature
        ),
    }


def from_json_call(value: Json) -> Call:
    """Return one Call from one JSON value."""
    object_ = json_object(value)

    return Call(
        arguments=destack._generated.mir.tree.value.from_json_value_slice(
            json_field(object_, "arguments")
        ),
        signature=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "signature")
        ),
    )


@dataclass(frozen=True, slots=True)
class CallSiteInstruction:
    """Callsite stored as an instruction."""

    instruction: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["instruction"] = "instruction"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_site(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_site(self)


@dataclass(frozen=True, slots=True)
class CallSiteTerminator:
    """Callsite stored as a block terminator."""

    terminator: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["terminator"] = "terminator"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_site(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_site(self)


"""Stable identifier for one callsite inside a function body."""
CallSite: typing.TypeAlias = CallSiteInstruction | CallSiteTerminator


def encode_call_site(writer: BinaryWriter, value: CallSite) -> None:
    """Encode one CallSite."""
    if value.kind == "instruction":
        writer.write_unsigned(0)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.instruction)
    elif value.kind == "terminator":
        writer.write_unsigned(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.terminator)
    else:
        raise SerdeError("unknown enum variant")


def decode_call_site(reader: BinaryReader) -> CallSite:
    """Decode one CallSite."""
    variant = reader.read_number()

    if variant == 0:
        instruction = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return CallSiteInstruction(instruction=instruction)
    elif variant == 1:
        terminator = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return CallSiteTerminator(terminator=terminator)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_call_site(value: CallSite) -> Json:
    """Return one JSON value for one CallSite."""
    if value.kind == "instruction":
        return {
            "kind": "instruction",
            "instruction": destack._generated.mir.tree.node.to_json_local_node_id(
                value.instruction
            ),
        }
    elif value.kind == "terminator":
        return {
            "kind": "terminator",
            "terminator": destack._generated.mir.tree.node.to_json_local_node_id(
                value.terminator
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_call_site(value: Json) -> CallSite:
    """Return one CallSite from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "instruction":
        return CallSiteInstruction(
            instruction=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "instruction")
            )
        )
    elif kind == "terminator":
        return CallSiteTerminator(
            terminator=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "terminator")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Call",
    "encode_call",
    "decode_call",
    "to_json_call",
    "from_json_call",
    "CallSite",
    "encode_call_site",
    "decode_call_site",
    "to_json_call_site",
    "from_json_call_site",
    "CallSiteInstruction",
    "CallSiteTerminator",
]
