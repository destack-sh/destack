from datetime import timedelta
from typing import TYPE_CHECKING, Optional, Type, Union, cast
from uuid import UUID

import cachetools

from bench.language.action import ActionBase
from bench.language.const import (
    BlockType,
    EnumType,
    FieldType,
    NodeType,
    StructType,
    TypeKind,
    enum_,
)
from bench.language.field import TypeBase
from bench.language.list import LocalNodeList
from bench.language.node import (
    NodeSubtypeStub,
    SourceNode,
    Struct,
    local_node_,
    node_subtype_,
    struct_,
)
from bench.language.property import (
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.language.run import RunKind, RunOptions
from bench.language.validation import (
    NAME_CONSTRAINT,
    constraint,
)
from bench.proto.wire import PipeData, StepData
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
        NodeReference,
        Rectangle,
        Text,
        TypeConstraint,
        Vector2,
    )

# pyright: reportIncompatibleVariableOverride=false


@struct_(StructType.OBJECT_MAPPING)
class ObjectMapping(Struct):
    field_mappings: list["FieldMapping"] = p_regular(
        40, require=True, array=True, struct=StructType.FIELD_MAPPING
    )


@struct_(StructType.FIELD_MAPPING)
class FieldMapping(Struct):
    source: "Field" = p_regular(35, require=True, references=NodeType.FIELD)
    target: "Field" = p_regular(36, require=True, references=NodeType.FIELD)


@enum_(EnumType.PORT_SIDE)
class PortSide(IdEnum):
    INCOMING = 1
    OUTGOING = 2


FIELD_ZONES_BY_SIDE: dict[PortSide, tuple[FieldType, ...]] = {
    PortSide.INCOMING: (FieldType.VARIABLE, FieldType.INPUT),
    PortSide.OUTGOING: (FieldType.OUTPUT,),
}


@enum_(EnumType.PIPE_TYPE)
class PipeType(IdEnum):
    PASS = 1
    SELECT = 2
    OPTION = 3
    STREAM = 50


SIGN_BY_PIPE_TYPE: dict[PipeType, str] = {
    PipeType.PASS: "->",
    PipeType.SELECT: "-?>",
    PipeType.OPTION: "-o>",
    PipeType.STREAM: "-=-",
}
PIPE_TYPES_BY_SIGN: dict[str, PipeType] = {v: k for k, v in SIGN_BY_PIPE_TYPE.items()}


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
    target: "Step" = p_regular(36, require=True, references=NodeType.STEP)
    if TYPE_CHECKING:
        source_ptr: Optional[NodeReference] = None
        source_id: Optional[UUID] = None
        source_ck: Optional[str] = None
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None
        target_ck: Optional[str] = None

    # associated_fields, ...?

    # run
    run_options: Optional["RunOptions"] = p_regular(
        50, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )

    # filter
    condition: Optional["Expression"] = p_regular(
        60, default=None, require=False, array=False, struct=StructType.EXPRESSION
    )
    condition_code: Optional["Code"] = p_regular(
        61, default=None, require=False, array=False, struct=StructType.CODE
    )
    constraint: Optional["TypeConstraint"] = p_regular(
        62, default=None, array=False, struct=StructType.TYPE_CONSTRAINT
    )

    # mapping
    mapping: Optional["ObjectMapping"] = p_regular(
        70,
        default=None,
        require=False,
        array=False,
        struct=StructType.OBJECT_MAPPING,
        description="Mapping for inputs from source node into target node.",
    )
    mapping_code: Optional["Code"] = p_regular(
        71,
        default=None,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="Mapping for outputs from source node into target node. Takes precedence over mapping.",
    )

    # modulation
    delay: Optional[timedelta] = p_regular(80, default=None)

    # view
    color: Optional["Color"] = p_regular(
        91, default=None, require=False, array=False, struct=StructType.COLOR
    )
    is_hidden: bool = p_regular(92, default=False)

    def __content_str__(self) -> str:
        sign = SIGN_BY_PIPE_TYPE.get(self.type, "???")
        source = self.source
        target = self.target
        return f"{source.absolute_path if source else '???'} {sign} {target.absolute_path if target else '???'}"

    @property
    def run_kind(self) -> RunKind:
        return RunKind.PIPE

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

    @property
    def input_type(self) -> "TypeBase | None":
        source = self.source
        return source.output_type if source is not None else None

    @property
    def output_type(self) -> "TypeBase | None":
        target = self.target
        return target.input_type if target is not None else None

    @staticmethod
    def new(type: PipeType, name: str, **kwargs) -> "Pipe":
        return Pipe(type=type, name=name, **kwargs)


@node_subtype_(PipeType.SELECT)
class SelectPipe(Pipe):
    text: Optional["Text"] = p_regular(
        100, default=None, require=False, array=False, struct=StructType.TEXT
    )


