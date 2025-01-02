from datetime import timedelta
from typing import TYPE_CHECKING, Any, Literal, Optional, Type, Union, assert_never, cast

import cachetools

from bench.language.const import (
    BlockType,
    EnumType,
    FieldType,
    NodeType,
    ObjectKind,
    RunType,
    StructType,
    TypeKind,
    enum_,
)
from bench.language.field import TypeBase, TypeConstraint, TypeInfo
from bench.language.list import LocalNodeList
from bench.language.node import (
    BuiltinObject,
    Node,
    NodeReference,
    NodeSubtypeStub,
    SourceNode,
    Struct,
    node_,
    node_subtype_,
    object_,
    struct_,
)
from bench.language.property import (
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.run import RunOptions
from bench.language.validation import (
    NAME_CONSTRAINT,
    constraint,
)
from bench.language.value import coerce_custom_object_scalar
from bench.proto.wire.lang_pb2 import ActionData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Block,
        Code,
        DomNode,
        Expression,
        Field,
        File,
        Icon,
        NodeReference,
        ObjectMapping,
        Pipe,
        PipeType,
        Text,
        Vector2,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.ACTION_TYPE)
class ActionType(IdEnum):
    # start
    START = 1  # source with inputs
    TRIGGER = 2  # source with trigger
    # end
    COMPLETE = 10  # terminate with outputs
    FAIL = 11  # terminate with error
    # ABORT?

    # read
    GET = 40
    SEARCH = 41
    # AGGREGATE?
    COPY = 50

    # write
    CREATE = 60
    DUPLICATE = 61
    UPDATE = 62
    DELETE = 63
    PASTE = 70

    # session
    YIELD = 80  # to something
    # PAUSE, RESUME, STOP, ...

    # static
    CODE = 100
    DELEGATE = 101
    WAIT = 102
    # dynamic
    GENERATE = 110
    TRANSFORM = 111
    EXTRACT = 112
    ROUTE = 120

    # state
    # ...

    # application
    OBSERVE = 1000
    CLICK = 1050
    PRESS = 1051
    TYPE = 1052
    SCROLL = 1053
    SELECT = 1054
    GO_BACKWARD = 1060
    GO_FORWARD = 1061

    # web
    GO_TO_URL = 1100
    GO_TO_TAB = 1101
    OPEN_TAB = 1102
    CLOSE_TAB = 1103

    # containers
    # GROUP = 500  # subflow region
    LOOP = 5001  # repeat

    # misc
    TEXT = 9000  # no-op, just for documentation

    @property
    def is_boundary(self) -> bool:
        return self < 40

    @property
    def is_container(self) -> bool:
        return self >= 5000 and self < 6000


