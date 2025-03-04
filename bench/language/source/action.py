from datetime import timedelta
from functools import cached_property
from typing import TYPE_CHECKING, Any, Literal, Optional, Union, cast
from uuid import UUID

import cachetools

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    BuiltinObject,
    ColorType,
    EnumType,
    FieldType,
    IsComputable,
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

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Browser,
        Code,
        DomNode,
        Field,
        File,
        Flow,
        Icon,
        Implementation,
        Link,
        LinkType,
        Message,
        NodeReference,
        RunOptions,
        Selection,
        Text,
        Trigger,
        Vector2,
    )

# pyright: reportIncompatibleVariableOverride=false

_type = type


@enum_(EnumType.ACTION_CATEGORY)
class ActionCategory(BuiltinEnum):
    READ = 1, "Read", "Reading", "fas fa-eye"
    SEARCH = 2, "Search", "Searching", "fas fa-magnifying-glass"
    BROWSE = 3, "Browse", "Browsing", "fas fa-globe"
    TYPE = 4, "Type", "Typing", "fas fa-keyboard"
    SPEAK = 5, "Speak", "Speaking", "fas fa-microphone"
    THINK = 7, "Think", "Thinking", "fas fa-brain-circuit"
    WAIT = 8, "Wait", "Waiting", "fas fa-clock"
    WORK = 10, "Work", "Working", "fas fa-hammer"
    INTERACT = 11, "Interact", "Interacting", "fas fa-hand-pointer"


@enum_(EnumType.ACTION_TYPE)
class ActionType(BuiltinEnum):
    # flow
    START = 1, "Start", "Begin the Flow", "fas fa-circle-play", ColorType.YELLOW
    COMPLETE = 10, "Complete", "Complete the entire Flow", "fas fa-flag-checkered", ColorType.YELLOW
    FAIL = 11, "Fail", "Fail the entire Flow", "fas fa-triangle-exclamation", ColorType.YELLOW
    # ABORT?

    # tool
    TOOL = 100, "Tool", "Delegate to a specific tool", "fas fa-screwdriver-wrench", ColorType.SKY
    CODE = 101, "Code", "Run some Code", "fas fa-code", ColorType.SKY
    # SHELL, ...

    # dynamic
    DO = 200, "Do", "Perform an arbitrary action", "fas fa-hammer", ColorType.VIOLET
    THINK = 201, "Think", "Reflect on the context", "fas fa-brain-circuit", ColorType.VIOLET
    ROUTE = 202, "Route", "Route between Actions", "fas fa-split", ColorType.VIOLET
    GENERATE = (
        203,
        "Generate",
        "Generate something new",
        "fas fa-wand-magic-sparkles",
        ColorType.VIOLET,
    )
    TRANSFORM = (
        204,
        "Transform",
        "Change the form of something",
        "fas fa-arrows-rotate",
        ColorType.VIOLET,
    )
    EXTRACT = 205, "Extract", "Extract structured data", "fas fa-filter", ColorType.VIOLET
    CLASSIFY = 206, "Classify", "Classify or categorize", "fas fa-tags", ColorType.VIOLET
    SUMMARIZE = 207, "Summarize", "Condense media content", "fas fa-file-lines", ColorType.VIOLET
    COMPARE = 208, "Compare", "Compare multiple things", "fas fa-code-compare", ColorType.VIOLET
    TRANSLATE = 209, "Translate", "Translate between languages", "fas fa-language", ColorType.VIOLET
    EDIT = 210, "Edit", "Edit this Bench", "fas fa-pen-to-square", ColorType.VIOLET

    # read
    # AGGREGATE?
    # COPY?

    # write
    CREATE = 400, "Create", "Create a Node", "fas fa-plus", ColorType.SKY
    DUPLICATE = 401, "Duplicate", "Duplicate a Node", "fas fa-clone", ColorType.SKY
    UPDATE = 402, "Update", "Update a Node", "fas fa-pencil", ColorType.SKY
    DELETE = 403, "Delete", "Delete a Node", "fas fa-trash", ColorType.SKY
    # PASTE?

    # communicate
    WAIT = 500, "Wait", "Wait for some trigger", "fas fa-clock", ColorType.PINK
    SEND = 510, "Send", "Send a Message", "fas fa-inbox-out", ColorType.PINK
    RECEIVE = 511, "Receive", "Receive a Message", "fas fa-inbox-in", ColorType.PINK
    YIELD = 520, "Yield", "Defer to someone", "fas fa-hand", ColorType.PINK
    # NOTIFY?

    # resource
    # ACQUIRE, PROVISION, SUSPEND, DECOMMISSION, ...

    # runtime
    # PAUSE, RESUME, STOP, KILL, ...

    # ...

    # environment
    LOOK = 800, "Look", "Look at the environment", "fas fa-eye", ColorType.EMERALD
    # LISTEN, ...

    # application
    CLICK = 1000, "Click", "Click an element", "fas fa-arrow-pointer", ColorType.INDIGO
    PRESS = 1001, "Press", "Press a key", "fas fa-keyboard", ColorType.INDIGO
    TYPE = 1002, "Type", "Type text", "fas fa-keyboard", ColorType.INDIGO
    SCROLL = (
        1003,
        "Scroll",
        "Scroll the mouse wheel",
        "fas fa-computer-mouse-scrollwheel",
        ColorType.INDIGO,
    )
    SELECT = 1004, "Select", "Select an element", "fas fa-lasso", ColorType.INDIGO
    DRAG = 1005, "Drag", "Drag an element", "fas fa-hand-pointer", ColorType.INDIGO
    GO_BACKWARD = 1006, "Go back", "Go back in history", "fas fa-arrow-turn-left", ColorType.INDIGO
    GO_FORWARD = (
        1007,
        "Go forward",
        "Go forward in history",
        "fas fa-arrow-turn-right",
        ColorType.INDIGO,
    )
    GO_TO_URL = 1050, "Go to URL", "Navigate to a URL", "fas fa-link", ColorType.INDIGO
    GO_TO_TAB = 1051, "Go to tab", "Switch to a tab", "fas fa-sidebar", ColorType.INDIGO
    OPEN_TAB = 1052, "Open tab", "Open a new tab", "fas fa-plus", ColorType.INDIGO
    CLOSE_TAB = 1053, "Close tab", "Close a tab", "fas fa-minus", ColorType.INDIGO

    # data
    HTTP = 1100, "HTTP", "Make an HTTP call", "fas fa-globe", ColorType.INDIGO
    REST = 1101, "REST", "Make a REST call", "fas fa-brackets-curly", ColorType.INDIGO
    GRAPHQL = 1102, "GraphQL", "Make a GraphQL call", "fas fa-brackets-curly", ColorType.INDIGO
    SQL = 1103, "SQL", "Make a SQL call", "fas fa-code", ColorType.INDIGO
    # GRPC, JDBC, SOQL, ...

    # internet
    WEB = 1200, "Web", "Search the Web", "fas fa-globe", ColorType.INDIGO
    # ...

    # containers
    # GROUP, LOOP, ...

    @property
    def category(self) -> ActionCategory:
        return ACTION_CATEGORY_BY_TYPE.get(self) or ActionCategory.WORK

    @property
    def is_boundary(self) -> bool:
        return self < 40

    @property
    def is_dynamic(self) -> bool:
        return self >= 200 and self < 300

    @property
    def is_container(self) -> bool:
        return self >= 8000 and self < 9000


