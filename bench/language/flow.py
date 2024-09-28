from datetime import timedelta
from typing import TYPE_CHECKING, Any, Optional, Union, assert_never, cast, final
from uuid import UUID

import cachetools

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
from bench.language.node import BuiltinObject, SourceNode, Struct, local_node_, object_, struct_
from bench.language.property import (
    Property,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import NAME_CONSTRAINT, ValidationHandler, constraint
from bench.proto.wire import PipeData, StepData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import (
        Block,
        Box,
        Code,
        Color,
        Expression,
        Field,
        Icon,
        Line,
        NodeReference,
        RunOptions,
        Text,
        Trigger,
        TypeConstraint,
        TypeInfo,
        Vector2,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PORT_SIDE)
class PortSide(IdEnum):
    INCOMING = 1
    OUTGOING = 2


@enum_(EnumType.PORT_TYPE)
class PortType(IdEnum):
    RUN = 1  # fire only (no content, just the Run)
    FIELD = 11  # fire with specific field (depending on side & type)

    @property
    def is_incoming(self) -> bool:
        return self in PORT_TYPES_BY_SIDE[PortSide.INCOMING]

    @property
    def is_outgoing(self) -> bool:
        return self in PORT_TYPES_BY_SIDE[PortSide.OUTGOING]


PORT_TYPES_BY_SIDE: dict[PortSide, tuple[PortType, ...]] = {
    PortSide.INCOMING: (PortType.RUN, PortType.FIELD),
    PortSide.OUTGOING: (PortType.RUN, PortType.FIELD),
}
FIELD_ZONES_BY_SIDE: dict[PortSide, tuple[FieldZone, ...]] = {
    PortSide.INCOMING: (FieldZone.VARIABLE, FieldZone.INPUT),
    PortSide.OUTGOING: (FieldZone.OUTPUT,),
}


@object_()
class PortKeyBase(BuiltinObject):
    """An identifier for a port on a Step."""

    # key
    type: PortType = p_internal(40)
    side: PortSide = p_regular(41)
    field: Optional["Field"] = p_regular(42, require=False, references=NodeType.FIELD)
    if TYPE_CHECKING:
        field_id: Optional[UUID] = None
        field_ck: Optional[UUID] = None
        field_ptr: Optional["Property"] = None

    def __content_str__(self) -> str:
        if self.type == PortType.RUN:
            return f"[{self.type.bench_name}]"
        elif self.type == PortType.FIELD:
            field = self.field
            return f".{field.code_name if field else '???'}"
        else:
            assert_never(self.type)


@struct_(StructType.PORT_KEY)
class PortKey(Struct, PortKeyBase):
    """An identifier for a port on a Step."""

    # redirect so we get PortKeyBase.__content_str__ (not Struct.__content_str__)
    __content_str__ = PortKeyBase.__content_str__  # type: ignore


PortIn = Union["Field", PortType, PortKey]


def to_port_key(port: PortIn, *, side: PortSide) -> PortKey:
    from bench.language.field import Field

    if isinstance(port, PortKey):
        if port.side != side:
            port = port.clone()
            port.side = side
        return port
    elif isinstance(port, Field):
        # NOTE :Cleanup :Architecture: we would like to check field zone / port sides here
        #  but we sometimes use input fields as outputs (e.g. Flow inputs as Start step outputs)
        # if port.zone not in FIELD_ZONES_BY_SIDE[side]:
        #     raise ValueError(f"field {port!r} not allowed on side {side.bench_name}")
        return PortKey(type=PortType.FIELD, side=side, field=port)
    elif isinstance(port, PortType):
        if port not in PORT_TYPES_BY_SIDE[side]:
            raise ValueError(f"port {port.bench_name} not allowed on side {side.bench_name}")
        return PortKey(type=port, side=side)
    else:
        assert_never(port)


@enum_(EnumType.PIPE_TYPE)
class PipeType(IdEnum):
    CONTROL_AND_DATA = 1  # then: value + fire
    DATA = 2  # with: value


@enum_(EnumType.PIPE_FILTER)
class PipeFilter(IdEnum):
    # positive
    IS_NON_EMPTY = 1  # drop empty (None, empty, False, 0, ...)
    IS_TRUTHY = 2  # drop falsy (None, empty, False, 0, ...)
    # IS_VALID = 3  # keep valid (drop invalid)
    # negative
    IS_EMPTY = 10  # keep empty only (None, empty, "", ...)
    IS_FALSY = 11  # keep falsy only (None, empty, False, 0, ...)
    # IS_INVALID doesn't make sense? (would need to know: valid for what target port?)
    # run
    HAS_ERROR = 20  # keep if run failed


@enum_(EnumType.PIPE_MAPPING)
class PipeMapping(IdEnum):
    AUTO = 1


@enum_(EnumType.PIPE_MODULATION)
class PipeModulation(IdEnum):
    FLATTEN = 10
    ACCUMULATE = 11
    WINDOW = 12
    DEBOUNCE = 20
    THROTTLE = 21


@enum_(EnumType.PIPE_COMBINATOR)
class PipeCombinator(IdEnum):
    ZIP = 1
    PRODUCT = 2


@local_node_(NodeType.PIPE)
class Pipe(SourceNode[PipeData]):
    """
    A connection between two Steps in a Flow (source = outgoing, target = incoming).
    Pipes are stored in the containing Flow or containing Step.
    """

    parent: Union["Block", "Step", None] = p_node_parent(4, NodeType.BLOCK, NodeType.STEP)

    # connection
    type: PipeType = p_internal(30)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    source: "Step" = p_regular(35, require=True, references=NodeType.STEP)
    source_port: "PortKey" = p_regular(36, require=True, struct=StructType.PORT_KEY)
    target: "Step" = p_regular(37, require=True, references=NodeType.STEP)
    target_port: "PortKey" = p_regular(38, require=True, struct=StructType.PORT_KEY)
    if TYPE_CHECKING:
        source_ptr: Optional[NodeReference] = None
        target_ptr: Optional[NodeReference] = None

    # filter (on source side)
    filter: PipeFilter | None = p_regular(50, default=None)
    constraint: Optional["TypeConstraint"] = p_regular(
        51, default=None, array=False, struct=StructType.TYPE_CONSTRAINT
    )
    condition: Optional["Expression"] = p_regular(
        52, default=None, array=False, struct=StructType.EXPRESSION
    )

    # mapping
    mapping: PipeMapping | None = p_regular(60, default=None)
    modulation: PipeModulation | None = p_regular(61, default=None)
    combinator: PipeCombinator | None = p_regular(62, default=None)
    delay: Optional[timedelta] = p_regular(65, default=None)
    size: Optional[int] = p_regular(66, default=None)

    # view
    line: Optional["Line"] = p_regular(
        80,
        default=None,
        require=False,
        array=False,
        struct=StructType.LINE,
        description="Line points to cover for the path.",
    )
    color: Optional["Color"] = p_regular(
        81, default=None, require=False, array=False, struct=StructType.COLOR
    )
    is_hidden: bool = p_regular(82, default=False)

    def __content_str__(self) -> str:
        if self.type == PipeType.CONTROL_AND_DATA:
            arrow_str = "->"
        elif self.type == PipeType.DATA:
            arrow_str = "-"
        else:
            assert_never(self.type)
        if self.filter:
            arrow_str += f"?[{self.filter.bench_name}]"
        source = self.source
        target = self.target
        return f"{source.absolute_path if source else '???'}:{self.source_port} {arrow_str} {target.absolute_path if target else '???'}:{self.target_port}"

    def _validate_component(self, properties: tuple[Property, ...], invalid: "ValidationHandler"):
        # source and target must be distinct
        if (
            self.source == self.target
            and self.source_port.field_ptr is not None
            and self.source_port.field_ck == self.target_port.field_ck
        ):
            invalid(self, "source and target must be distinct", (Pipe.source, Pipe.target))

    @staticmethod
    def new(type: PipeType, name: str, **kwargs) -> "Pipe":
        return Pipe(type=type, name=name, **kwargs)


@enum_(EnumType.STEP_TYPE)
class StepType(IdEnum):
    # boundary (only incoming OR outgoing)
    START = 1  # source with inputs
    COMPLETE = 2  # terminate with outputs
    FAIL = 3  # terminate with error
    TRIGGER = 4  # source with just(trigger.value)
    # ABORT?

    # run
    BLOCK = 51  # run a runnable block
    TEXT = 54  # run text
    CODE = 55  # run code
    SEND = 56  # send a message/signal/notification/...
    # YIELD # to other program/human
    # SEND, APPLY, CREATE, PASS?

    # state
    VALUE = 100  # read (and write?) value

    # containers
    GROUP = 500  # sub-flow
    # LOOP, REPEAT, ...?

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
    def is_container(self) -> bool:
        return self >= 150 and self < 200


@local_node_(NodeType.STEP, passthrough=("value", "fields"))
class Step(SourceNode[StepData]):
    """
    A data or control flow node in a Flow. Ports on Steps are connected by Pipes.
    Pipes are stored in the containing Flow or containing Step.
    Ports are implicit via Pipes unless extra configuration is provided.

    A Step may run multiple times if it is fired multiple times (even concurrently).
    When a Step completes, then:
     1. Fire output values to all output ports
     2. Fire output control port
    When a Step fails, then:
     - If run_options.suppress_failure: fire error on run port
     - Else: fail containing Flow
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

    # content
    value_type: Optional["TypeInfo"] = p_regular(40, default=None, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(41)
    value: Any = p_value_runtime(41, typ=lambda self: cast("Step", self).value_type)
    node: Union["Block", "Trigger", None] = p_regular(
        43,
        require=False,
        references=(NodeType.BLOCK, NodeType.TRIGGER),
        constraint=constraint(block_types=[BlockType.CODE, BlockType.FLOW]),
    )
    code: Optional["Code"] = p_regular(
        45, default=None, require=False, array=False, struct=StructType.CODE
    )
    roles: list["Block"] = p_regular(
        46,
        require=False,
        array=True,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.ROLE]),
    )
    identity: Optional["Block"] = p_regular(
        47,
        require=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.IDENTITY]),
    )
    if TYPE_CHECKING:
        node_ptr: Optional["NodeReference"] = None
        roles_ptr: tuple["NodeReference", ...] = ()
        identity_ptr: Optional["NodeReference"] = None

    # view
    position: Optional["Vector2"] = p_regular(
        80, default=None, require=False, array=False, struct=StructType.VECTOR2
    )
    size: Optional["Box"] = p_regular(
        81, default=None, require=False, array=False, struct=StructType.BOX
    )

    steps: NodeList["Step"] = p_node_children(NodeType.STEP)
    pipes: NodeList["Pipe"] = p_node_children(NodeType.PIPE)
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
        name: str | None = None,
        source_port: "PortIn" = PortType.RUN,
        target_port: "PortIn" = PortType.RUN,
        filter: PipeFilter | None = None,
        constraint: "TypeConstraint | None" = None,
        condition: "Expression | None" = None,
        mapping: PipeMapping | None = None,
        modulation: PipeModulation | None = None,
        delay: timedelta | None = None,
        size: int | None = None,
        parent: Union["Block", "Step", None] = None,
    ) -> "Pipe":
        """Connects a source Step to this Step."""
        parent = parent or self.parent
        assert parent is not None, f"{self!r} is not attached to a parent"

        # assign next name like Pipe1, .. in parent :AutoNaming
        if name is None:
            siblings = parent.pipes.tolist()
            count = len(siblings) + 1
            name = f"{self.type.bench_name}{count}"
            while any(p.name == name for p in siblings):
                count += 1
                name = f"{self.type.bench_name}{count}"

        pipe = Pipe(
            type=type,
            name=name,
            source=source,
            source_port=to_port_key(source_port, side=PortSide.OUTGOING),
            target=self,
            target_port=to_port_key(target_port, side=PortSide.INCOMING),
            filter=filter,
            constraint=constraint,
            condition=condition,
            mapping=mapping,
            modulation=modulation,
            delay=delay,
            size=size,
            parent=parent,
        )
        parent.pipes.append(pipe)
        return pipe

    def then(
        self,
        target: "Step",
        *,
        name: str | None = None,
        source_port: "PortIn" = PortType.RUN,
        target_port: "PortIn" = PortType.RUN,
        filter: PipeFilter | None = None,
        constraint: "TypeConstraint | None" = None,
        condition: "Expression | None" = None,
        mapping: PipeMapping | None = None,
        modulation: PipeModulation | None = None,
        delay: timedelta | None = None,
        size: int | None = None,
        parent: Union["Block", "Step", None] = None,
    ) -> "Step":
        """Connects a source Step to this Step as a Then. Returns the target Step (for chaining)."""
        _ = target.connect(
            type=PipeType.CONTROL_AND_DATA,
            name=name,
            source=self,
            source_port=source_port,
            target_port=target_port,
            filter=filter,
            constraint=constraint,
            condition=condition,
            mapping=mapping,
            modulation=modulation,
            delay=delay,
            size=size,
            parent=parent,
        )
        return target

    def with_(
        self,
        target: "Step",
        *,
        name: str | None = None,
        source_port: "PortIn" = PortType.RUN,
        target_port: "PortIn" = PortType.RUN,
        filter: PipeFilter | None = None,
        constraint: "TypeConstraint | None" = None,
        condition: "Expression | None" = None,
        mapping: PipeMapping | None = None,
        modulation: PipeModulation | None = None,
        delay: timedelta | None = None,
        size: int | None = None,
        parent: Union["Block", "Step", None] = None,
    ) -> "Step":
        """Connects a source Step to this Step as a With. Returns the target Step (for chaining)."""
        _ = target.connect(
            type=PipeType.DATA,
            name=name,
            source=self,
            source_port=source_port,
            target_port=target_port,
            filter=filter,
            constraint=constraint,
            condition=condition,
            mapping=mapping,
            modulation=modulation,
            delay=delay,
            size=size,
            parent=parent,
        )
        return target

    @cachetools.cached({})  # :CachedTypeInfo
    def to_type(
        self, as_object: bool = True, zone: FieldZone | None = None
    ) -> "TypeInfoBase | None":
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
    def variable_type(self) -> "TypeInfoBase | None":
        return self.to_type(as_object=True, zone=FieldZone.VARIABLE)

    @property
    def input_type(self) -> "TypeInfoBase | None":
        return self.to_type(as_object=True, zone=FieldZone.INPUT)

    @property
    def output_type(self) -> "TypeInfoBase | None":
        return self.to_type(as_object=True, zone=FieldZone.OUTPUT)

    @property
    def incoming_ports(self) -> list[PortKey]:
        ports: list[PortKey] = []  # no dynamic ports yet :StaticSteps
        input_type = self.input_type
        if input_type is not None:
            for field in input_type._fields:
                port = PortKey(type=PortType.FIELD, side=PortSide.INCOMING, field=field)
                ports.append(port)
        return ports

    @property
    def outgoing_ports(self) -> list[PortKey]:
        ports: list[PortKey] = []  # no dynamic ports yet :StaticSteps
        output_type = self.output_type
        if output_type is not None:
            for field in output_type._fields:
                port = PortKey(type=PortType.FIELD, side=PortSide.OUTGOING, field=field)
                ports.append(port)
        return ports

    @staticmethod
    def new(typ: StepType, name: str, **kwargs):
        """Creates a new Step of the given type."""
        return Step(type=typ, name=name, **kwargs)
