from typing import TYPE_CHECKING, Any, Optional, Union, cast, final

from bench.language.const import EnumType, FieldZone, NodeType, StructType, enum_
from bench.language.graph import NodeList
from bench.language.issue import Issue
from bench.language.node import (
    InlineStruct,
    SourceNode,
    Struct,
    local_node,
    object_component,
    struct_,
)
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
        Box,
        Code,
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
    # source/sinks
    START = 1  # source with inputs (at most one per Flow)
    COMPLETE = 2  # terminate with outputs (at most one per Flow)
    VALUE = 3  # source with just(value)
    TRIGGER = 4  # source with just(trigger)

    # run
    RUN = 20  # (block)
    CODE = 21
    TEXT = 22
    SEND = 23  # (signal/notification)
    # YIELD/SUSPEND
    # APPLY
    # CREATE
    PASS = 30  # noop, output = input

    # control
    MATCH = 40  #
    FILTER = 41  # X -> | X -> X | None | -> X
    LOOP = 42  # X[] -> | X -> ... -> Y | -> Y[]
    MERGE = 43  # X1, X2, ... -> X
    SPLIT = 44  # X -> X1, X2, ...
    FLATTEN = 45  # X[] -> X
    ACCUMULATE = 46  # X -> X[]
    REDUCE = 47  # X[] -> Y
    ZIP = 48  # X1[], X2[], ... -> (X1, X2, ...)[]
    # WAIT/DELAY?
    # DEBOUNCE?
    # THROTTLE?
    # TELEPORT?

    # organize
    GROUP = 60

    ...


@enum_(EnumType.PIPE_TYPE)
class PipeType(IdEnum):
    THEN = 1
    ...


@struct_(StructType.PIPE)
class Pipe(Struct):
    """A connection between two Steps in a FlowBlock."""

    type: PipeType = p_internal(30)
    source: "Step" = p_regular(31, require=True, references=(NodeType.STEP,))


@local_node(NodeType.STEP)
class Step(SourceNode[StepData], HasValues):
    """
    An data or control flow node in a FlowBlock.
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
    incoming_pipes: list[Pipe] = p_regular(37, array=True, struct=StructType.PIPE)

    # content
    value_type: Optional["TypeInfo"] = p_regular(40, default=None, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(41)
    secret_value_packed: Any = p_secret_value_packed(42)
    value: Any = p_value_runtime(41, 42, typ=lambda self: cast("Step", self).value_type)
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

    # layout/style ('mini-view')
    position: Optional["Offset"] = p_regular(
        60, default=None, require=False, array=False, struct=StructType.OFFSET
    )
    size: Optional["Box"] = p_regular(
        61, default=None, require=False, array=False, struct=StructType.BOX
    )

    # flags
    ...

    steps: NodeList["Step"] = p_node_children(NodeType.STEP)
    fields: NodeList["Field"] = p_node_children(NodeType.FIELD)
    issues: NodeList["Issue"] = p_node_children(NodeType.ISSUE)

    @final
    def __repr__(self):  # type: ignore we want to override the default repr
        return f"<{self.type.bench_name}Step {self}>"

    def to_type(self, as_object: bool = True, zone: FieldZone | None = None):
        """Gets a type represented by this Step (if any)"""
        raise NotImplementedError


#
# Custom view states
#


@object_component()
class StepState(InlineStruct):
    """Builtin special Value as the state of some specific step type (in Step.value)."""

    pass
