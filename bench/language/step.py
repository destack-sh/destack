from typing import TYPE_CHECKING, Any, Optional, Union, final

from bench.language.const import EnumType, FieldZone, NodeType, StructType, enum_
from bench.language.graph import NodeList
from bench.language.issue import Issue
from bench.language.node import SourceNode, Struct, node_, struct_
from bench.language.property import (
    p_internal,
    p_node_children,
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
    from bench.language import (
        Block,
        Code,
        Color,
        Expression,
        Field,
        Icon,
        Offset,
        Policy,
        RunOptions,
        Text,
        Trigger,
        TypeInfo,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.STEP_TYPE)
class StepType(IdEnum):
    # source
    START = 1
    VALUE = 2
    TRIGGER = 10
    RUN = 20
    RUN_DEFERRED = 21
    SEND = 22
    COMPLETE = 23
    BRANCH = 30
    FILTER = 31
    LOOP = 32
    GROUP = 50


@enum_(EnumType.STEP_CONNECTION_TYPE)
class StepConnectionType(IdEnum):
    THEN = 1
    # ... not sure yet


@struct_(StructType.STEP_CONNECTION)
class StepConnection(Struct):
    """A connection between two Steps in a FlowBlock."""

    type: StepConnectionType = p_internal(30)
    source: "Step" = p_regular(31, require=True, references=(NodeType.STEP,))


@node_(NodeType.STEP)
class Step(SourceNode[StepData], HasValues):
    """
    An data or control flow unit in a FlowBlock.
    """

    parent: Union["Block", "Step", None] = p_node_parent(4, NodeType.BLOCK, NodeType.STEP)

    # common
    type: StepType = p_internal(30)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.ICON
    )
    run_options: Optional["RunOptions"] = p_regular(
        36, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )
    connections: list[StepConnection] = p_regular(37, array=True, struct=StructType.STEP_CONNECTION)

    # content
    value_type: Optional["TypeInfo"] = p_regular(40, default=None, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(41)
    secret_value_packed: Any = p_secret_value_packed(42)
    value: Any = p_value_runtime(41, 42, typ=None)  # freely typed?
    node: Union["Block", "Step", "Trigger", None] = p_regular(
        43, require=False, references=(NodeType.BLOCK, NodeType.STEP, NodeType.TRIGGER)
    )
    code: Optional["Code"] = p_regular(
        44, default=None, require=False, array=False, struct=StructType.CODE
    )
    condition: Optional["Expression"] = p_regular(
        45, require=False, array=False, default=None, struct=StructType.EXPRESSION
    )
    roles: list["Block"] = p_regular(46, require=False, array=True, references=NodeType.BLOCK)
    identity: Optional["Block"] = p_regular(47, require=False, references=NodeType.BLOCK)
    policies: list["Policy"] = p_regular(48, require=False, array=True, struct=StructType.POLICY)

    # layout/style ('view')
    position: Optional["Offset"] = p_regular(
        50, default=None, require=False, array=False, struct=StructType.OFFSET
    )
    background_color: Optional["Color"] = p_regular(
        51, default=None, require=False, array=False, struct=StructType.COLOR
    )

    # flags
    is_template: bool = p_regular(60, default=False)

    steps: NodeList["Step"] = p_node_children(NodeType.STEP)
    fields: NodeList["Field"] = p_node_children(NodeType.FIELD)
    triggers: NodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    issues: NodeList["Issue"] = p_node_children(NodeType.ISSUE)

    @final
    def __repr__(self):  # type: ignore we want to override the default repr
        return f"<{self.type.bench_name}Step {self}>"

    def to_type(self, as_object: bool = True, zone: FieldZone | None = None):
        """Gets a type represented by this Step (if any)"""
        raise NotImplementedError
