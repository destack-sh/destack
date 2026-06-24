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
    json_int,
    json_object,
    json_string,
    nested_bytes,
)

import destack._generated.mir.metadata.frame
import destack._generated.mir.tree.node
import destack._generated.mir.tree.value
import destack._generated.program.function
import destack._generated.program.vm.instruction
import destack._generated.program.vm.range


@dataclass(frozen=True, slots=True)
class FunctionTable:
    """Lowered function registry owned by one program."""

    # lowered functions by dense index
    functions: Sequence[Function]
    # call target by function id
    target_by_id: Mapping[destack._generated.program.function.FunctionId, CallTarget]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionTable:
        """Decode one FunctionTable."""
        return decode_function_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_table(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionTable:
        """Return one FunctionTable from one JSON value."""
        return from_json_function_table(value)


def encode_function_table(writer: BinaryWriter, value: FunctionTable) -> None:
    """Encode one FunctionTable."""
    writer.write_unsigned(len(value.functions))
    for item_value_functions_0 in value.functions:
        encode_function(writer, item_value_functions_0)
    entries_value_target_by_id_0 = []
    for (
        key_value_target_by_id_0,
        item_value_target_by_id_0,
    ) in value.target_by_id.items():

        def write_key_value_target_by_id_0(writer: BinaryWriter) -> None:
            destack._generated.program.function.encode_function_id(
                writer, key_value_target_by_id_0
            )

        key_bytes = nested_bytes(write_key_value_target_by_id_0)
        entries_value_target_by_id_0.append(
            (key_value_target_by_id_0, item_value_target_by_id_0, key_bytes)
        )
    entries_value_target_by_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_target_by_id_0))
    for entry_value_target_by_id_0 in entries_value_target_by_id_0:
        destack._generated.program.function.encode_function_id(
            writer, entry_value_target_by_id_0[0]
        )
        encode_call_target(writer, entry_value_target_by_id_0[1])


def decode_function_table(reader: BinaryReader) -> FunctionTable:
    """Decode one FunctionTable."""
    functions = [decode_function(reader) for _ in range(reader.read_number())]
    target_by_id = {
        destack._generated.program.function.decode_function_id(
            reader
        ): decode_call_target(reader)
        for _ in range(reader.read_number())
    }

    return FunctionTable(
        functions=functions,
        target_by_id=target_by_id,
    )


def to_json_function_table(value: FunctionTable) -> Json:
    """Return one JSON value for one FunctionTable."""
    return {
        "functions": [to_json_function(item_0) for item_0 in value.functions],
        "targetById": [
            [
                destack._generated.program.function.to_json_function_id(key_0),
                to_json_call_target(item_0),
            ]
            for key_0, item_0 in value.target_by_id.items()
        ],
    }


def from_json_function_table(value: Json) -> FunctionTable:
    """Return one FunctionTable from one JSON value."""
    object_ = json_object(value)

    return FunctionTable(
        functions=[
            from_json_function(item_0)
            for item_0 in json_array(json_field(object_, "functions"))
        ],
        target_by_id={
            destack._generated.program.function.from_json_function_id(
                key_0
            ): from_json_call_target(item_0)
            for key_0, item_0 in json_array(json_field(object_, "targetById"))
        },
    )


