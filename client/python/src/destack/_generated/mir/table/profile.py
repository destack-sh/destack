# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
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
    nested_bytes,
)

import destack._generated.mir.tree.call
import destack._generated.mir.tree.edge
import destack._generated.mir.tree.node
import destack._generated.mir.tree.value

"""Identifier for one emitted profile counter, positional within a function."""
CounterId: typing.TypeAlias = int


def encode_counter_id(writer: BinaryWriter, value: CounterId) -> None:
    """Encode one CounterId."""
    writer.write_unsigned(value)


def decode_counter_id(reader: BinaryReader) -> CounterId:
    """Decode one CounterId."""
    return reader.read_number()


def to_json_counter_id(value: CounterId) -> Json:
    """Return one JSON value for one CounterId."""
    return value


def from_json_counter_id(value: Json) -> CounterId:
    """Return one CounterId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class ProfileTable:
    """Static profile counter table for one MIR module."""

    # per-function profile counter tables
    functions: Mapping[
        destack._generated.mir.tree.node.LocalNodeId, FunctionProfileTable
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_profile_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProfileTable:
        """Decode one ProfileTable."""
        return decode_profile_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_profile_table(self)

    @classmethod
    def from_json(cls, value: Json) -> ProfileTable:
        """Return one ProfileTable from one JSON value."""
        return from_json_profile_table(value)


def encode_profile_table(writer: BinaryWriter, value: ProfileTable) -> None:
    """Encode one ProfileTable."""
    entries_value_functions_0 = []
    for key_value_functions_0, item_value_functions_0 in value.functions.items():

        def write_key_value_functions_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_functions_0
            )

        key_bytes = nested_bytes(write_key_value_functions_0)
        entries_value_functions_0.append(
            (key_value_functions_0, item_value_functions_0, key_bytes)
        )
    entries_value_functions_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_functions_0))
    for entry_value_functions_0 in entries_value_functions_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_functions_0[0]
        )
        encode_function_profile_table(writer, entry_value_functions_0[1])


def decode_profile_table(reader: BinaryReader) -> ProfileTable:
    """Decode one ProfileTable."""
    functions = {
        destack._generated.mir.tree.node.decode_local_node_id(
            reader
        ): decode_function_profile_table(reader)
        for _ in range(reader.read_number())
    }

    return ProfileTable(
        functions=functions,
    )


def to_json_profile_table(value: ProfileTable) -> Json:
    """Return one JSON value for one ProfileTable."""
    return {
        "functions": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                to_json_function_profile_table(item_0),
            ]
            for key_0, item_0 in value.functions.items()
        ],
    }


def from_json_profile_table(value: Json) -> ProfileTable:
    """Return one ProfileTable from one JSON value."""
    object_ = json_object(value)

    return ProfileTable(
        functions={
            destack._generated.mir.tree.node.from_json_local_node_id(
                key_0
            ): from_json_function_profile_table(item_0)
            for key_0, item_0 in json_array(json_field(object_, "functions"))
        },
    )


@dataclass(frozen=True, slots=True)
class FunctionProfileTable:
    """Static profile counter table for one function."""

    # control-flow hash guarding against stale profile application
    hash: FunctionHash
    # profile points indexed by counter id
    points: Sequence[ProfilePoint]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_profile_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionProfileTable:
        """Decode one FunctionProfileTable."""
        return decode_function_profile_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_profile_table(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionProfileTable:
        """Return one FunctionProfileTable from one JSON value."""
        return from_json_function_profile_table(value)


def encode_function_profile_table(
    writer: BinaryWriter, value: FunctionProfileTable
) -> None:
    """Encode one FunctionProfileTable."""
    encode_function_hash(writer, value.hash)
    writer.write_unsigned(len(value.points))
    for item_value_points_0 in value.points:
        encode_profile_point(writer, item_value_points_0)


def decode_function_profile_table(reader: BinaryReader) -> FunctionProfileTable:
    """Decode one FunctionProfileTable."""
    hash = decode_function_hash(reader)
    points = [decode_profile_point(reader) for _ in range(reader.read_number())]

    return FunctionProfileTable(
        hash=hash,
        points=points,
    )


def to_json_function_profile_table(value: FunctionProfileTable) -> Json:
    """Return one JSON value for one FunctionProfileTable."""
    return {
        "hash": to_json_function_hash(value.hash),
        "points": [to_json_profile_point(item_0) for item_0 in value.points],
    }


def from_json_function_profile_table(value: Json) -> FunctionProfileTable:
    """Return one FunctionProfileTable from one JSON value."""
    object_ = json_object(value)

    return FunctionProfileTable(
        hash=from_json_function_hash(json_field(object_, "hash")),
        points=[
            from_json_profile_point(item_0)
            for item_0 in json_array(json_field(object_, "points"))
        ],
    )


"""Structural hash of a function's profiled control flow, for stale detection."""
FunctionHash: typing.TypeAlias = int


