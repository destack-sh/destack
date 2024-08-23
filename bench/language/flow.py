from typing import TYPE_CHECKING, Any, Optional, Union, assert_never, cast, final
from uuid import UUID

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
from bench.proto.wire import PipeData, PortData, StepData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import (
        Block,
        Box,
        Code,
        Color,
        Field,
        Icon,
        Line,
        NodeReference,
        RunOptions,
        Text,
        Trigger,
        TypeInfo,
        Vector2,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PIPE_TYPE)
class PipeType(IdEnum):
    THEN = 1  # value + fire
    WITH = 2  # value
    ...


@enum_(EnumType.PIPE_FILTER_TYPE)
class PipeFilterType(IdEnum):
    # positive
    IS_NON_EMPTY = 1  # drop empty (None, empty, False, 0, ...)
    IS_TRUTHY = 2  # drop falsy (None, empty, False, 0, ...)
    # IS_VALID = 3  # keep valid (drop invalid)
    # negative
    IS_EMPTY = 50  # keep empty only (None, empty, "", ...)
    IS_FALSY = 51  # keep falsy only (None, empty, False, 0, ...)
    # IS_INVALID doesn't make sense? (would need to know: valid for what target port?)


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

    # filter
    filter_type: PipeFilterType | None = p_regular(50, default=None)
    # filter_constraint, filter_condition, ...
    ...

    # mapping/casting
    ...

    # view
    line: Optional["Line"] = p_regular(
        80,
        default=None,
        require=False,
        array=False,
        struct=StructType.LINE,
        description="Line points to cover.",
    )
    color: Optional["Color"] = p_regular(
        81, default=None, require=False, array=False, struct=StructType.COLOR
    )

    def __content_str__(self) -> str:
        if self.type == PipeType.THEN:
            arrow_str = "->"
        elif self.type == PipeType.WITH:
            arrow_str = "-"
        else:
            assert_never(self.type)
        if self.filter_type:
            arrow_str += f"?[{self.filter_type.bench_name}]"
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


@enum_(EnumType.PORT_SIDE)
class PortSide(IdEnum):
    INCOMING = 1
    OUTGOING = 2


@enum_(EnumType.PORT_TYPE)
class PortType(IdEnum):
    RUN = 1  # fire only (no content, just the Run)
    ERROR = 2  # fire with error in case of failure (output only)
    OBJECT = 10  # fire with full input/output/... value (depending on side)
    FIELD = 11  # fire with specific field (depending on side & type)

    @property
    def is_incoming(self) -> bool:
        return self in PORT_TYPES_BY_SIDE[PortSide.INCOMING]

    @property
    def is_outgoing(self) -> bool:
        return self in PORT_TYPES_BY_SIDE[PortSide.OUTGOING]


PORT_TYPES_BY_SIDE: dict[PortSide, tuple[PortType, ...]] = {
    PortSide.INCOMING: (PortType.RUN, PortType.OBJECT, PortType.FIELD),
    PortSide.OUTGOING: (PortType.RUN, PortType.ERROR, PortType.OBJECT, PortType.FIELD),
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
        if self.type == PortType.RUN or self.type == PortType.ERROR:
            return f"[{self.type.bench_name}]"
        elif self.type == PortType.OBJECT:
            return f".*[{self.side.bench_name}]"
        elif self.type == PortType.FIELD:
            field = self.field
            return f".{field.py_name if field else '???'}"
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


@local_node_(NodeType.PORT)
class Port(SourceNode[PortData], PortKeyBase):
    """
    Extra configuration for a port (key) on a Step with some value.
    Not all ports need a Port, just if there is extra behavior to define.
    """

    parent: Union["Step", None] = p_node_parent(4, NodeType.STEP)

    # content
    name: str = p_regular(30, default=None, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(31, default=INTEGER_ZERO)

    # type identity
    # ...TypeInfo[40-69]

    # static/initial value
    value_type: Optional["TypeInfo"] = p_regular(50, default=None, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(51)
    value: Any = p_value_runtime(51, typ=None)


@enum_(EnumType.STEP_TYPE)
class StepType(IdEnum):
    # boundary (only incoming OR outgoing)
    START = 1  # source with inputs
    COMPLETE = 2  # terminate with outputs
    FAIL = 3  # terminate with error
    # ABORT?
    VALUE = 10  # source with just(value)
    TRIGGER = 11  # source with just(trigger)

    # run
    BLOCK = 51  # run a runnable block
    TEXT = 54  # run text
    CODE = 55  # run code
    SEND = 56  # emit signal/notification
    # YIELD # to other program/human
    # APPLY, CREATE, PASS?

    # data
    # NOTE :Architecture: could the special control Steps be factored into general Port behaviors?
    #  (for instance, flatten/accumulate could be special incoming and outgoing port-side mappings;
    #   as opposed to pipe mappings which should probably be stateless)
    # MATCH = 100  # X -> | n ports | -> X' filtered output port (per expression)
    # FILTER = 101  # X -> | X -> bool | -> X if true
    # MERGE = 102  # X1, X2, ... -> X
    # FLATTEN = 103  # X[] -> X
    # ACCUMULATE = 104  # X -> X[]
    # REDUCE = 105  # X[] -> Y
    # ZIP = 106  # X1[], X2[], ... -> (X1, X2, ...)[]
    # JOIN = 107  # X1, X2, ... -> (X1, X2, ...)

    # control
    # WAIT/DELAY?, DEBOUNCE?, TELEPORT?, THROTTLE?

    # containers
    GROUP = 500  # sub-flow
    LOOP = 501  # loop inside: X[] -> | X -> ... -> Y | -> Y[]
    REPEAT = 502  # repeat X N times / until some condition
    # SHIELD?

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
     - If error port exists: fire error on error port
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
        source_port: "PortIn" = PortType.OBJECT,
        target_port: "PortIn" = PortType.OBJECT,
        filter_type: PipeFilterType | None = None,
    ) -> "Pipe":
        """Connects a source Step to this Step."""
        parent = self.parent
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
            filter_type=filter_type,
        )
        parent.pipes.append(pipe)
        return pipe

    def then(
        self,
        target: "Step",
        *,
        name: str | None = None,
        source_port: "PortIn" = PortType.OBJECT,
        target_port: "PortIn" = PortType.OBJECT,
        filter_type: PipeFilterType | None = None,
    ) -> "Step":
        """Connects a source Step to this Step as a Then. Returns the target Step (for chaining)."""
        _ = target.connect(
            type=PipeType.THEN,
            name=name,
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
        name: str | None = None,
        source_port: "PortIn" = PortType.OBJECT,
        target_port: "PortIn" = PortType.OBJECT,
        filter_type: PipeFilterType | None = None,
    ) -> "Step":
        """Connects a source Step to this Step as a With. Returns the target Step (for chaining)."""
        _ = target.connect(
            type=PipeType.WITH,
            name=name,
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
            port = PortKey(type=PortType.FIELD, side=PortSide.INCOMING, field=field)
            ports.append(port)
        return ports

    @property
    def outgoing_ports(self) -> list[PortKey]:
        ports: list[PortKey] = []  # no dynamic ports yet :StaticSteps
        for field in self.output_type._fields:
            port = PortKey(type=PortType.FIELD, side=PortSide.OUTGOING, field=field)
            ports.append(port)
        return ports

    @staticmethod
    def new(typ: StepType, name: str, **kwargs):
        """Creates a new Step of the given type."""
        return Step(type=typ, name=name, **kwargs)
