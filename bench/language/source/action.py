from datetime import timedelta
from functools import cached_property
from typing import TYPE_CHECKING, Any, Literal, Optional, Union, assert_never, cast

import cachetools

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
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
    Struct,
    StructType,
    Type,
    TypeBase,
    TypeConstraint,
    TypeKind,
    coerce_custom_object_scalar,
    enum_,
    node_,
    object_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
    struct_,
    subnode_,
)
from bench.pb2.lang_pb2 import ActionData
from bench.utils.fractional import INTEGER_ZERO

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
        Flow,
        Icon,
        Machine,
        Message,
        NodeReference,
        Page,
        Pipe,
        PipeType,
        RunOptions,
        Text,
        Trigger,
        Vector2,
    )

# pyright: reportIncompatibleVariableOverride=false

_type = type


@enum_(EnumType.ACTION_TYPE)
class ActionType(BuiltinEnum):
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
    # COPY?

    # write
    CREATE = 400, "Create a Node"
    DUPLICATE = 401, "Duplicate a Node"
    UPDATE = 402, "Update a Node"
    DELETE = 403, "Delete a Node"
    # PASTE?

    # communicate
    WAIT = 500, "Wait for some trigger"
    SEND = 510, "Send a Message"
    RECEIVE = 511, "Receive a Message"
    YIELD = 520, "Defer to someone"
    # NOTIFY?

    # resource
    # ACQUIRE, PROVISION, SUSPEND, DECOMMISSION, ...

    # runtime
    # PAUSE, RESUME, STOP, KILL, ...

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

    @property
    def category(self) -> "ActionCategory":
        return ActionCategory((self.value // 100) * 100)


@enum_(EnumType.ACTION_CATEGORY)
class ActionCategory(BuiltinEnum):
    FLOW = 100
    READ = 300
    WRITE = 400
    COMMUNICATE = 500
    ENVIRONMENT = 800
    APPLICATION = 1000
    WEB = 1100


DYNAMIC_ACTION_TYPES = [t for t in ActionType if t.is_dynamic]


@node_(NodeType.ACTION, passthrough_get=("value", "fields"), has_subtypes=True)
class Action(SourceNode[ActionData]):
    """
    A data or control flow node in a Flow. Actions are connected by Pipes.
    """

    parent: Union["Flow", "Action", None] = p_node_parent(4, NodeType.FLOW, NodeType.ACTION)

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
    tool_selection: Optional["ToolSelection"] = p_regular(
        41,
        default=None,
        require=False,
        array=False,
        struct=StructType.TOOL_SELECTION,
        field_type=FieldType.INPUT,
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
    tool: Union["Flow", None] = p_regular(
        53,
        require=False,
        array=False,
        references=(NodeType.FLOW,),
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
    triggers: LocalNodeList["Trigger"] = p_node_children(NodeType.TRIGGER)

    @property
    def page(self) -> "Page | None":
        """Gets the containing ancestor Page (if any)"""
        from bench.language import Page

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Page):
                return parent
            parent = parent.parent
        return None

    @property
    def flow(self) -> "Flow | None":
        """Gets the containing ancestor Flow (if any)"""
        from bench.language import Flow

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Flow):
                return parent
        return None

    def run_type(self) -> RunType:
        return RunType.ACTION

    def connect(
        self,
        type: "PipeType",
        target: "Action",
        name: str | None = None,
        *,
        parent: Union["Flow", "Action", None] = None,
        run_options: "RunOptions | None" = None,
    ) -> "Pipe":
        """Connects a target Action to this Action."""
        from bench.language import Flow, Pipe

        parent = parent or self.parent
        assert parent is not None, f"{self!r} is not attached to a parent"
        assert isinstance(parent, (Action, Flow)), f"{parent!r} is not valid for {self!r}"

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
        of: Literal["instance", "value"] = "value",
        field_types: list[FieldType] | None = None,
        field_only: bool | None = None,
    ) -> "TypeBase | None":
        """Gets a type represented by this Action (if any)"""
        from bench.language import Type

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

    def to_type(
        self,
        *,
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase":
        typ = self.to_type_maybe(field_types=field_types)
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @cached_property  # :CachedTypeInfo
    def variable_type(self) -> "TypeBase | None":
        return self.to_type_maybe(field_types=[FieldType.VARIABLE])

    @cached_property
    def variable_type_field_only(self) -> "TypeBase | None":
        return self.to_type_maybe(field_types=[FieldType.VARIABLE], field_only=True)

    @cached_property  # :CachedTypeInfo
    def input_type(self) -> "TypeBase | None":
        return self.to_type_maybe(field_types=[FieldType.INPUT])

    @cached_property  # :CachedTypeInfo
    def input_type_field_only(self) -> "TypeBase | None":
        return self.to_type_maybe(field_types=[FieldType.INPUT], field_only=True)

    @cached_property  # :CachedTypeInfo
    def output_type(self) -> "TypeBase | None":
        return self.to_type_maybe(field_types=[FieldType.OUTPUT])

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


@enum_(EnumType.TOOL_FILTER)
class ToolFilter(BuiltinEnum):
    ANY = 10, "Any Actions"
    SELECT_BUILIN = 20, "Only Builtin Actions"
    SELECT_CUSTOM = 30, "Only Custom Actions"
    SELECT = 40, "Only Specific Actions"


@struct_(StructType.TOOL_SELECTION)
class ToolSelection(Struct):
    """
    Options for dynamic Actions.
    """

    filter: ToolFilter | None = p_regular(35, default=None)
    tool_nodes: list[Union["Flow", "Action"]] = p_regular(
        40, array=True, require=False, references=(NodeType.ACTION, NodeType.FLOW)
    )
    tool_types: list[ActionType] = p_regular(41, array=True, require=False)
    tool_categories: list[ActionCategory] = p_regular(42, array=True, require=False)

    def supports(self, action_type: ActionType, tool: Union["Flow", "Action", None]) -> bool:
        """Whether this tool filter includes the given Action."""
        if self.filter is None or self.filter == ToolFilter.ANY:
            return True
        elif self.filter == ToolFilter.SELECT_CUSTOM:
            # must be in tool_nodes
            return tool is not None and tool in self.tool_nodes
        elif self.filter == ToolFilter.SELECT_BUILIN:
            # must be in tool_types or tool_categories
            return action_type in self.tool_types or action_type.category in self.tool_categories
        elif self.filter == ToolFilter.SELECT:
            # must be in tool_types or tool_categories
            return (
                action_type in self.tool_types
                or action_type.category in self.tool_categories
                or (tool is not None and tool in self.tool_nodes)
            )
        else:
            assert_never(self.filter)

    @staticmethod
    def custom(*tools: Union["Flow", "Action"]) -> "ToolSelection":
        return ToolSelection(filter=ToolFilter.SELECT_CUSTOM, tool_nodes=list(tools))

    @staticmethod
    def builtin(*tools: Union["ActionType", "ActionCategory"]) -> "ToolSelection":
        return ToolSelection(
            filter=ToolFilter.SELECT_BUILIN,
            tool_types=[t for t in tools if isinstance(t, ActionType)],
            tool_categories=[t for t in tools if isinstance(t, ActionCategory)],
        )

    @staticmethod
    def only(*tools: Union["ActionType", "ActionCategory", "Flow", "Action"]) -> "ToolSelection":
        return ToolSelection(
            filter=ToolFilter.SELECT,
            tool_types=[t for t in tools if isinstance(t, ActionType)],
            tool_categories=[t for t in tools if isinstance(t, ActionCategory)],
            tool_nodes=[t for t in tools if isinstance(t, Node)],
        )

    @staticmethod
    def any() -> "ToolSelection":
        return ToolSelection(filter=ToolFilter.ANY)


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
# Communicate
#


@subnode_(ActionType.WAIT)
class WaitAction(Action):
    delay: timedelta | None = p_regular(120, default=None, field_type=FieldType.INPUT)
    node: Optional["Node"] = p_regular(
        200,
        require=False,
        references="any",
        field_type=FieldType.OUTPUT,
        description="The Node the Trigger was waiting on.",
    )


@subnode_(ActionType.SEND)
class SendAction(Action):
    message_in_packed: Any = p_value_packed(121, field_type=FieldType.INPUT, partial=True)
    message_in: Any = p_value_runtime(
        121,
        typ=lambda self: Type(kind=TypeKind.PARTIAL_OBJECT, bench_type=NodeType.MESSAGE),
        field_type=FieldType.INPUT,
        partial=True,
    )
    is_blocking: bool | None = p_regular(
        130, default=False, field_type=FieldType.INPUT, description="Whether to wait for a reply."
    )
    message: Optional["Message"] = p_regular(
        200, require=False, array=False, references=NodeType.MESSAGE, field_type=FieldType.OUTPUT
    )


@subnode_(ActionType.RECEIVE)
class ReceiveAction(Action):
    message: Optional["Message"] = p_regular(
        200, require=False, array=False, references=NodeType.MESSAGE, field_type=FieldType.OUTPUT
    )


@subnode_(ActionType.YIELD)
class YieldAction(Action):
    pass


#
# Application
#


@object_()
class HasApplicationContext(BuiltinObject):
    application: Optional["Browser"] = p_regular(
        100, require=False, references=(NodeType.BROWSER,), field_type=FieldType.INPUT
    )
    element_id: str | None = p_regular(110, field_type=FieldType.INPUT)
    element_position: Optional["Vector2"] = p_regular(
        111, default=None, array=False, struct=StructType.VECTOR2, field_type=FieldType.INPUT
    )


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
    button: Optional[str] = p_regular(120, default=None, field_type=FieldType.INPUT)


@subnode_(ActionType.PRESS)
class PressAction(Action, HasApplicationContext):
    combination: str | None = p_regular(120, default=None, field_type=FieldType.INPUT)
    delay: timedelta | None = p_regular(121, default=None, field_type=FieldType.INPUT)


@subnode_(ActionType.TYPE)
class TypeAction(Action, HasApplicationContext):
    string: str | None = p_regular(120, default=None, field_type=FieldType.INPUT)
    delay: timedelta | None = p_regular(121, default=None, field_type=FieldType.INPUT)


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
