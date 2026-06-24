# generated bridge target, do not edit

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
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

from destack._impl.mir.metadata.function import (
    FunctionMetadataTableImpl,
)

import destack._generated.mir.metadata.effect
import destack._generated.mir.metadata.memory
import destack._generated.mir.tree.node


@dataclass(frozen=True, slots=True)
class FunctionMetadataTable(FunctionMetadataTableImpl):
    """Function and call metadata derived from semantic MIR."""

    # metadata keyed by function id
    functions: Mapping[destack._generated.mir.tree.node.LocalNodeId, FunctionMetadata]
    # metadata keyed by callsite
    calls: Mapping[CallSite, CallMetadata]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_metadata_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionMetadataTable:
        """Decode one FunctionMetadataTable."""
        return decode_function_metadata_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_metadata_table(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionMetadataTable:
        """Return one FunctionMetadataTable from one JSON value."""
        return from_json_function_metadata_table(value)


def encode_function_metadata_table(
    writer: BinaryWriter, value: FunctionMetadataTable
) -> None:
    """Encode one FunctionMetadataTable."""
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
        encode_function_metadata(writer, entry_value_functions_0[1])
    entries_value_calls_0 = []
    for key_value_calls_0, item_value_calls_0 in value.calls.items():

        def write_key_value_calls_0(writer: BinaryWriter) -> None:
            encode_call_site(writer, key_value_calls_0)

        key_bytes = nested_bytes(write_key_value_calls_0)
        entries_value_calls_0.append((key_value_calls_0, item_value_calls_0, key_bytes))
    entries_value_calls_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_calls_0))
    for entry_value_calls_0 in entries_value_calls_0:
        encode_call_site(writer, entry_value_calls_0[0])
        encode_call_metadata(writer, entry_value_calls_0[1])


def decode_function_metadata_table(reader: BinaryReader) -> FunctionMetadataTable:
    """Decode one FunctionMetadataTable."""
    functions = {
        destack._generated.mir.tree.node.decode_local_node_id(
            reader
        ): decode_function_metadata(reader)
        for _ in range(reader.read_number())
    }
    calls = {
        decode_call_site(reader): decode_call_metadata(reader)
        for _ in range(reader.read_number())
    }

    return FunctionMetadataTable(
        functions=functions,
        calls=calls,
    )


def to_json_function_metadata_table(value: FunctionMetadataTable) -> Json:
    """Return one JSON value for one FunctionMetadataTable."""
    return {
        "functions": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                to_json_function_metadata(item_0),
            ]
            for key_0, item_0 in value.functions.items()
        ],
        "calls": [
            [to_json_call_site(key_0), to_json_call_metadata(item_0)]
            for key_0, item_0 in value.calls.items()
        ],
    }


def from_json_function_metadata_table(value: Json) -> FunctionMetadataTable:
    """Return one FunctionMetadataTable from one JSON value."""
    object_ = json_object(value)

    return FunctionMetadataTable(
        functions={
            destack._generated.mir.tree.node.from_json_local_node_id(
                key_0
            ): from_json_function_metadata(item_0)
            for key_0, item_0 in json_array(json_field(object_, "functions"))
        },
        calls={
            from_json_call_site(key_0): from_json_call_metadata(item_0)
            for key_0, item_0 in json_array(json_field(object_, "calls"))
        },
    )