@dataclass(frozen=True, slots=True)
class Function:
    """Lowered function with executable code and frame metadata."""

    # runtime function id
    function: destack._generated.program.function.FunctionId
    # the logical frame layout for this function
    frame_layout: destack._generated.mir.metadata.frame.FrameLayoutId
    # function parameters
    parameters: destack._generated.program.vm.range.ArgumentRange
    # entry block index
    entry: int
    # contiguous instruction code
    code: Sequence[destack._generated.program.vm.instruction.Instruction]
    # lowered block ranges
    blocks: Sequence[Block]
    # pool of argument values referenced by ranges
    argument_pool: Sequence[destack._generated.mir.tree.value.Value]
    # pool of value move pairs referenced by ranges
    move_pool: Sequence[destack._generated.program.vm.range.MovePair]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Function:
        """Decode one Function."""
        return decode_function(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function(self)

    @classmethod
    def from_json(cls, value: Json) -> Function:
        """Return one Function from one JSON value."""
        return from_json_function(value)


def encode_function(writer: BinaryWriter, value: Function) -> None:
    """Encode one Function."""
    destack._generated.program.function.encode_function_id(writer, value.function)
    destack._generated.mir.metadata.frame.encode_frame_layout_id(
        writer, value.frame_layout
    )
    destack._generated.program.vm.range.encode_argument_range(writer, value.parameters)
    writer.write_unsigned(value.entry)
    writer.write_unsigned(len(value.code))
    for item_value_code_0 in value.code:
        destack._generated.program.vm.instruction.encode_instruction(
            writer, item_value_code_0
        )
    writer.write_unsigned(len(value.blocks))
    for item_value_blocks_0 in value.blocks:
        encode_block(writer, item_value_blocks_0)
    writer.write_unsigned(len(value.argument_pool))
    for item_value_argument_pool_0 in value.argument_pool:
        destack._generated.mir.tree.value.encode_value(
            writer, item_value_argument_pool_0
        )
    writer.write_unsigned(len(value.move_pool))
    for item_value_move_pool_0 in value.move_pool:
        destack._generated.program.vm.range.encode_move_pair(
            writer, item_value_move_pool_0
        )


def decode_function(reader: BinaryReader) -> Function:
    """Decode one Function."""
    function = destack._generated.program.function.decode_function_id(reader)
    frame_layout = destack._generated.mir.metadata.frame.decode_frame_layout_id(reader)
    parameters = destack._generated.program.vm.range.decode_argument_range(reader)
    entry = reader.read_number()
    code = [
        destack._generated.program.vm.instruction.decode_instruction(reader)
        for _ in range(reader.read_number())
    ]
    blocks = [decode_block(reader) for _ in range(reader.read_number())]
    argument_pool = [
        destack._generated.mir.tree.value.decode_value(reader)
        for _ in range(reader.read_number())
    ]
    move_pool = [
        destack._generated.program.vm.range.decode_move_pair(reader)
        for _ in range(reader.read_number())
    ]

    return Function(
        function=function,
        frame_layout=frame_layout,
        parameters=parameters,
        entry=entry,
        code=code,
        blocks=blocks,
        argument_pool=argument_pool,
        move_pool=move_pool,
    )


def to_json_function(value: Function) -> Json:
    """Return one JSON value for one Function."""
    return {
        "function": destack._generated.program.function.to_json_function_id(
            value.function
        ),
        "frameLayout": destack._generated.mir.metadata.frame.to_json_frame_layout_id(
            value.frame_layout
        ),
        "parameters": destack._generated.program.vm.range.to_json_argument_range(
            value.parameters
        ),
        "entry": value.entry,
        "code": [
            destack._generated.program.vm.instruction.to_json_instruction(item_0)
            for item_0 in value.code
        ],
        "blocks": [to_json_block(item_0) for item_0 in value.blocks],
        "argumentPool": [
            destack._generated.mir.tree.value.to_json_value(item_0)
            for item_0 in value.argument_pool
        ],
        "movePool": [
            destack._generated.program.vm.range.to_json_move_pair(item_0)
            for item_0 in value.move_pool
        ],
    }


def from_json_function(value: Json) -> Function:
    """Return one Function from one JSON value."""
    object_ = json_object(value)

    return Function(
        function=destack._generated.program.function.from_json_function_id(
            json_field(object_, "function")
        ),
        frame_layout=destack._generated.mir.metadata.frame.from_json_frame_layout_id(
            json_field(object_, "frameLayout")
        ),
        parameters=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "parameters")
        ),
        entry=json_int(json_field(object_, "entry")),
        code=[
            destack._generated.program.vm.instruction.from_json_instruction(item_0)
            for item_0 in json_array(json_field(object_, "code"))
        ],
        blocks=[
            from_json_block(item_0)
            for item_0 in json_array(json_field(object_, "blocks"))
        ],
        argument_pool=[
            destack._generated.mir.tree.value.from_json_value(item_0)
            for item_0 in json_array(json_field(object_, "argumentPool"))
        ],
        move_pool=[
            destack._generated.program.vm.range.from_json_move_pair(item_0)
            for item_0 in json_array(json_field(object_, "movePool"))
        ],
    )