@enum_(EnumType.STEP_TYPE)
class StepType(IdEnum):
    # boundary (may be only incoming or outgoing)
    START = 1  # source with inputs
    COMPLETE = 2  # terminate with outputs
    FAIL = 3  # terminate with error
    TRIGGER = 4  # source or intermediary
    # ABORT?

    # run
    ACTION = 60
    SEND = 61  # emit a message
    YIELD = 62  # to other program/human

    # state
    # ...

    # containers
    # GROUP = 500  # subflow region
    LOOP = 501  # repeat

    # misc
    TEXT = 900  # no-op, just for documentation

    @property
    def is_boundary(self) -> bool:
        return self < 50

    @property
    def is_run(self) -> bool:
        return self >= 50 and self < 100

    @property
    def is_container(self) -> bool:
        return self >= 500 and self < 600


@local_node_(NodeType.STEP, passthrough_get=("value", "fields"))
class Step(SourceNode[StepData]):
    """
    A data or control flow node in a Flow. Steps are connected by Pipes.
    """

    parent: Union["Block", "Step", None] = p_node_parent(4, NodeType.BLOCK, NodeType.STEP)

    # common
    type: StepType = p_internal(30)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.ICON
    )
    text: Optional["Text"] = p_regular(
        36, default=None, require=False, array=False, struct=StructType.TEXT
    )

    # content
    run_options: Optional["RunOptions"] = p_regular(
        41, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )
    roles: list["Block"] = p_regular(
        42,
        require=False,
        array=True,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.ROLE]),
    )
    identity: Optional["Block"] = p_regular(
        43,
        require=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.IDENTITY]),
    )
    if TYPE_CHECKING:
        roles_ptr: tuple["NodeReference", ...] = ()
        identity_ptr: Optional["NodeReference"] = None

    # view
    position: Optional["Vector2"] = p_regular(
        80, default=None, require=False, array=False, struct=StructType.VECTOR2
    )
    size: Optional["Rectangle"] = p_regular(
        81, default=None, require=False, array=False, struct=StructType.RECTANGLE
    )

    steps: LocalNodeList["Step"] = p_node_children(NodeType.STEP)
    pipes: LocalNodeList["Pipe"] = p_node_children(NodeType.PIPE)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)

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

    @property
    def run_kind(self) -> RunKind:
        return RunKind.STEP

    def connect(
        self,
        type: PipeType,
        target: "Step",
        *,
        name: str | None = None,
        parent: Union["Block", "Step", None] = None,
        run_options: RunOptions | None = None,
    ) -> "Pipe":
        """Connects a target Step to this Step."""
        from bench.language.block import Block

        parent = parent or self.parent
        assert parent is not None, f"{self!r} is not attached to a parent"
        assert isinstance(parent, (Step, Block)), f"{parent!r} is not valid for {self!r}"

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
            source=self,
            target=target,
            parent=parent,
            run_options=run_options,
        )
        parent.pipes.append(pipe)
        return pipe

    @cachetools.cached({})  # :CachedTypeInfo
    def to_type(
        self, as_object: bool = True, field_type: FieldType | None = None
    ) -> "TypeBase | None":
        """Gets a type represented by this Step (if any)"""
        from bench.language.field import TypeInfo

        if self.type == StepType.START:
            parent = self.parent
            return parent.input_type if parent is not None else None
        elif self.type == StepType.COMPLETE:
            parent = self.parent
            return parent.output_type if parent is not None else None
        elif self.type == StepType.ACTION and cast(ActionStep, self).delegate_ptr:
            delegate = cast(ActionStep, self).delegate
            return (
                delegate.to_type(as_object=as_object, field_type=field_type) if delegate else None
            )
        else:
            if not as_object:
                typ = TypeInfo(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RUN)
            else:
                assert field_type is not None, f"missing field_type for object {self!r}"
                typ = TypeInfo(kind=TypeKind.OBJECT, base_type=self, base_field_type=field_type)
            return typ

    @property
    def variable_type(self) -> "TypeBase | None":
        return self.to_type(as_object=True, field_type=FieldType.VARIABLE)

    @property
    def input_type(self) -> "TypeBase | None":
        return self.to_type(as_object=True, field_type=FieldType.INPUT)

    @property
    def output_type(self) -> "TypeBase | None":
        return self.to_type(as_object=True, field_type=FieldType.OUTPUT)

    @staticmethod
    def new[StepT: Step = Step](
        typ: StepType | Type[StepT] | NodeSubtypeStub[StepT], name: str, **kwargs
    ) -> StepT:
        """Creates a new Step of the given type."""
        if isinstance(typ, type):
            typ = Step.__subtype_by_subclass__[typ]  # type: ignore
        elif isinstance(typ, NodeSubtypeStub):
            typ = cast(StepType, typ._node_subtype)
        return Step(type=typ, name=name, **kwargs)  # type: ignore


@node_subtype_(StepType.ACTION)
class ActionStep(Step, ActionBase):
    # ...ActionBase[100-129]
    pass


@node_subtype_(StepType.SEND)
class SendStep(Step):
    pass


@node_subtype_(StepType.LOOP)
class LoopStep(Step):
    for_field: Optional["Field"] = p_regular(
        100,
        require=False,
        array=False,
        references=NodeType.FIELD,
    )


@node_subtype_(StepType.TEXT)
class TextStep(Step):
    pass