@dataclass(frozen=True, slots=True)
class FunctionMetadata:
    """Metadata for one function body or declaration."""

    # memory touched by this function
    memory: destack._generated.mir.metadata.effect.MemoryEffect
    # behavioral effects of this function
    behavior: destack._generated.mir.metadata.effect.FunctionBehavior
    # allocation result size relation when known
    allocation_size: destack._generated.mir.metadata.memory.AllocationSize | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_metadata(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionMetadata:
        """Decode one FunctionMetadata."""
        return decode_function_metadata(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_metadata(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionMetadata:
        """Return one FunctionMetadata from one JSON value."""
        return from_json_function_metadata(value)


def encode_function_metadata(writer: BinaryWriter, value: FunctionMetadata) -> None:
    """Encode one FunctionMetadata."""
    destack._generated.mir.metadata.effect.encode_memory_effect(writer, value.memory)
    destack._generated.mir.metadata.effect.encode_function_behavior(
        writer, value.behavior
    )
    if value.allocation_size is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.metadata.memory.encode_allocation_size(
            writer, value.allocation_size
        )


def decode_function_metadata(reader: BinaryReader) -> FunctionMetadata:
    """Decode one FunctionMetadata."""
    memory = destack._generated.mir.metadata.effect.decode_memory_effect(reader)
    behavior = destack._generated.mir.metadata.effect.decode_function_behavior(reader)
    allocation_size = reader.read_option(
        lambda: destack._generated.mir.metadata.memory.decode_allocation_size(reader)
    )

    return FunctionMetadata(
        memory=memory,
        behavior=behavior,
        allocation_size=allocation_size,
    )


def to_json_function_metadata(value: FunctionMetadata) -> Json:
    """Return one JSON value for one FunctionMetadata."""
    return {
        "memory": destack._generated.mir.metadata.effect.to_json_memory_effect(
            value.memory
        ),
        "behavior": destack._generated.mir.metadata.effect.to_json_function_behavior(
            value.behavior
        ),
        **(
            {}
            if value.allocation_size is None
            else {
                "allocationSize": destack._generated.mir.metadata.memory.to_json_allocation_size(
                    value.allocation_size
                )
            }
        ),
    }


def from_json_function_metadata(value: Json) -> FunctionMetadata:
    """Return one FunctionMetadata from one JSON value."""
    object_ = json_object(value)

    return FunctionMetadata(
        memory=destack._generated.mir.metadata.effect.from_json_memory_effect(
            json_field(object_, "memory")
        ),
        behavior=destack._generated.mir.metadata.effect.from_json_function_behavior(
            json_field(object_, "behavior")
        ),
        allocation_size=json_optional(
            object_,
            "allocationSize",
            lambda value: (
                destack._generated.mir.metadata.memory.from_json_allocation_size(value)
            ),
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


@dataclass(frozen=True, slots=True)
class CallMetadata:
    """Metadata for one callsite."""

    # memory touched by this call
    memory: destack._generated.mir.metadata.effect.MemoryEffect
    # behavioral effects of this call
    behavior: destack._generated.mir.metadata.effect.FunctionBehavior
    # allocation result size relation when known
    allocation_size: destack._generated.mir.metadata.memory.AllocationSize | None
    # resolved direct target when dispatch analysis proves one
    target: destack._generated.mir.tree.node.LocalNodeId | None
    # argument memory behavior when known
    arguments: Sequence[destack._generated.mir.metadata.memory.CallArgumentEffect]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_metadata(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallMetadata:
        """Decode one CallMetadata."""
        return decode_call_metadata(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_metadata(self)

    @classmethod
    def from_json(cls, value: Json) -> CallMetadata:
        """Return one CallMetadata from one JSON value."""
        return from_json_call_metadata(value)


def encode_call_metadata(writer: BinaryWriter, value: CallMetadata) -> None:
    """Encode one CallMetadata."""
    destack._generated.mir.metadata.effect.encode_memory_effect(writer, value.memory)
    destack._generated.mir.metadata.effect.encode_function_behavior(
        writer, value.behavior
    )
    if value.allocation_size is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.metadata.memory.encode_allocation_size(
            writer, value.allocation_size
        )
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.target)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.mir.metadata.memory.encode_call_argument_effect(
            writer, item_value_arguments_0
        )


def decode_call_metadata(reader: BinaryReader) -> CallMetadata:
    """Decode one CallMetadata."""
    memory = destack._generated.mir.metadata.effect.decode_memory_effect(reader)
    behavior = destack._generated.mir.metadata.effect.decode_function_behavior(reader)
    allocation_size = reader.read_option(
        lambda: destack._generated.mir.metadata.memory.decode_allocation_size(reader)
    )
    target = reader.read_option(
        lambda: destack._generated.mir.tree.node.decode_local_node_id(reader)
    )
    arguments = [
        destack._generated.mir.metadata.memory.decode_call_argument_effect(reader)
        for _ in range(reader.read_number())
    ]

    return CallMetadata(
        memory=memory,
        behavior=behavior,
        allocation_size=allocation_size,
        target=target,
        arguments=arguments,
    )


def to_json_call_metadata(value: CallMetadata) -> Json:
    """Return one JSON value for one CallMetadata."""
    return {
        "memory": destack._generated.mir.metadata.effect.to_json_memory_effect(
            value.memory
        ),
        "behavior": destack._generated.mir.metadata.effect.to_json_function_behavior(
            value.behavior
        ),
        **(
            {}
            if value.allocation_size is None
            else {
                "allocationSize": destack._generated.mir.metadata.memory.to_json_allocation_size(
                    value.allocation_size
                )
            }
        ),
        **(
            {}
            if value.target is None
            else {
                "target": destack._generated.mir.tree.node.to_json_local_node_id(
                    value.target
                )
            }
        ),
        "arguments": [
            destack._generated.mir.metadata.memory.to_json_call_argument_effect(item_0)
            for item_0 in value.arguments
        ],
    }


def from_json_call_metadata(value: Json) -> CallMetadata:
    """Return one CallMetadata from one JSON value."""
    object_ = json_object(value)

    return CallMetadata(
        memory=destack._generated.mir.metadata.effect.from_json_memory_effect(
            json_field(object_, "memory")
        ),
        behavior=destack._generated.mir.metadata.effect.from_json_function_behavior(
            json_field(object_, "behavior")
        ),
        allocation_size=json_optional(
            object_,
            "allocationSize",
            lambda value: (
                destack._generated.mir.metadata.memory.from_json_allocation_size(value)
            ),
        ),
        target=json_optional(
            object_,
            "target",
            lambda value: destack._generated.mir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        arguments=[
            destack._generated.mir.metadata.memory.from_json_call_argument_effect(
                item_0
            )
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


__all__ = [
    "FunctionMetadataTable",
    "encode_function_metadata_table",
    "decode_function_metadata_table",
    "to_json_function_metadata_table",
    "from_json_function_metadata_table",
    "FunctionMetadata",
    "encode_function_metadata",
    "decode_function_metadata",
    "to_json_function_metadata",
    "from_json_function_metadata",
    "CallSite",
    "encode_call_site",
    "decode_call_site",
    "to_json_call_site",
    "from_json_call_site",
    "CallSiteInstruction",
    "CallSiteTerminator",
    "CallMetadata",
    "encode_call_metadata",
    "decode_call_metadata",
    "to_json_call_metadata",
    "from_json_call_metadata",
]
