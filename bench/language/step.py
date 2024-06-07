from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.const import EnumType, NodeType, StructType, enum_
from bench.language.graph import NodeList
from bench.language.issue import Issue
from bench.language.node import SourceNode, Struct, node, struct
from bench.language.property import (
    p_internal,
    p_node_child,
    p_node_parent,
    p_regular,
    p_secret_value_packed,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import NAME_CONSTRAINT
from bench.language.value import HasValues
from bench.proto.wire import StepData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Code, Expression, Field, RunOptions, Text, Trigger, TypeInfo

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.STEP_TYPE)
class StepType(IdEnum):
    BLANK = 1
    # trigger
    TRIGGER = 10
    # function
    RUN_BLOCK = 20
    RUN_STEP = 21
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
class Step(SourceNode[StepData], HasValues):
    """
    An informational, logic, data or control flow unit in a Flow (Block).
    NOTE: steps only track incoming connections.
    """

    parent: Union["Block", "Step"] = p_node_parent(4, NodeType.BLOCK, NodeType.STEP)

    type: StepType = p_internal(30, default=StepType.BLANK)
    # custom type?
    name: str | None = p_regular(32, default=None, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.TEXT
    )
    code: Optional["Code"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.CODE
    )
    run: Optional["RunOptions"] = p_regular(
        36, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )
    connections: list[StepConnection] = p_regular(37, array=True, struct=StructType.STEP_CONNECTION)

    value_type: Optional["TypeInfo"] = p_regular(40, default=None, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(41)
    secret_value_packed: Any = p_secret_value_packed(42)
    value: Any = p_value_runtime(41, 42)
    node: Union["Block", "Step", None] = p_regular(
        43, require=False, references=(NodeType.BLOCK, NodeType.STEP)
    )
    condition: Optional["Expression"] = p_regular(
        46, require=False, array=False, default=None, struct=StructType.EXPRESSION
    )

    # flags
    is_template: bool = p_regular(60, default=False)

    steps: NodeList["Step"] = p_node_child(NodeType.STEP)
    fields: NodeList["Field"] = p_node_child(NodeType.FIELD)
    triggers: NodeList["Trigger"] = p_node_child(NodeType.TRIGGER)
    notices: NodeList["Issue"] = p_node_child(NodeType.ISSUE)