@node_(NodeType.ACTION, passthrough_get=("value", "fields"))
class Action(SourceNode[ActionData]):
    """
    A data or control flow node in a Flow. Actions are connected by Pipes.
    """

    parent: Union["Block", "Action", None] = p_node_parent(4, NodeType.BLOCK, NodeType.ACTION)

    # common
    type: ActionType = p_system(30)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.ICON
    )
    text: Optional["Text"] = p_regular(
        36, default=None, require=False, array=False, struct=StructType.TEXT
    )

    # meta
    run_options: Optional["RunOptions"] = p_regular(
        41, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )
    roles: list["Block"] = p_regular(
        42,
        require=False,
        array=True,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.ROLE]),
    )
    identity: Optional["Block"] = p_regular(
        43,
        require=False,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.IDENTITY]),
    )
    if TYPE_CHECKING:
        roles_ptr: tuple["NodeReference", ...] = ()
        identity_ptr: Optional["NodeReference"] = None

    # content
    code: Optional["Code"] = p_regular(
        52,
        default=None,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="Current implementation code for this action.",
        field_type=FieldType.INPUT,
    )
    delegate: Union["Block", None] = p_regular(
        53,
        require=False,
        array=False,
        references=(NodeType.BLOCK,),
        constraint=constraint(node_subtypes=[BlockType.FLOW]),
        description="Current implementation for this action.",
        field_type=FieldType.INPUT,
    )
    if TYPE_CHECKING:
        delegate_ptr: "NodeReference | None" = None
    calls: list["Call"] = p_regular(
        55, array=True, struct=StructType.CALL, field_type=FieldType.OUTPUT
    )

    # flow
    position: Optional["Vector2"] = p_regular(
        80, default=None, require=False, array=False, struct=StructType.VECTOR2
    )

    actions: LocalNodeList["Action"] = p_node_children(NodeType.ACTION)
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
    def run_type(self) -> RunType:
        return RunType.ACTION

    def connect(
        self,
        type: "PipeType",
        target: "Action",
        *,
        name: str | None = None,
        parent: Union["Block", "Action", None] = None,
        run_options: RunOptions | None = None,
    ) -> "Pipe":
        """Connects a target Action to this Action."""
        from bench.language import Block, Pipe

        parent = parent or self.parent
        assert parent is not None, f"{self!r} is not attached to a parent"
        assert isinstance(parent, (Action, Block)), f"{parent!r} is not valid for {self!r}"

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
    def to_type_maybe(
        self,
        of: Literal["instance", "value"] = "instance",
        field_type: FieldType | None = None,
    ) -> "TypeBase | None":
        """Gets a type represented by this Action (if any)"""
        from bench.language import TypeInfo

        if of == "instance":
            typ = TypeInfo(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RUN)
            return typ
        elif of == "value":
            if self.type == ActionType.START:
                parent = self.parent
                return parent.input_type if parent is not None else None
            elif self.type == ActionType.COMPLETE:
                parent = self.parent
                return parent.output_type if parent is not None else None

            base = self
            if self.type == ActionType.DELEGATE and self.delegate_ptr is not None:
                base = self.delegate

            # actions also have their subtype as input & output type
            #  (to enable dynamically setting action properties as inputs)
            if field_type == FieldType.INPUT:
                typ = TypeInfo(
                    kind=TypeKind.PARTIAL_OBJECT,
                    base_type=base,
                    bench_type=NodeType.ACTION,
                    base_field_type=field_type,
                    property_field_type=field_type,
                    constraint=TypeConstraint(node_subtypes=[self.type]),
                )
            elif field_type == FieldType.OUTPUT:
                typ = TypeInfo(
                    kind=TypeKind.PARTIAL_OBJECT,
                    base_type=base,
                    base_field_type=field_type,
                    property_field_type=field_type,
                    constraint=TypeConstraint(node_subtypes=[self.type]),
                )
            else:
                typ = TypeInfo(
                    kind=TypeKind.CUSTOM_OBJECT, base_type=base, base_field_type=field_type
                )
            return typ
        else:
            assert_never(of)

    def to_type(
        self, *, of: Literal["instance", "value"] = "instance", field_type: FieldType | None = None
    ) -> "TypeBase":
        typ = self.to_type_maybe(of=of, field_type=field_type)
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @property
    def variable_type(self) -> "TypeBase | None":
        return None  # Actions don't have variables?

    @property
    def input_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_type=FieldType.INPUT)

    @property
    def output_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_type=FieldType.OUTPUT)

    @staticmethod
    def new[ActionT: "Action" = "Action"](
        typ: ActionType | Type[ActionT] | NodeSubtypeStub[ActionT], name: str, **kwargs
    ) -> ActionT:
        """Creates a new Action of the given type."""
        if isinstance(typ, type):
            typ = Action.__subtype_by_subclass__[typ]  # type: ignore
        elif isinstance(typ, NodeSubtypeStub):
            typ = cast(ActionType, typ._node_subtype)
        return Action(type=typ, name=name, **kwargs)  # type: ignore


#
# Read
#


@node_subtype_(ActionType.GET)
class GetAction(Action):
    """Get a single Node."""

    node_type: NodeType | None = p_regular(100, default=None, field_type=FieldType.INPUT)
    base_block: Optional["Block"] = p_regular(
        101,
        array=False,
        require=False,
        default=None,
        references=NodeType.BLOCK,
        field_type=FieldType.INPUT,
    )
    filter: Optional["Expression"] = p_regular(
        102, default=None, struct=StructType.EXPRESSION, field_type=FieldType.INPUT
    )