def encode_function_hash(writer: BinaryWriter, value: FunctionHash) -> None:
    """Encode one FunctionHash."""
    writer.write_unsigned(value)


def decode_function_hash(reader: BinaryReader) -> FunctionHash:
    """Decode one FunctionHash."""
    return reader.read_number()


def to_json_function_hash(value: FunctionHash) -> Json:
    """Return one JSON value for one FunctionHash."""
    return value


def from_json_function_hash(value: Json) -> FunctionHash:
    """Return one FunctionHash from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class ProfilePointEntry:
    """Function entry execution count."""

    kind: typing.Literal["entry"] = "entry"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_profile_point(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_profile_point(self)


@dataclass(frozen=True, slots=True)
class ProfilePointEdge:
    """Control-flow edge count."""

    edge: destack._generated.mir.tree.edge.Edge
    kind: typing.Literal["edge"] = "edge"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_profile_point(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_profile_point(self)


@dataclass(frozen=True, slots=True)
class ProfilePointValue:
    """Value distribution for one SSA value."""

    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["value"] = "value"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_profile_point(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_profile_point(self)


@dataclass(frozen=True, slots=True)
class ProfilePointCallTarget:
    """Observed call target distribution for one callsite."""

    call_target: destack._generated.mir.tree.call.CallSite
    kind: typing.Literal["callTarget"] = "callTarget"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_profile_point(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_profile_point(self)


@dataclass(frozen=True, slots=True)
class ProfilePointReceiverType:
    """Observed receiver type distribution for one callsite."""

    receiver_type: destack._generated.mir.tree.call.CallSite
    kind: typing.Literal["receiverType"] = "receiverType"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_profile_point(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_profile_point(self)


@dataclass(frozen=True, slots=True)
class ProfilePointAllocation:
    """Observed allocation behavior for one allocation instruction."""

    allocation: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["allocation"] = "allocation"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_profile_point(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_profile_point(self)


@dataclass(frozen=True, slots=True)
class ProfilePointSuspension:
    """Suspension behavior for one instruction."""

    suspension: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["suspension"] = "suspension"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_profile_point(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_profile_point(self)


"""Semantic meaning of one profile counter."""
ProfilePoint: typing.TypeAlias = (
    ProfilePointEntry
    | ProfilePointEdge
    | ProfilePointValue
    | ProfilePointCallTarget
    | ProfilePointReceiverType
    | ProfilePointAllocation
    | ProfilePointSuspension
)


def encode_profile_point(writer: BinaryWriter, value: ProfilePoint) -> None:
    """Encode one ProfilePoint."""
    if value.kind == "entry":
        writer.write_unsigned(0)
    elif value.kind == "edge":
        writer.write_unsigned(1)
        destack._generated.mir.tree.edge.encode_edge(writer, value.edge)
    elif value.kind == "value":
        writer.write_unsigned(2)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "callTarget":
        writer.write_unsigned(3)
        destack._generated.mir.tree.call.encode_call_site(writer, value.call_target)
    elif value.kind == "receiverType":
        writer.write_unsigned(4)
        destack._generated.mir.tree.call.encode_call_site(writer, value.receiver_type)
    elif value.kind == "allocation":
        writer.write_unsigned(5)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.allocation)
    elif value.kind == "suspension":
        writer.write_unsigned(6)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.suspension)
    else:
        raise SerdeError("unknown enum variant")


def decode_profile_point(reader: BinaryReader) -> ProfilePoint:
    """Decode one ProfilePoint."""
    variant = reader.read_number()

    if variant == 0:
        return ProfilePointEntry()
    elif variant == 1:
        edge = destack._generated.mir.tree.edge.decode_edge(reader)

        return ProfilePointEdge(edge=edge)
    elif variant == 2:
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return ProfilePointValue(value=value_)
    elif variant == 3:
        call_target = destack._generated.mir.tree.call.decode_call_site(reader)

        return ProfilePointCallTarget(call_target=call_target)
    elif variant == 4:
        receiver_type = destack._generated.mir.tree.call.decode_call_site(reader)

        return ProfilePointReceiverType(receiver_type=receiver_type)
    elif variant == 5:
        allocation = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return ProfilePointAllocation(allocation=allocation)
    elif variant == 6:
        suspension = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return ProfilePointSuspension(suspension=suspension)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_profile_point(value: ProfilePoint) -> Json:
    """Return one JSON value for one ProfilePoint."""
    if value.kind == "entry":
        return {
            "kind": "entry",
        }
    elif value.kind == "edge":
        return {
            "kind": "edge",
            "edge": destack._generated.mir.tree.edge.to_json_edge(value.edge),
        }
    elif value.kind == "value":
        return {
            "kind": "value",
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "callTarget":
        return {
            "kind": "callTarget",
            "call_target": destack._generated.mir.tree.call.to_json_call_site(
                value.call_target
            ),
        }
    elif value.kind == "receiverType":
        return {
            "kind": "receiverType",
            "receiver_type": destack._generated.mir.tree.call.to_json_call_site(
                value.receiver_type
            ),
        }
    elif value.kind == "allocation":
        return {
            "kind": "allocation",
            "allocation": destack._generated.mir.tree.node.to_json_local_node_id(
                value.allocation
            ),
        }
    elif value.kind == "suspension":
        return {
            "kind": "suspension",
            "suspension": destack._generated.mir.tree.node.to_json_local_node_id(
                value.suspension
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_profile_point(value: Json) -> ProfilePoint:
    """Return one ProfilePoint from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "entry":
        return ProfilePointEntry()
    elif kind == "edge":
        return ProfilePointEdge(
            edge=destack._generated.mir.tree.edge.from_json_edge(
                json_field(object_, "edge")
            )
        )
    elif kind == "value":
        return ProfilePointValue(
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            )
        )
    elif kind == "callTarget":
        return ProfilePointCallTarget(
            call_target=destack._generated.mir.tree.call.from_json_call_site(
                json_field(object_, "call_target")
            )
        )
    elif kind == "receiverType":
        return ProfilePointReceiverType(
            receiver_type=destack._generated.mir.tree.call.from_json_call_site(
                json_field(object_, "receiver_type")
            )
        )
    elif kind == "allocation":
        return ProfilePointAllocation(
            allocation=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "allocation")
            )
        )
    elif kind == "suspension":
        return ProfilePointSuspension(
            suspension=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "suspension")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "CounterId",
    "encode_counter_id",
    "decode_counter_id",
    "to_json_counter_id",
    "from_json_counter_id",
    "ProfileTable",
    "encode_profile_table",
    "decode_profile_table",
    "to_json_profile_table",
    "from_json_profile_table",
    "FunctionProfileTable",
    "encode_function_profile_table",
    "decode_function_profile_table",
    "to_json_function_profile_table",
    "from_json_function_profile_table",
    "FunctionHash",
    "encode_function_hash",
    "decode_function_hash",
    "to_json_function_hash",
    "from_json_function_hash",
    "ProfilePoint",
    "encode_profile_point",
    "decode_profile_point",
    "to_json_profile_point",
    "from_json_profile_point",
    "ProfilePointEntry",
    "ProfilePointEdge",
    "ProfilePointValue",
    "ProfilePointCallTarget",
    "ProfilePointReceiverType",
    "ProfilePointAllocation",
    "ProfilePointSuspension",
]