@dataclass(frozen=True, slots=True)
class Block:
    """Lowered basic block position inside one function."""

    # original MIR block id
    mir_block: destack._generated.mir.tree.node.LocalNodeId
    # first instruction in the function code
    start: int
    # number of instructions in this block
    len: int
    # source MIR point for each lowered PC in this block
    mir_point_by_pc: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_block(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Block:
        """Decode one Block."""
        return decode_block(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_block(self)

    @classmethod
    def from_json(cls, value: Json) -> Block:
        """Return one Block from one JSON value."""
        return from_json_block(value)


def encode_block(writer: BinaryWriter, value: Block) -> None:
    """Encode one Block."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.mir_block)
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.len)
    writer.write_unsigned(len(value.mir_point_by_pc))
    for item_value_mir_point_by_pc_0 in value.mir_point_by_pc:
        writer.write_unsigned(item_value_mir_point_by_pc_0)


def decode_block(reader: BinaryReader) -> Block:
    """Decode one Block."""
    mir_block = destack._generated.mir.tree.node.decode_local_node_id(reader)
    start = reader.read_number()
    len = reader.read_number()
    mir_point_by_pc = [reader.read_number() for _ in range(reader.read_number())]

    return Block(
        mir_block=mir_block,
        start=start,
        len=len,
        mir_point_by_pc=mir_point_by_pc,
    )


def to_json_block(value: Block) -> Json:
    """Return one JSON value for one Block."""
    return {
        "mirBlock": destack._generated.mir.tree.node.to_json_local_node_id(
            value.mir_block
        ),
        "start": value.start,
        "len": value.len,
        "mirPointByPc": [item_0 for item_0 in value.mir_point_by_pc],
    }


def from_json_block(value: Json) -> Block:
    """Return one Block from one JSON value."""
    object_ = json_object(value)

    return Block(
        mir_block=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "mirBlock")
        ),
        start=json_int(json_field(object_, "start")),
        len=json_int(json_field(object_, "len")),
        mir_point_by_pc=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "mirPointByPc"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CallTargetImport:
    """The function id names one imported function."""

    kind: typing.Literal["import"] = "import"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_target(self)


@dataclass(frozen=True, slots=True)
class CallTargetLocal:
    """The function id names one lowered function."""

    local: int
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_target(self)


"""Program call target for one function id."""
CallTarget: typing.TypeAlias = CallTargetImport | CallTargetLocal


def encode_call_target(writer: BinaryWriter, value: CallTarget) -> None:
    """Encode one CallTarget."""
    if value.kind == "import":
        writer.write_unsigned(0)
    elif value.kind == "local":
        writer.write_unsigned(1)
        writer.write_unsigned(value.local)
    else:
        raise SerdeError("unknown enum variant")


def decode_call_target(reader: BinaryReader) -> CallTarget:
    """Decode one CallTarget."""
    variant = reader.read_number()

    if variant == 0:
        return CallTargetImport()
    elif variant == 1:
        local = reader.read_number()

        return CallTargetLocal(local=local)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_call_target(value: CallTarget) -> Json:
    """Return one JSON value for one CallTarget."""
    if value.kind == "import":
        return {
            "kind": "import",
        }
    elif value.kind == "local":
        return {
            "kind": "local",
            "local": value.local,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_call_target(value: Json) -> CallTarget:
    """Return one CallTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "import":
        return CallTargetImport()
    elif kind == "local":
        return CallTargetLocal(local=json_int(json_field(object_, "local")))
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class SwitchCase:
    """One lowered switch case."""

    # match value
    value: int
    # target block
    target: int
    # block parameter moves
    moves: destack._generated.program.vm.range.MoveRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_switch_case(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SwitchCase:
        """Decode one SwitchCase."""
        return decode_switch_case(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_switch_case(self)

    @classmethod
    def from_json(cls, value: Json) -> SwitchCase:
        """Return one SwitchCase from one JSON value."""
        return from_json_switch_case(value)


def encode_switch_case(writer: BinaryWriter, value: SwitchCase) -> None:
    """Encode one SwitchCase."""
    writer.write_signed(value.value)
    writer.write_unsigned(value.target)
    destack._generated.program.vm.range.encode_move_range(writer, value.moves)


def decode_switch_case(reader: BinaryReader) -> SwitchCase:
    """Decode one SwitchCase."""
    value_ = reader.read_signed_number()
    target = reader.read_number()
    moves = destack._generated.program.vm.range.decode_move_range(reader)

    return SwitchCase(
        value=value_,
        target=target,
        moves=moves,
    )


def to_json_switch_case(value: SwitchCase) -> Json:
    """Return one JSON value for one SwitchCase."""
    return {
        "value": value.value,
        "target": value.target,
        "moves": destack._generated.program.vm.range.to_json_move_range(value.moves),
    }


def from_json_switch_case(value: Json) -> SwitchCase:
    """Return one SwitchCase from one JSON value."""
    object_ = json_object(value)

    return SwitchCase(
        value=json_int(json_field(object_, "value")),
        target=json_int(json_field(object_, "target")),
        moves=destack._generated.program.vm.range.from_json_move_range(
            json_field(object_, "moves")
        ),
    )


__all__ = [
    "FunctionTable",
    "encode_function_table",
    "decode_function_table",
    "to_json_function_table",
    "from_json_function_table",
    "Function",
    "encode_function",
    "decode_function",
    "to_json_function",
    "from_json_function",
    "Block",
    "encode_block",
    "decode_block",
    "to_json_block",
    "from_json_block",
    "CallTarget",
    "encode_call_target",
    "decode_call_target",
    "to_json_call_target",
    "from_json_call_target",
    "CallTargetImport",
    "CallTargetLocal",
    "SwitchCase",
    "encode_switch_case",
    "decode_switch_case",
    "to_json_switch_case",
    "from_json_switch_case",
]