@node_subtype_(ActionType.SEARCH)
class SearchAction(Action):
    """Search for Nodes."""

    node_type: NodeType | None = p_regular(100, default=None, field_type=FieldType.INPUT)
    base_block: Optional["Block"] = p_regular(
        101,
        array=False,
        require=False,
        default=None,
        references=NodeType.BLOCK,
        field_type=FieldType.INPUT,
    )
    filter: Optional["Expression"] = p_regular(
        102, default=None, struct=StructType.EXPRESSION, field_type=FieldType.INPUT
    )
    sort: Optional[list["Expression"]] = p_regular(
        103, default=None, array=True, struct=StructType.EXPRESSION, field_type=FieldType.INPUT
    )


#
# Write
#


@node_subtype_(ActionType.CREATE)
class CreateAction(Action):
    node_partial_packed = p_value_packed(100, field_type=FieldType.INPUT)
    node_partial: Any = p_value_runtime(
        100,
        kind=ObjectKind.BUILTIN,
        typ=lambda self: CreateAction._node_partial_type(),
        field_type=FieldType.INPUT,
    )

    @classmethod
    @cachetools.cached({})  # :CachedTypeInfo
    def _node_partial_type(cls) -> "TypeBase":
        return TypeInfo(kind=TypeKind.PARTIAL_OBJECT)


@node_subtype_(ActionType.DUPLICATE)
class DuplicateAction(Action):
    node: Node | None = p_regular(100, require=False, references="any", field_type=FieldType.INPUT)
    if TYPE_CHECKING:
        node_ptr: NodeReference | None = None
    node_partial_packed = p_value_packed(101, field_type=FieldType.INPUT)
    node_partial = p_value_runtime(
        101,
        kind=ObjectKind.BUILTIN,
        typ=lambda self: DuplicateAction._node_partial_type(),
        field_type=FieldType.INPUT,
    )
    is_shallow: bool | None = p_regular(110, default=False, field_type=FieldType.INPUT)

    @classmethod
    @cachetools.cached({})  # :CachedTypeInfo
    def _node_partial_type(cls) -> "TypeBase":
        return TypeInfo(kind=TypeKind.PARTIAL_OBJECT)


@node_subtype_(ActionType.UPDATE)
class UpdateAction(Action):
    node: Node | None = p_regular(100, require=False, references="any", field_type=FieldType.INPUT)
    if TYPE_CHECKING:
        node_ptr: NodeReference | None = None
    node_partial_packed = p_value_packed(101)
    node_partial = p_value_runtime(
        101,
        kind=ObjectKind.BUILTIN,
        typ=lambda self: UpdateAction._node_partial_type(),
        field_type=FieldType.INPUT,
    )

    @classmethod
    @cachetools.cached({})  # :CachedTypeInfo
    def _node_partial_type(cls) -> "TypeBase":
        return TypeInfo(kind=TypeKind.PARTIAL_OBJECT)


@node_subtype_(ActionType.DELETE)
class DeleteAction(Action):
    node: Node | None = p_regular(100, require=False, references="any", field_type=FieldType.INPUT)
    if TYPE_CHECKING:
        node_ptr: NodeReference | None = None


#
# Session
#


@node_subtype_(ActionType.FAIL)
class FailAction(Action):
    error_title: str | None = p_regular(
        100, default=None, require=False, field_type=FieldType.INPUT
    )
    error_text: Optional["Text"] = p_regular(
        101,
        default=None,
        require=False,
        array=False,
        struct=StructType.TEXT,
        field_type=FieldType.INPUT,
    )


#
# Static
#


@node_subtype_(ActionType.CODE)
class CodeAction(Action):
    pass


@node_subtype_(ActionType.DELEGATE)
class DelegateAction(Action):
    pass


@node_subtype_(ActionType.WAIT)
class WaitAction(Action):
    delay: timedelta | None = p_regular(100, default=None, field_type=FieldType.INPUT)


#
# Dynamic
#


@object_()
class HasDynamicContext(BuiltinObject):
    pass


@node_subtype_(ActionType.GENERATE)
class GenerateAction(Action, HasDynamicContext):
    pass


@node_subtype_(ActionType.TRANSFORM)
class TransformAction(Action, HasDynamicContext):
    pass


@node_subtype_(ActionType.EXTRACT)
class ExtractAction(Action, HasDynamicContext):
    pass