# NOTE: the default ActionCategory is WORK
ACTION_CATEGORY_BY_TYPE = {
    ActionType.THINK: ActionCategory.THINK,
    ActionType.WAIT: ActionCategory.WAIT,
    ActionType.SEND: ActionCategory.TYPE,
    ActionType.RECEIVE: ActionCategory.TYPE,
    ActionType.YIELD: ActionCategory.WAIT,
    # application
    **{t: ActionCategory.INTERACT for t in ActionType if t >= 1000 and t < 1100},
    # data
    **{t: ActionCategory.READ for t in ActionType if t >= 1100 and t < 1200},
    # internet
    **{t: ActionCategory.BROWSE for t in ActionType if t >= 1200 and t < 1300},
}


@node_(NodeType.ACTION, passthrough_get=("value", "fields"), has_subtypes=True)
class Action(SourceNode[ActionData], IsComputable):
    """
    A data or control flow node in a Flow. Actions are connected by Links.
    """

    parent: Union["Flow", "Implementation", "Action", None] = p_node_parent(
        4, NodeType.FLOW, NodeType.IMPLEMENTATION, NodeType.ACTION
    )

    # common
    type: ActionType = p_regular(
        30, description="Type of this Action. Only dynamic for tools.", field_type=FieldType.INPUT
    )
    category: ActionCategory | None = p_regular(31, description="Category of this Action.")
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
    selection: Optional["Selection"] = p_regular(
        41,
        default=None,
        require=False,
        array=False,
        struct=StructType.SELECTION,
        description="The selection of Tools to use.",
    )

    # inputs
    code: Optional["Code"] = p_regular(
        52,
        default=None,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="The implementation code for this action.",
        field_type=FieldType.INPUT,
    )
    tool: Union["Flow", "Action", None] = p_regular(
        53,
        require=False,
        array=False,
        references=(NodeType.FLOW, NodeType.ACTION),
        description="The implementation for this action.",
        field_type=FieldType.INPUT,
    )
    if TYPE_CHECKING:
        tool_ptr: "NodeReference | None" = None
        tool_id: Optional[UUID] = None
        tool_ck: Optional[UUID] = None
    # variables for tool
    variables_packed: Any = p_value_packed(55)
    variables: Any = p_value_runtime(
        55,
        type=FieldType.VARIABLE,
        typ=lambda self: cast("Action", self).variable_type_field_only,
    )
    # inputs for tool
    inputs_packed: Any = p_value_packed(56)
    inputs: Any = p_value_runtime(
        56,
        type=FieldType.INPUT,
        typ=lambda self: cast("Action", self).input_type_field_only,
    )

    # outputs
    # fanout/fanin, ...?

    # flags
    # is_streaming: bool = p_regular(80, default=False)

    # flow
    position: Optional["Vector2"] = p_regular(
        80, default=None, require=False, array=False, struct=StructType.VECTOR2
    )

    actions: LocalNodeList["Action"] = p_node_children(NodeType.ACTION)
    links: LocalNodeList["Link"] = p_node_children(NodeType.LINK)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    triggers: LocalNodeList["Trigger"] = p_node_children(NodeType.TRIGGER)

    @property
    def flow(self) -> "Flow | None":
        """Gets the containing ancestor Flow (if any)"""
        from bench.language import Flow

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Flow):
                return parent
        return None

    @property
    def container(self) -> "Node | None":
        return self.flow

    def run_type(self) -> RunType:
        return RunType.ACTION

    def connect(
        self,
        type: "LinkType",
        target: "Action",
        name: str | None = None,
        *,
        parent: Union["Flow", "Implementation", "Action", None] = None,
        run_options: "RunOptions | None" = None,
        is_manual: bool = False,
    ) -> "Link":
        """Connects a target Action to this Action."""
        from bench.language import Flow, Link

        parent = parent or self.parent
        assert parent is not None, f"{self!r} is not attached to a parent"
        assert isinstance(parent, (Action, Flow)), f"{parent!r} is not valid for {self!r}"

        # assign next name like Pipe1, .. in parent :AutoNaming
        if name is None:
            siblings = parent.links.tolist()
            count = len(siblings) + 1
            name = f"{type.bench_name}{count}"
            while any(p.name == name for p in siblings):
                count += 1
                name = f"{type.bench_name}{count}"

        link = Link(
            type=type,
            name=name,
            source=self,
            target=target,
            parent=parent,
            run_options=run_options,
            is_manual=is_manual,
        )
        parent.links.append(link)
        return link

    def to_type_maybe(
        self,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
        field_only: bool | None = None,
    ) -> "TypeBase | None":
        """Gets a type represented by this Action (if any)"""
        from bench.language import Implementation, Type

        if of == "instance":
            return Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RUN)
        else:
            property_field_types = field_types
            if self.type == ActionType.COMPLETE:
                if field_types and FieldType.INPUT not in field_types:
                    return None
                base = self.parent
                assert not isinstance(base, Implementation), f"{self!r} is invalid inside {base!r}"
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
                return Type(
                    kind=TypeKind.PARTIAL_OBJECT,
                    base_type=base,
                    bench_type=NodeType.ACTION,
                    base_field_types=field_types,
                    property_field_types=property_field_types or [],
                    constraint=TypeConstraint(node_subtypes=[self.type]),
                )
            else:
                return Type(
                    kind=TypeKind.CUSTOM_OBJECT, base_type=base, base_field_types=field_types or []
                )

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


@subnode_(ActionType.EDIT)
class EditAction(Action):
    pass


#
# Read
#


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
    # is_blocking, ...?
    message: Optional["Message"] = p_regular(
        200, require=False, array=False, references=NodeType.MESSAGE, field_type=FieldType.OUTPUT
    )


@subnode_(ActionType.RECEIVE)
class ReceiveAction(Action):
    message: Optional["Message"] = p_regular(
        100, require=False, array=False, references=NodeType.MESSAGE, field_type=FieldType.INPUT
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
