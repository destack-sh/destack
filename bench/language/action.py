from typing import TYPE_CHECKING, Optional, Union

from bench.language.const import BlockType, EnumType, NodeType, StructType, enum_
from bench.language.node import BuiltinObject, NodeReference, Struct, object_, struct_
from bench.language.property import p_regular
from bench.language.validation import constraint
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Code, Pipe, Run, RunOptions, Step, Text


@enum_(EnumType.ACTION_MODE)
class ActionMode(IdEnum):
    STRICT = 1  # MUST run the code/node exactly
    ADAPTIVE = 2  # SHOULD run the code/node, may adapt it
    LENIENT = 3  # MAY run the code/node, may adapt it
    DYNAMIC = 4  # CAN run the code/node, may do something else


@object_()
class ActionBase(BuiltinObject):
    """Common base for 'actions' that do something using code somehow."""

    mode: ActionMode = p_regular(100, default=ActionMode.ADAPTIVE)
    text: Optional["Text"] = p_regular(
        101, default=None, require=False, array=False, struct=StructType.TEXT
    )
    code: Optional["Code"] = p_regular(
        102, default=None, require=False, array=False, struct=StructType.CODE
    )
    node: Optional["Block"] = p_regular(
        103,
        require=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.ACTION, BlockType.FLOW]),
    )
    run_options: Optional["RunOptions"] = p_regular(
        110, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )
    if TYPE_CHECKING:
        node_ptr: Optional["NodeReference"] = None


@struct_(StructType.CONTEXT)
class Context(Struct):
    runs: list["Run"] = p_regular(30, require=True, array=True, references=NodeType.RUN)


@struct_(StructType.CONTINUATION)
class Continuation(Struct):
    """A context-specific continuation for a Run (like in a Flow)."""

    node: Optional[Union["Block", "Step", "Pipe"]] = p_regular(
        30,
        require=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.ACTION, BlockType.FLOW]),
    )
    # inputs, ...?

    @staticmethod
    def new(node: "Block | Step | Pipe", **kwargs) -> "Continuation":
        return Continuation(node=node, **kwargs)
