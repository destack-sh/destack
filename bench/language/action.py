from typing import TYPE_CHECKING, Any, Optional, Union, cast

from bench.language.const import BlockType, EnumType, NodeType, ObjectKind, StructType, enum_
from bench.language.field import TypeBase
from bench.language.node import BuiltinObject, NodeReference, Struct, object_, struct_
from bench.language.property import p_regular, p_value_packed, p_value_runtime
from bench.language.validation import constraint
from bench.language.value import coerce_custom_object
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Code, ObjectMapping, Pipe, RunOptions, Step, Text


@enum_(EnumType.ACTION_MODE)
class ActionMode(IdEnum):
    STRICT = 1  # always run as specified
    ADAPTIVE = 2  # run as specified by default but adapt if out of date or error
    DYNAMIC = 3  # dynamically adapt to inputs every time


@object_()
class ActionBase(BuiltinObject):
    """Common base for 'actions' that do something using code somehow."""

    mode: ActionMode = p_regular(100, default=ActionMode.ADAPTIVE)
    text: Optional["Text"] = p_regular(
        101,
        default=None,
        require=False,
        array=False,
        struct=StructType.TEXT,
        description="Text description of this action.",
    )
    code: Optional["Code"] = p_regular(
        102,
        default=None,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="Current implementation code for this action.",
    )
    tools: list["Block"] = p_regular(
        103,
        require=False,
        array=True,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.ACTION, BlockType.FLOW]),
        description="Available implementations for this action.",
    )
    run_options: Optional["RunOptions"] = p_regular(
        110, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )
    if TYPE_CHECKING:
        node_ptr: Optional["NodeReference"] = None


@struct_(StructType.CALL)
class Call(Struct):
    """A Call to a Run (inside/from the current Run usually)."""

    node: Union["Block", "Step"] = p_regular(
        30,
        require=True,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.ACTION, BlockType.FLOW]),
    )
    inputs_packed: Any = p_value_packed(31)
    inputs: Any = p_value_runtime(
        31, kind=ObjectKind.INPUT, typ=lambda self: cast(Call, self).input_type
    )
    mapping: Optional["ObjectMapping"] = p_regular(
        50,
        require=False,
        array=False,
        struct=StructType.OBJECT_MAPPING,
        description="Mapping for inputs from current node into called node.",
    )
    mapping_code: Optional["Code"] = p_regular(
        51,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="Mapping for outputs from called node into new node. Takes precedence over mapping.",
    )

    @property
    def input_type(self) -> Optional["TypeBase"]:
        node = self.node
        return node.input_type if node is not None else None

    @staticmethod
    def new(node: "Block | Step", inputs: Any, **kwargs) -> "Call":
        input_type = node.input_type
        assert input_type is not None, f"no input type for {node!r}"
        return Call(
            node=node, inputs=coerce_custom_object(ObjectKind.INPUT, input_type, inputs), **kwargs
        )


@struct_(StructType.CONTINUE)
class Continue(Struct):
    """A "Continuation" of a Run somewhere (like in a Flow)."""

    node: Union["Block", "Step", "Pipe"] = p_regular(
        30,
        require=True,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.ACTION, BlockType.FLOW]),
    )
    # inputs, ...?
    mapping: Optional["ObjectMapping"] = p_regular(
        50,
        require=False,
        array=False,
        struct=StructType.OBJECT_MAPPING,
        description="Mapping for inputs from current node into next node.",
    )
    mapping_code: Optional["Code"] = p_regular(
        51,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="Mapping to get inputs for next node. Takes precedence over mapping.",
    )

    @staticmethod
    def new(node: "Block | Step | Pipe", **kwargs) -> "Continue":
        return Continue(node=node, **kwargs)