@node_subtype_(ActionType.ROUTE)
class RouteAction(Action, HasDynamicContext):
    pass


#
# Application
#


@object_()
class HasApplicationContext(BuiltinObject):
    element_id: str | None = p_regular(100)
    element_path: str | None = p_regular(101)
    element_position: Optional["Vector2"] = p_regular(
        102, default=None, array=False, struct=StructType.VECTOR2, field_type=FieldType.INPUT
    )


@node_subtype_(ActionType.OBSERVE)
class ObserveAction(Action):
    exclude_image: bool | None = p_regular(100, default=False, field_type=FieldType.INPUT)
    screenshot: Optional["File"] = p_regular(
        150,
        require=False,
        array=False,
        references=NodeType.FILE,
        field_type=FieldType.OUTPUT,
    )
    dom: Optional["DomNode"] = p_regular(
        151,
        require=False,
        array=False,
        struct=StructType.DOM_NODE,
        field_type=FieldType.OUTPUT,
    )


@node_subtype_(ActionType.CLICK)
class ClickAction(Action, HasApplicationContext):
    pass


@node_subtype_(ActionType.PRESS)
class PressAction(Action, HasApplicationContext):
    keys: str | None = p_regular(110, default=None, field_type=FieldType.INPUT)
    delay: float | None = p_regular(111, default=None, field_type=FieldType.INPUT)


@node_subtype_(ActionType.TYPE)
class TypeAction(Action, HasApplicationContext):
    string: str | None = p_regular(110, default=None, field_type=FieldType.INPUT)
    delay: float | None = p_regular(111, default=None, field_type=FieldType.INPUT)


@node_subtype_(ActionType.SCROLL)
class ScrollAction(Action, HasApplicationContext):
    amount: Optional["Vector2"] = p_regular(
        110,
        default=None,
        require=False,
        array=False,
        struct=StructType.VECTOR2,
        field_type=FieldType.INPUT,
    )


@node_subtype_(ActionType.SELECT)
class SelectAction(Action, HasApplicationContext):
    pass


@node_subtype_(ActionType.GO_BACKWARD)
class GoBackwardAction(Action, HasApplicationContext):
    pass


@node_subtype_(ActionType.GO_FORWARD)
class GoForwardAction(Action, HasApplicationContext):
    pass


#
# Web
#


@node_subtype_(ActionType.GO_TO_URL)
class GoToUrlAction(Action):
    url: str | None = p_regular(100, default=None, field_type=FieldType.INPUT)


@node_subtype_(ActionType.GO_TO_TAB)
class GoToTabAction(Action):
    tab_index: int | None = p_regular(100, default=None, field_type=FieldType.INPUT)


#
# Control flow
#


@node_subtype_(ActionType.LOOP)
class LoopAction(Action):
    for_field: Optional["Field"] = p_regular(
        100,
        require=False,
        array=False,
        references=NodeType.FIELD,
        field_type=FieldType.INPUT,
    )


@struct_(StructType.CALL)
class Call(Struct):
    """A Call to something."""

    node: Union["Block", "Action", None] = p_regular(
        31,
        require=True,
        references=(NodeType.BLOCK, NodeType.ACTION),
        constraint=constraint(node_subtypes=[BlockType.FLOW]),
    )
    action_type: ActionType | None = p_regular(32, default=None)
    inputs_packed: Any = p_value_packed(35)
    inputs: Any = p_value_runtime(
        35, kind=ObjectKind.INPUT, typ=lambda self: cast(Call, self).input_type
    )
    mapping: Optional["ObjectMapping"] = p_regular(
        50,
        require=False,
        array=False,
        struct=StructType.OBJECT_MAPPING,
        description="Mapping for inputs from current node into called node.",
    )

    @property
    def input_type(self) -> Optional["TypeBase"]:
        node = self.node
        return node.input_type if node is not None else None

    @staticmethod
    def new(node: "Block | Action", inputs: Any | None = None, **kwargs) -> "Call":
        input_type = node.input_type
        assert input_type is not None, f"no input type for {node!r}"
        return Call(
            node=node,
            inputs=coerce_custom_object_scalar(ObjectKind.INPUT, inputs or {}, input_type),
            **kwargs,
        )
