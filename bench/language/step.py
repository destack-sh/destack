from typing import TYPE_CHECKING, Any, Optional, Union, assert_never, cast, final

from bench.language.const import (
    BlockType,
    EnumType,
    FieldZone,
    NodeType,
    StructType,
    TypeKind,
    enum_,
)
from bench.language.field import TypeInfoBase
from bench.language.graph import NodeList
from bench.language.node import SourceNode, Struct, local_node_, struct_
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
    # ABORT?
    VALUE = 10  # source with just(value)
    TRIGGER = 11  # source with just(trigger)

    # run
    PASS = 50  # noop, output = input
    BLOCK = 51  # run a runnable block
    CODE = 54  # run code
    TEXT = 55  # run text
    SEND = 56  # emit signal/notification
    # YIELD # to other program/human
    # APPLY
    # CREATE

    # control
    MATCH = 100  # X -> | n ports | -> X' filtered output port (per expression)
    FILTER = 101  # X -> | X -> bool | -> X if true
    MERGE = 102  # X1, X2, ... -> X
    FLATTEN = 103  # X[] -> X
    ACCUMULATE = 104  # X -> X[]
    REDUCE = 105  # X[] -> Y
    ZIP = 106  # X1[], X2[], ... -> (X1, X2, ...)[]
    # WAIT/DELAY?
    # DEBOUNCE?
    # THROTTLE?
    # TELEPORT?

    # group
    GROUP = 150  # no semantic meaning
    LOOP = 151  # loop inside: X[] -> | X -> ... -> Y | -> Y[]
    SHIELD = 152  # capture errors inside

    ...

    @property
    def is_boundary(self) -> bool:
        return self < 50

    @property
    def is_run(self) -> bool:
        return self >= 50 and self < 100

    @property
    def is_control(self) -> bool:
        return self >= 100 and self < 150

    @property
    def is_groupa(self) -> bool:
        return self >= 150 and self < 200


@enum_(EnumType.PIPE_TYPE)
class PipeType(IdEnum):
    THEN = 1  # content + trigger
    WITH = 2  # content (only)
    ...


@enum_(EnumType.PIPE_FILTER_TYPE)
class PipeFilterType(IdEnum):
    # truthy
    IS_NON_EMPTY = 1  # drop empty (None, empty, False, 0, ...)
    IS_TRUTHY = 2  # drop falsy (None, empty, False, 0, ...)
    # IS_VALID = 3  # keep valid (drop invalid)
    # falsy
    IS_EMPTY = 50  # keep empty only (None, empty, "", ...)
    IS_FALSY = 51  # keep falsy only (None, empty, False, 0, ...)
    # IS_INVALID doesn't make sense? (would need to know: valid for what target port?)


@struct_(StructType.PIPE)
class Pipe(Struct):
    """
    A connection between two Steps in a FlowBlock.
    The pipe is stored in the incoming Step, so the target Step is implicit.
    """

    # connection
    type: PipeType = p_internal(30)
    source: "Step" = p_regular(31, require=True, references=NodeType.STEP)
    source_port: "PortKey" = p_regular(32, require=True, struct=StructType.PORT_KEY)
    target: "Step" = p_regular(33, require=True, references=NodeType.STEP)
    target_port: "PortKey" = p_regular(34, require=True, struct=StructType.PORT_KEY)

    # filter/mapping/casting
    filter_type: PipeFilterType | None = p_regular(40, default=None)
    # filter_constraint, filter_condition, ...
    ...

    def __content_str__(self) -> str:
        arrow_str = ">" if self.type == PipeType.THEN else "->"
        source = self.source
        target = self.target
        return f"{source.absolute_path if source else '???'}:{self.source_port} {arrow_str} {target.absolute_path if target else '???'}:{self.target_port}"


@enum_(EnumType.PORT_TYPE)
class PortType(IdEnum):
    TRIGGER = 1  # trigger only (no data)
    DATA = 2  # trigger with full input/output value (depending on side)
    ERROR = 3  # trigger with error in case of failure (output only)
    FIELD = 5  # trigger with specific field (depending on side & type)


PORT_TYPES_BY_ZONE: dict[FieldZone, tuple[PortType, ...]] = {
    FieldZone.VARIABLE: (PortType.DATA, PortType.FIELD),
    FieldZone.INPUT: (PortType.TRIGGER, PortType.DATA, PortType.FIELD),
    FieldZone.OUTPUT: (PortType.TRIGGER, PortType.DATA, PortType.ERROR, PortType.FIELD),
}


@struct_(StructType.PORT_KEY)
class PortKey(Struct):
    """An identifier for a port on a Step."""

    # key
    type: PortType = p_internal(30)
    zone: FieldZone = p_regular(31)
    field: Optional["Field"] = p_regular(32, require=False, references=NodeType.FIELD)

    def __content_str__(self) -> str:
        if self.type == PortType.TRIGGER:
            return "?"
        elif self.type == PortType.DATA:
            return f"{self.zone.bench_name}"
        elif self.type == PortType.ERROR:
            return "!"
        elif self.type == PortType.FIELD:
            return f"{self.field.py_name if self.field else '???'}"
        else:
            assert_never(self.type)


