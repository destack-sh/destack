from datetime import timedelta
from functools import cached_property
from typing import TYPE_CHECKING, Any, Literal, Optional, Union, assert_never, cast

import cachetools

from bench.language.core import (
    NAME_CONSTRAINT,
    BlockType,
    BuiltinObject,
    EnumType,
    FieldType,
    LocalNodeList,
    Node,
    NodeReference,
    NodeSubtypeStub,
    NodeType,
    RunType,
    SourceNode,
    StructType,
    Type,
    TypeBase,
    TypeConstraint,
    TypeKind,
    coerce_custom_object_scalar,
    constraint,
    enum_,
    node_,
    object_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
    subnode_,
)
from bench.pb2.lang_pb2 import ActionData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Block,
        Browser,
        CallPlan,
        Code,
        DomNode,
        Expression,
        Field,
        File,
        Icon,
        Machine,
        NodeReference,
        Pipe,
        PipeType,
        RunOptions,
        Text,
        Vector2,
    )

# pyright: reportIncompatibleVariableOverride=false

_type = type


@enum_(EnumType.ACTION_TYPE)
class ActionType(IdEnum):
    # flow
    START = 1, "Begin the Flow"
    COMPLETE = 10, "Complete the entire Flow"
    FAIL = 11, "Fail the entire Flow"
    # ABORT?

    # tool
    TOOL = 100, "Delegate to a tool"
    CODE = 101, "Run some Code"

    # dynamic
    ACT = 200, "Perform an arbitrary action"
    THINK = 201, "Reflect on the context"
    ROUTE = 202, "Route between Actions"
    GENERATE = 203, "Generate something new"
    TRANSFORM = 204, "Change the form of something"
    EXTRACT = 205, "Extract structured data"
    CLASSIFY = 206, "Classify or categorize"
    SUMMARIZE = 207, "Condense media content"
    COMPARE = 208, "Compare multiple things"
    TRANSLATE = 209, "Translate between languages"
    CHANGE = 210, "Edit this Bench"

    # read
    GET = 300, "Get a Node"
    SEARCH = 301, "Search for Nodes"
    # AGGREGATE?
    COPY = 302, "Copy some Nodes"

    # write
    CREATE = 400, "Create a Node"
    DUPLICATE = 401, "Duplicate a Node"
    UPDATE = 402, "Update a Node"
    DELETE = 403, "Delete a Node"
    PASTE = 404, "Paste a Node"

    # async
    SEND = 500, "Send a Message"
    RECEIVE = 501, "Receive a Message"
    MESSAGE = 502, "Send & Receive a Message"
    WAIT = 505, "Wait for something"
    YIELD = 506, "Defer to someone"
    NOTIFY = 510, "Notify someone"

    # resource
    # DOWNLOAD, UPLOAD, PROVISION, SUSPEND, DECOMMISSION, ...

    # runtime
    # PAUSE, RESUME, STOP, ...

    # state
    # ...

    # environment
    LOOK = 800, "Look at the environment"
    # LISTEN, ...

    # application
    CLICK = 1000, "Click an element"
    PRESS = 1001, "Press a key"
    TYPE = 1002, "Type text"
    SCROLL = 1003, "Scroll the mouse wheel"
    SELECT = 1004, "Select an element"
    DRAG = 1005, "Drag an element"
    GO_BACKWARD = 1006, "Go back in history"
    GO_FORWARD = 1007, "Go forward in history"

    # web
    GO_TO_URL = 1100, "Navigate to a URL"
    GO_TO_TAB = 1101, "Switch to a tab"
    OPEN_TAB = 1102, "Open a new tab"
    CLOSE_TAB = 1103, "Close a tab"

    # containers
    # GROUP = 8000, "Associate multiple Actions"
    # LOOP = 8001, "Repeat some Actions"

    # misc
    TEXT = 9000, "Just some documentation"

    @property
    def is_boundary(self) -> bool:
        return self < 40

    @property
    def is_dynamic(self) -> bool:
        return self >= 200 and self < 300

    @property
    def is_container(self) -> bool:
        return self >= 8000 and self < 9000


DYNAMIC_ACTION_TYPES = [t for t in ActionType if t.is_dynamic]


