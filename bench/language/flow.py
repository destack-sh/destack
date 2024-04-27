from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.const import EnumType, NodeType, StructType, enum_
from bench.language.node import Node, Struct, node, struct
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_regular,
    p_secret_value_packed,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import validate_name
from bench.language.value import HasValues
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Code, Expression, Text, Trigger, TypeInfo

# pyright: reportIncompatibleVariableOverride=false

@enum_(EnumType.STEP_TYPE)
class StepType(IdEnum):
    BLANK = 1
    # trigger
    TRIGGER = 10
    # function
    RUN_BLOCK = 20
    # conditional
    BRANCH = 30
    FILTER = 31
    LOOP = 32
    # organizational
    GROUP = 40


@struct(StructType.STEP_CONNECTION)
class StepConnection(Struct):
    """A connection between to a Step in a Flow."""

    source: Union["Block", "Step", "Trigger"] = p_regular(
        30, require=True, references=NodeType.STEP
    )
    # ...?


@node(NodeType.STEP)
class Step(Node, HasValues):
    """
    A logic, data or control flow unit in a Flow (Block).
    NOTE: steps only track connections coming in.
    """

    parent: Union["Block", "Step"] = p_node_parent(4, NodeType.BLOCK, NodeType.STEP) 

    type: StepType = p_internal(30, default=StepType.BLANK)
    # custom type?
    name: str | None = p_regular(32, default=None, validate=validate_name)
    order_key: str = p_internal(33, default=INTEGER_ZERO) 
    text: Optional["Text"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.TEXT
    )
    code: Optional["Code"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.CODE
    )
    connections: list[StepConnection] = p_regular(36, array=True, struct=StructType.STEP_CONNECTION)

    value_type: Optional["TypeInfo"] = p_regular(40, default=None, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(41)
    secret_value_packed: Any = p_secret_value_packed(42)
    value = p_value_runtime(41, 42)
    node: Optional["Block"] = p_regular(43, require=False, references=NodeType.BLOCK)
    condition: Optional["Expression"] = p_regular(
        46, require=False, array=False, default=None, struct=StructType.EXPRESSION
    )

    # flags
    # ...?
