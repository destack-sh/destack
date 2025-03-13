from functools import cached_property
from typing import TYPE_CHECKING, Literal, Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    ColorType,
    EnumType,
    FieldType,
    IsComputable,
    IsTemplatable,
    IsTraceable,
    LocalNodeList,
    NodeReference,
    NodeSubtypeStub,
    NodeType,
    PackageNode,
    RunType,
    StructType,
    TypeBase,
    TypeKind,
    enum_,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    subnode_,
)
from bench.pb2.lang_pb2 import ActionData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Code,
        Field,
        Flow,
        Icon,
        Kit,
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
    ORCHESTRATE = 1, "Orchestrate", "Orchestrating", "fas fa-arrows-rotate", ColorType.YELLOW
    COMPUTE = 100, "Compute", "Computing", "fas fa-code", ColorType.SKY
    WORK = 200, "Work", "Working", "fas fa-hammer", ColorType.VIOLET
    WRITE = 300, "Write", "Writing", "fas fa-pencil", ColorType.SKY
    COMMUNICATE = 400, "Communicate", "Communicating", "fas fa-inbox-out", ColorType.PINK
    OBSERVE = 800, "Observe", "Observing", "fas fa-eye", ColorType.EMERALD
    INTERACT = 1000, "Interact", "Interacting", "fas fa-hand-pointer", ColorType.INDIGO
    FETCH = 1100, "Fetch", "Fetching", "fas fa-download", ColorType.INDIGO


@enum_(EnumType.ACTION_TYPE)
class ActionType(BuiltinEnum):
    # orchestrate
    START = 10, "Start", "Begin the Flow", "fas fa-circle-play", ColorType.YELLOW
    COMPLETE = 20, "Complete", "Complete the entire Flow", "fas fa-flag-checkered", ColorType.YELLOW
    # WAIT = 30, "Wait", "Wait for some trigger", "fas fa-clock", ColorType.PINK
    # FAIL = 11, "Fail", "Fail the entire Flow", "fas fa-triangle-exclamation", ColorType.YELLOW
    # ABORT?

    # compute
    TOOL = 100, "Tool", "Delegate to a specific tool", "fas fa-screwdriver-wrench", ColorType.SKY
    CODE = 101, "Code", "Run some Code", "fas fa-code", ColorType.SKY

    # work
    DO = 200, "Do", "Perform an arbitrary action", "fas fa-hammer", ColorType.VIOLET
    # THINK = 201, "Think", "Reflect on the context", "fas fa-brain-circuit", ColorType.VIOLET
    # ROUTE = 202, "Route", "Route between Actions", "fas fa-split", ColorType.VIOLET
    # GENERATE = (
    #     203,
    #     "Generate",
    #     "Generate something new",
    #     "fas fa-wand-magic-sparkles",
    #     ColorType.VIOLET,
    # )
    # TRANSFORM = (
    #     204,
    #     "Transform",
    #     "Change the form of something",
    #     "fas fa-arrows-rotate",
    #     ColorType.VIOLET,
    # )
    # EXTRACT = 205, "Extract", "Extract structured data", "fas fa-filter", ColorType.VIOLET
    # EDIT = 210, "Edit", "Edit this Bench", "fas fa-pen-to-square", ColorType.VIOLET

    # read
    # AGGREGATE?
    # COPY?

    # nocheckin: turn builtin-Actions into builtin Action nodes (in bench package)

    # write
    # CREATE = 400, "Create", "Create a Node", "fas fa-plus", ColorType.SKY
    # DUPLICATE = 401, "Duplicate", "Duplicate a Node", "fas fa-clone", ColorType.SKY
    # UPDATE = 402, "Update", "Update a Node", "fas fa-pencil", ColorType.SKY
    # DELETE = 403, "Delete", "Delete a Node", "fas fa-trash", ColorType.SKY
    # PASTE?

    # communicate
    # SEND = 510, "Send", "Send a Message", "fas fa-inbox-out", ColorType.PINK
    RECEIVE = 511, "Receive", "Receive a Message", "fas fa-inbox-in", ColorType.PINK
    # YIELD = 520, "Yield", "Defer to someone", "fas fa-hand", ColorType.PINK
    # NOTIFY?

    # resource
    # ACQUIRE, PROVISION, SUSPEND, DECOMMISSION, ...

    # runtime
    # PAUSE, RESUME, STOP, KILL, ...

    # ...

    # environment
    # LOOK = 800, "Look", "Look at the environment", "fas fa-eye", ColorType.EMERALD
    # LISTEN, ...

    # application
    # CLICK = 1000, "Click", "Click an element", "fas fa-arrow-pointer", ColorType.INDIGO
    # PRESS = 1001, "Press", "Press a key", "fas fa-keyboard", ColorType.INDIGO
    # TYPE = 1002, "Type", "Type text", "fas fa-keyboard", ColorType.INDIGO
    # SCROLL = (
    #     1003,
    #     "Scroll",
    #     "Scroll the mouse wheel",
    #     "fas fa-computer-mouse-scrollwheel",
    #     ColorType.INDIGO,
    # )
    # SELECT = 1004, "Select", "Select an element", "fas fa-lasso", ColorType.INDIGO
    # DRAG = 1005, "Drag", "Drag an element", "fas fa-hand-pointer", ColorType.INDIGO
    # GO_BACKWARD = 1006, "Go back", "Go back in history", "fas fa-arrow-turn-left", ColorType.INDIGO
    # GO_FORWARD = (
    #     1007,
    #     "Go forward",
    #     "Go forward in history",
    #     "fas fa-arrow-turn-right",
    #     ColorType.INDIGO,
    # )
    # GO_TO_URL = 1050, "Go to URL", "Navigate to a URL", "fas fa-link", ColorType.INDIGO
    # GO_TO_TAB = 1051, "Go to tab", "Switch to a tab", "fas fa-sidebar", ColorType.INDIGO
    # OPEN_TAB = 1052, "Open tab", "Open a new tab", "fas fa-plus", ColorType.INDIGO
    # CLOSE_TAB = 1053, "Close tab", "Close a tab", "fas fa-minus", ColorType.INDIGO

    # data
    # HTTP = 1100, "HTTP", "Make an HTTP call", "fas fa-globe", ColorType.INDIGO
    # REST = 1101, "REST", "Make a REST call", "fas fa-brackets-curly", ColorType.INDIGO
    # GRAPHQL = 1102, "GraphQL", "Make a GraphQL call", "fas fa-brackets-curly", ColorType.INDIGO
    # SQL = 1103, "SQL", "Make a SQL call", "fas fa-code", ColorType.INDIGO
    # GRPC, JDBC, SOQL, ...

    # internet
    # ...

    # containers
    # GROUP, LOOP, ...

    @property
    def category(self) -> ActionCategory:
        if self < 100:
            return ActionCategory.ORCHESTRATE
        else:
            return ActionCategory(self / 100)

    @property
    def is_boundary(self) -> bool:
        return self < 40

    @property
    def is_dynamic(self) -> bool:
        return self >= 200 and self < 300

    @property
    def is_container(self) -> bool:
        return self >= 8000 and self < 9000


