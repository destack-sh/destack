from typing import TYPE_CHECKING, Any, Optional, Union, cast, final

from bench.language.const import BlockType, EnumType, FieldZone, NodeType, StructType, enum_
from bench.language.field import TypeInfoBase
from bench.language.graph import NodeList
from bench.language.node import (
    SourceNode,
    Struct,
    local_node_,
    object_component,
    struct_,
)
from bench.language.property import (
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import NAME_CONSTRAINT, constraint
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
        NodeReference,
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
    # boundary
    START = 1  # source with inputs (at most one per Flow)
    COMPLETE = 2  # terminate with outputs (at most one per Flow)
    FAIL = 3  # terminate with error
    VALUE = 10  # source with just(value)
    TRIGGER = 11  # source with just(trigger)

    # run
    PASS = 50  # noop, output = input
    BLOCK = 51  # run a runnable block
    CODE = 52  # run code
    TEXT = 53  # run text
    SEND = 54  # (signal/notification)
    # YIELD # to other program/human
    # APPLY
    # CREATE

    # control
    MATCH = 100  # X -> | n expressions | -> X' filtered output port (per expression)
    FILTER = 101  # X -> | X -> bool | -> X if true
    LOOP = 102  # X[] -> | X -> ... -> Y | -> Y[]
    MERGE = 103  # X1, X2, ... -> X
    FLATTEN = 105  # X[] -> X
    ACCUMULATE = 106  # X -> X[]
    REDUCE = 107  # X[] -> | X[] -> Y | -> Y
    ZIP = 108  # X1[], X2[], ... -> (X1, X2, ...)[]
    # WAIT/DELAY?
    # DEBOUNCE?
    # THROTTLE?
    # TELEPORT?

    # organize
    GROUP = 150

    ...


@enum_(EnumType.PIPE_TYPE)
class PipeType(IdEnum):
    THEN = 1  # trigger + data
    WITH = 2  # just data binding
    ...


@struct_(StructType.PIPE)
class Pipe(Struct):
    """A connection between two Steps in a FlowBlock."""

    type: PipeType = p_internal(30)
    source: "Step" = p_regular(31, require=True, references=(NodeType.STEP,))
    # ports/mapping/...?


@local_node_(NodeType.STEP, passthrough=("value", "fields"))
class Step(SourceNode[StepData]):
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
    value: Any = p_value_runtime(41, typ=lambda self: cast("Step", self).value_type)
    node: Union["Block", "Step", "Trigger", None] = p_regular(
        43, require=False, references=(NodeType.BLOCK, NodeType.STEP, NodeType.TRIGGER)
    )
    code: Optional["Code"] = p_regular(
        44, default=None, require=False, array=False, struct=StructType.CODE
    )
    condition: Optional["Expression"] = p_regular(
        45, require=False, array=False, default=None, struct=StructType.EXPRESSION
    )
    roles: list["Block"] = p_regular(
        46,
        require=False,
        array=True,
        references=NodeType.BLOCK,
        constraint=constraint(block_type=BlockType.ROLE),
    )
    identity: Optional["Block"] = p_regular(
        47,
        require=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_type=BlockType.IDENTITY),
    )
    policies: list["Policy"] = p_regular(48, require=False, array=True, struct=StructType.POLICY)
    if TYPE_CHECKING:
        node_ptr: Optional["NodeReference"] = None
        roles_ptr: tuple["NodeReference", ...] = ()
        identity_ptr: Optional["NodeReference"] = None

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

    @final
    def __repr__(self):  # type: ignore we want to override the default repr
        return f"<{self.type.bench_name}Step {self}>"

    @property
    def block(self) -> "Block | None":
        """Gets the containing ancestor Block (if any)"""
        from bench.language.block import Block

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Block):
                return parent
            parent = parent.parent
        return None

    def to_type(self, as_object: bool = True, zone: FieldZone | None = None):
        """Gets a type represented by this Step (if any)"""
        raise NotImplementedError

    @property
    def input_type(self) -> "TypeInfoBase":
        return self.to_type(as_object=True, zone=FieldZone.INPUT)

    @property
    def output_type(self) -> "TypeInfoBase":
        return self.to_type(as_object=True, zone=FieldZone.OUTPUT)

    @staticmethod
    def new(typ: StepType, name: str, **kwargs):
        """Creates a new Step of the given type."""
        return Step(type=typ, name=name, **kwargs)


#
# Custom step state :NodeInheritance
#


@object_component()
class StepState(Struct):  # :NodeInheritance
    """Builtin special Value as the state of some specific step type (in Step.value)."""

    pass