@node_(NodeType.ACTION, passthrough_get=("value", "fields"), has_subtypes=True)
class Action(SourceNode[ActionData]):
    """
    A data or control flow node in a Flow. Actions are connected by Pipes.
    """

    parent: Union["Block", "Action", None] = p_node_parent(4, NodeType.BLOCK, NodeType.ACTION)

    # common
    type: ActionType = p_regular(
        30, description="Type of this Action. Only dynamic for tools.", field_type=FieldType.INPUT
    )
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
        40, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )
    # roles, identity, ...

    # inputs
    machine: Optional["Machine"] = p_regular(
        51, require=False, references=(NodeType.MACHINE,), field_type=FieldType.INPUT
    )
    code: Optional["Code"] = p_regular(
        52,
        default=None,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="The implementation code for this action.",
        field_type=FieldType.INPUT,
    )
    tool: Union["Block", None] = p_regular(
        53,
        require=False,
        array=False,
        references=(NodeType.BLOCK,),
        constraint=constraint(node_subtypes=[BlockType.FLOW]),
        description="The implementation for this action.",
        field_type=FieldType.INPUT,
    )
    if TYPE_CHECKING:
        tool_ptr: "NodeReference | None" = None
    # variables for tool
    variables_packed: Any = p_value_packed(55)
    variables: Any = p_value_runtime(
        55,
        type=FieldType.VARIABLE,
        typ=lambda self: cast("Action", self).variable_type_field_only,
    )
    # inputs for delegate
    inputs_packed: Any = p_value_packed(56)
    inputs: Any = p_value_runtime(
        56,
        type=FieldType.INPUT,
        typ=lambda self: cast("Action", self).input_type_field_only,
    )

    # outputs
    plans: list["CallPlan"] = p_regular(
        60,
        default=None,
        require=False,
        array=True,
        struct=StructType.CALL_PLAN,
        field_type=FieldType.OUTPUT,
    )
    # fanout/fanin, ...?

    # flags
    # is_streaming: bool = p_regular(80, default=False)

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
        from bench.language.source.block import Block

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
        name: str | None = None,
        *,
        parent: Union["Block", "Action", None] = None,
        run_options: "RunOptions | None" = None,
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
            name = f"{type.bench_name}{count}"
            while any(p.name == name for p in siblings):
                count += 1
                name = f"{type.bench_name}{count}"

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

    def to_type_maybe(
        self,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
        field_only: bool | None = None,
    ) -> "TypeBase | None":
        """Gets a type represented by this Action (if any)"""
        from bench.language import Type

        if of == "instance":
            typ = Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RUN)
            return typ
        elif of == "value":
            property_field_types = field_types
            if self.type == ActionType.START:
                if field_types and FieldType.OUTPUT not in field_types:
                    return None
                base = self.parent
                property_field_types = field_types
                field_types = [FieldType.INPUT]  # remap to only input fields from Flow
            elif self.type == ActionType.COMPLETE:
                if field_types and FieldType.INPUT not in field_types:
                    return None
                base = self.parent
                property_field_types = field_types
                field_types = [FieldType.OUTPUT]  # remap to only output fields from Flow
            elif self.type == ActionType.TOOL:
                base = self.tool
            else:
                base = self
            if (
                field_types
                and (FieldType.INPUT in field_types or FieldType.OUTPUT in field_types)
                and not field_only
            ):
                # actions also have their subtype as input & output type
                #  (to support dynamically setting some action properties as inputs)
                typ = Type(
                    kind=TypeKind.PARTIAL_OBJECT,
                    base_type=base,
                    bench_type=NodeType.ACTION,
                    base_field_types=field_types,
                    property_field_types=property_field_types or [],
                    constraint=TypeConstraint(node_subtypes=[self.type]),
                )
            else:
                typ = Type(
                    kind=TypeKind.CUSTOM_OBJECT, base_type=base, base_field_types=field_types or []
                )
            return typ
        else:
            assert_never(of)

    def to_type(
        self,
        *,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase":
        typ = self.to_type_maybe(of=of, field_types=field_types)
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @cached_property  # :CachedTypeInfo
    def variable_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.VARIABLE])

    @cached_property
    def variable_type_field_only(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.VARIABLE], field_only=True)

    @cached_property  # :CachedTypeInfo
    def input_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.INPUT])

    @cached_property  # :CachedTypeInfo
    def input_type_field_only(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.INPUT], field_only=True)

    @cached_property  # :CachedTypeInfo
    def output_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.OUTPUT])

    @staticmethod
    def new[ActionT: "Action" = "Action"](
        typ: ActionType | _type[ActionT] | NodeSubtypeStub[ActionT],
        name: str,
        variables: dict[str, Any] | None = None,
        inputs: dict[str, Any] | None = None,
        **kwargs,
    ) -> ActionT:
        """Creates a new Action of the given type."""
        if isinstance(typ, type):
            typ = Action.__subtype_by_subclass__[typ]  # type: ignore
        elif isinstance(typ, NodeSubtypeStub):
            typ = cast(ActionType, typ._node_subtype)
        action = Action(type=cast(ActionType, typ), name=name, **kwargs)
        if variables is not None:
            variable_type = action.variable_type
            assert variable_type is not None, f"no variable_type for {action!r}"
            action.variables = coerce_custom_object_scalar(variables, variable_type)
        if inputs is not None:
            input_type = action.input_type
            assert input_type is not None, f"no input_type for {action!r}"
            action.inputs = coerce_custom_object_scalar(inputs, input_type)
        return cast(ActionT, action)