@node_(NodeType.ACTION, passthrough_get=("value", "fields"), has_subtypes=True)
class Action(IsComputable, IsTemplatable, IsTraceable, PackageNode[ActionData]):
    """
    A data or control flow node in a Flow. Actions are connected by Links.
    """

    parent: Union["Flow", "Kit", "Action", None] = p_node_parent(
        4, NodeType.FLOW, NodeType.KIT, NodeType.ACTION
    )

    # common
    type: ActionType = p_regular(30, description="Type of this Action. Only dynamic for tools.")
    category: ActionCategory | None = p_regular(31, description="Category of this Action.")
    name: str | None = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.ICON
    )
    text: Optional["Text"] = p_regular(
        36, default=None, require=False, array=False, struct=StructType.TEXT
    )

    # meta
    options: Optional["RunOptions"] = p_regular(
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
    )
    tool: Union["Flow", "Action", None] = p_regular(
        53,
        require=False,
        array=False,
        references=(NodeType.FLOW, NodeType.ACTION),
        description="The implementation for this action.",
    )
    if TYPE_CHECKING:
        tool_ptr: "NodeReference | None" = None
        tool_id: Optional[UUID] = None
        tool_ck: Optional[UUID] = None

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
            parent = parent.parent
        return None

    @property
    def kit(self) -> "Kit | None":
        """Gets the containing ancestor Kit (if any)"""
        from bench.language import Kit

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Kit):
                return parent
            parent = parent.parent
        return None

    def run_type(self) -> RunType:
        return RunType.ACTION

    def connect(
        self,
        type: "LinkType",
        target: "Action",
        name: str | None = None,
        *,
        parent: Union["Flow", "Kit", "Action", None] = None,
        options: "RunOptions | None" = None,
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
            options=options,
            is_manual=is_manual,
        )
        parent.links.append(link)
        return link

    def to_type_maybe(
        self,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase | None":
        """Gets a type represented by this Action (if any)"""
        from bench.language import Kit, Type

        if of == "instance":
            return Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RUN)
        else:
            if self.type == ActionType.COMPLETE:
                if field_types and FieldType.INPUT not in field_types:
                    return None
                base = self.parent
                assert not isinstance(base, Kit), f"{self!r} is invalid inside {base!r}"
                field_types = [FieldType.OUTPUT]  # remap to only output fields from Flow
            elif self.type == ActionType.TOOL:
                base = self.tool
            else:
                base = self
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
    def resource_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.RESOURCE])

    @cached_property  # :CachedTypeInfo
    def input_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.INPUT])

    @cached_property  # :CachedTypeInfo
    def output_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.OUTPUT])

    @staticmethod
    def new[ActionT: "Action" = "Action"](
        typ: Union["Action", ActionType, _type[ActionT], NodeSubtypeStub[ActionT]],
        name: str | None = None,
        **kwargs,
    ) -> ActionT:
        """Creates a new Action of the given type."""
        if isinstance(typ, Action):
            action = Action(type=ActionType.TOOL, tool=typ, name=name or typ.name, **kwargs)
            return cast(ActionT, action)
        elif isinstance(typ, type):
            typ = Action.__subtype_by_subclass__[typ]  # type: ignore
        elif isinstance(typ, NodeSubtypeStub):
            typ = cast(ActionType, typ._node_subtype)
        action = Action(
            type=cast(ActionType, typ), name=name or cast(ActionType, typ).bench_name, **kwargs
        )
        return cast(ActionT, action)


#
# Orchestrate
#


@subnode_(ActionType.START)
class StartAction(Action):
    pass


@subnode_(ActionType.COMPLETE)
class CompleteAction(Action):
    pass


#
# Compute
#


@subnode_(ActionType.CODE)
class CodeAction(Action):
    pass


@subnode_(ActionType.TOOL)
class ToolAction(Action):
    pass


#
# Work
#


#
# Read
#


#
# Communicate
#


@subnode_(ActionType.RECEIVE)
class ReceiveAction(Action):
    message: Optional["Message"] = p_regular(
        100, require=False, array=False, references=NodeType.MESSAGE, field_type=FieldType.INPUT
    )