PortIn = Union["Field", PortType, PortKey]


def to_port_key(port: PortIn, zone: FieldZone) -> PortKey:
    from bench.language.field import Field

    if isinstance(port, PortKey):
        if port.zone != zone:
            port = port.clone()
            port.zone = zone
        return port
    elif isinstance(port, Field):
        return PortKey(type=PortType.FIELD, zone=zone, field=port)
    elif isinstance(port, PortType):
        return PortKey(type=port, zone=zone)
    else:
        assert_never(port)


@struct_(StructType.PORT)
class Port(PortKey):
    """A full port on a Step with some value."""

    # value
    value_type: Optional["TypeInfo"] = p_regular(40, default=None, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(41)
    value: Any = p_value_runtime(41, typ=None)


@local_node_(NodeType.STEP, passthrough=("value", "fields"))
class Step(SourceNode[StepData]):
    """
    An data or control flow node in a FlowBlock. Ports on Steps are connected by Pipes.
    Pipes are stored in the source Step. Ports are implicit via Pipes unless tied to some value.
    A Step is run when it is triggered, specifically:
     - When its control port fires OR
     - When all its input ports (for all fields or full value) fire
    A Step may run multiple times if it is triggered multiple times (even concurrently).
    A Step may directly trigger any Step (including itself) at most once per run.
    Steps are run in order of definition per firing (regardless of pipe & port order).

    When a Step completes, then:
     1. Fire output values to all output ports
     2. Fire output control port
    When a Step fails, then:
     1. FIre error on error port
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
    pipes: list[Pipe] = p_regular(37, array=True, struct=StructType.PIPE)

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
    # is_test? (for testing)
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

    def connect(
        self,
        type: PipeType,
        source: "Step",
        *,
        source_port: "PortIn" = PortType.DATA,
        target_port: "PortIn" = PortType.DATA,
        filter_type: PipeFilterType | None = None,
    ) -> "Pipe":
        """Connects a source Step to this Step."""
        pipe = Pipe(
            type=type,
            source=source,
            source_port=to_port_key(source_port, FieldZone.OUTPUT),
            target=self,
            target_port=to_port_key(target_port, FieldZone.INPUT),
            filter_type=filter_type,
        )
        source.pipes.append(pipe)
        return pipe

    def then(
        self,
        target: "Step",
        *,
        source_port: "PortIn" = PortType.DATA,
        target_port: "PortIn" = PortType.DATA,
        filter_type: PipeFilterType | None = None,
    ) -> "Step":
        """Connects a source Step to this Step as a Then. Returns the target Step (for chaining)."""
        _ = target.connect(
            type=PipeType.THEN,
            source=self,
            source_port=source_port,
            target_port=target_port,
            filter_type=filter_type,
        )
        return target

    def with_(
        self,
        target: "Step",
        *,
        source_port: "PortIn" = PortType.DATA,
        target_port: "PortIn" = PortType.DATA,
        filter_type: PipeFilterType | None = None,
    ) -> "Step":
        """Connects a source Step to this Step as a With. Returns the target Step (for chaining)."""
        _ = target.connect(
            type=PipeType.WITH,
            source=self,
            source_port=source_port,
            target_port=target_port,
            filter_type=filter_type,
        )
        return target

    def to_type(self, as_object: bool = True, zone: FieldZone | None = None):
        """Gets a type represented by this Step (if any)"""
        from bench.language.block import Block
        from bench.language.field import TypeInfo

        if self.type == StepType.START:
            assert self.parent is not None, f"{self!r} has no parent"
            return self.parent.input_type
        elif self.type == StepType.COMPLETE:
            assert self.parent is not None, f"{self!r} has no parent"
            return self.parent.output_type
        elif self.type == StepType.BLOCK:
            assert isinstance(self.node, Block), f"{self!r} has no block: {self.node!r}"
            return self.node.to_type(as_object=as_object, zone=zone)
        else:
            if not as_object:
                typ = TypeInfo(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RUN)
            else:
                typ = TypeInfo(kind=TypeKind.OBJECT, base_type=self, base_field_zone=zone)
            typ._resolve_type()  # auto resolve type
            return typ

    @property
    def input_type(self) -> "TypeInfoBase":
        return self.to_type(as_object=True, zone=FieldZone.INPUT)

    @property
    def output_type(self) -> "TypeInfoBase":
        return self.to_type(as_object=True, zone=FieldZone.OUTPUT)

    @property
    def incoming_ports(self) -> list[PortKey]:
        ports: list[PortKey] = []  # no dynamic ports yet :StaticSteps
        for field in self.input_type._fields:
            port = PortKey(type=PortType.FIELD, zone=FieldZone.INPUT, field=field)
            ports.append(port)
        return ports

    @property
    def outgoing_ports(self) -> list[PortKey]:
        ports: list[PortKey] = []  # no dynamic ports yet :StaticSteps
        for field in self.output_type._fields:
            port = PortKey(type=PortType.FIELD, zone=FieldZone.OUTPUT, field=field)
            ports.append(port)
        return ports

    @staticmethod
    def new(typ: StepType, name: str, **kwargs):
        """Creates a new Step of the given type."""
        return Step(type=typ, name=name, **kwargs)