#
# Flow
#


@subnode_(ActionType.START)
class StartAction(Action):
    pass


@subnode_(ActionType.COMPLETE)
class CompleteAction(Action):
    pass


@subnode_(ActionType.FAIL)
class FailAction(Action):
    error_title: str | None = p_regular(
        120, default=None, require=False, field_type=FieldType.INPUT
    )
    error_text: Optional["Text"] = p_regular(
        121,
        default=None,
        require=False,
        array=False,
        struct=StructType.TEXT,
        field_type=FieldType.INPUT,
    )


#
# Generic
#


@subnode_(ActionType.CODE)
class CodeAction(Action):
    pass


@subnode_(ActionType.TOOL)
class ToolAction(Action):
    pass


@subnode_(ActionType.GENERATE)
class GenerateAction(Action):
    pass


@subnode_(ActionType.TRANSFORM)
class TransformAction(Action):
    pass


@subnode_(ActionType.ROUTE)
class RouteAction(Action):
    pass


@subnode_(ActionType.CHANGE)
class ChangeAction(Action):
    nodes: list[Node] = p_regular(
        120, array=True, require=False, references="any", field_type=FieldType.INPUT
    )


#
# Read
#


@subnode_(ActionType.GET)
class GetAction(Action):
    """Get a single Node."""

    node_type: NodeType | None = p_regular(120, default=None, field_type=FieldType.INPUT)
    base_block: Optional["Block"] = p_regular(
        121,
        array=False,
        require=False,
        default=None,
        references=NodeType.BLOCK,
        field_type=FieldType.INPUT,
    )
    filter: Optional["Expression"] = p_regular(
        122, default=None, struct=StructType.EXPRESSION, field_type=FieldType.INPUT
    )


@subnode_(ActionType.SEARCH)
class SearchAction(Action):
    """Search for Nodes."""

    node_type: NodeType | None = p_regular(120, default=None, field_type=FieldType.INPUT)
    base_block: Optional["Block"] = p_regular(
        121,
        array=False,
        require=False,
        default=None,
        references=NodeType.BLOCK,
        field_type=FieldType.INPUT,
    )
    filter: Optional["Expression"] = p_regular(
        122, default=None, struct=StructType.EXPRESSION, field_type=FieldType.INPUT
    )
    sort: Optional[list["Expression"]] = p_regular(
        123, default=None, array=True, struct=StructType.EXPRESSION, field_type=FieldType.INPUT
    )


#
# Write
#


@subnode_(ActionType.CREATE)
class CreateAction(Action):
    node_partial_packed: Any = p_value_packed(120, field_type=FieldType.INPUT, partial=True)
    node_partial: Any = p_value_runtime(
        120,
        typ=lambda self: CreateAction._node_partial_type(),
        field_type=FieldType.INPUT,
        partial=True,
    )
    node: Node | None = p_regular(220, require=False, references="any", field_type=FieldType.OUTPUT)

    @classmethod
    @cachetools.cached({})  # :CachedTypeInfo
    def _node_partial_type(cls) -> "TypeBase":
        return Type(kind=TypeKind.PARTIAL_OBJECT)


@subnode_(ActionType.DUPLICATE)
class DuplicateAction(Action):
    node: Node | None = p_regular(120, require=False, references="any", field_type=FieldType.INPUT)
    if TYPE_CHECKING:
        node_ptr: NodeReference | None = None
    node_partial_packed: Any = p_value_packed(121, field_type=FieldType.INPUT, partial=True)
    node_partial: Any = p_value_runtime(
        121,
        typ=lambda self: DuplicateAction._node_partial_type(),
        field_type=FieldType.INPUT,
        partial=True,
    )
    is_shallow: bool | None = p_regular(122, default=False, field_type=FieldType.INPUT)
    duplicated_node: Node | None = p_regular(
        123, require=False, references="any", field_type=FieldType.OUTPUT
    )

    @classmethod
    @cachetools.cached({})  # :CachedTypeInfo
    def _node_partial_type(cls) -> "TypeBase":
        return Type(kind=TypeKind.PARTIAL_OBJECT)


@subnode_(ActionType.UPDATE)
class UpdateAction(Action):
    node: Node | None = p_regular(120, require=False, references="any", field_type=FieldType.INPUT)
    if TYPE_CHECKING:
        node_ptr: NodeReference | None = None
    node_partial_packed: Any = p_value_packed(121, field_type=FieldType.INPUT, partial=True)
    node_partial: Any = p_value_runtime(
        121,
        typ=lambda self: UpdateAction._node_partial_type(),
        field_type=FieldType.INPUT,
        partial=True,
    )

    @classmethod
    @cachetools.cached({})  # :CachedTypeInfo
    def _node_partial_type(cls) -> "TypeBase":
        return Type(kind=TypeKind.PARTIAL_OBJECT)


@subnode_(ActionType.DELETE)
class DeleteAction(Action):
    node: Node | None = p_regular(120, require=False, references="any", field_type=FieldType.INPUT)
    if TYPE_CHECKING:
        node_ptr: NodeReference | None = None


#
# Async
#


@subnode_(ActionType.SEND)
class SendAction(Action):
    message_type: "Block | None" = p_regular(
        120, require=False, default=None, references=NodeType.BLOCK, field_type=FieldType.INPUT
    )
    node_partial_packed: Any = p_value_packed(121, field_type=FieldType.INPUT, partial=True)
    node_partial: Any = p_value_runtime(
        121,
        typ=lambda self: cast(SendAction, self)._node_partial_type(),
        field_type=FieldType.INPUT,
        partial=True,
    )

    @cachetools.cached({})  # :CachedTypeInfo
    def _node_partial_type(self) -> "TypeBase":
        return Type(kind=TypeKind.PARTIAL_OBJECT, base_type=self.message_type)


@subnode_(ActionType.RECEIVE)
class ReceiveAction(Action):
    pass


@subnode_(ActionType.WAIT)
class WaitAction(Action):
    delay: timedelta | None = p_regular(120, default=None, field_type=FieldType.INPUT)


#
# Application
#


@object_()
class HasApplicationContext(BuiltinObject):
    application: Optional["Browser"] = p_regular(
        100, require=False, references=(NodeType.BROWSER,), field_type=FieldType.INPUT
    )
    element_id: str | None = p_regular(110, field_type=FieldType.INPUT)
    element_path: str | None = p_regular(111, field_type=FieldType.INPUT)
    element_position: Optional["Vector2"] = p_regular(
        112, default=None, array=False, struct=StructType.VECTOR2, field_type=FieldType.INPUT
    )
    element_text: str | None = p_regular(113, default=None, field_type=FieldType.INPUT)


@subnode_(ActionType.LOOK)
class LookAction(Action, HasApplicationContext):  # move out of application?
    exclude_image: bool | None = p_regular(120, default=False, field_type=FieldType.INPUT)
    screenshot: Optional["File"] = p_regular(
        200,
        require=False,
        array=False,
        references=NodeType.FILE,
        field_type=FieldType.OUTPUT,
    )
    dom_nodes: list["DomNode"] = p_regular(
        201,
        require=False,
        array=True,
        struct=StructType.DOM_NODE,
        field_type=FieldType.OUTPUT,
    )


@subnode_(ActionType.CLICK)
class ClickAction(Action, HasApplicationContext):
    pass


@subnode_(ActionType.PRESS)
class PressAction(Action, HasApplicationContext):
    keys: str | None = p_regular(120, default=None, field_type=FieldType.INPUT)
    delay: float | None = p_regular(121, default=None, field_type=FieldType.INPUT)


@subnode_(ActionType.TYPE)
class TypeAction(Action, HasApplicationContext):
    string: str | None = p_regular(120, default=None, field_type=FieldType.INPUT)
    delay: float | None = p_regular(121, default=None, field_type=FieldType.INPUT)


@subnode_(ActionType.SCROLL)
class ScrollAction(Action, HasApplicationContext):
    amount: Optional["Vector2"] = p_regular(
        120,
        default=None,
        require=False,
        array=False,
        struct=StructType.VECTOR2,
        field_type=FieldType.INPUT,
    )


@subnode_(ActionType.SELECT)
class SelectAction(Action, HasApplicationContext):
    pass


@subnode_(ActionType.GO_BACKWARD)
class GoBackwardAction(Action, HasApplicationContext):
    pass


@subnode_(ActionType.GO_FORWARD)
class GoForwardAction(Action, HasApplicationContext):
    pass


#
# Web
#


@subnode_(ActionType.GO_TO_URL)
class GoToUrlAction(Action, HasApplicationContext):
    url: str | None = p_regular(120, default=None, field_type=FieldType.INPUT)


@subnode_(ActionType.GO_TO_TAB)
class GoToTabAction(Action, HasApplicationContext):
    tab_index: int | None = p_regular(120, default=None, field_type=FieldType.INPUT)


#
# Group
#

...
