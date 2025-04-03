from functools import cached_property
from typing import TYPE_CHECKING, Any, Literal, Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    ColorType,
    EnumType,
    FieldType,
    IsClaimable,
    IsComputable,
    IsFieldBase,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsRunnable,
    IsType,
    LocalNodeList,
    NodeReference,
    NodeSubtypeStub,
    NodeType,
    PackageNode,
    RunType,
    StructType,
    TypeKind,
    enum_,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.pb2 import ActionData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Agent,
        Claim,
        Code,
        Field,
        Flow,
        Icon,
        Kit,
        Link,
        LinkType,
        NodeReference,
        RunOptions,
        Text,
        Trigger,
        Vector2,
    )

# pyright: reportIncompatibleVariableOverride=false

_type = type


@enum_(EnumType.ACTION_TYPE)
class ActionType(BuiltinEnum):
    # orchestrate
    START = 10, "Start", "Begin the Flow", "fas fa-circle-play", ColorType.YELLOW
    END = 20, "End", "Complete the Flow", "fas fa-flag-checkered", ColorType.YELLOW
    # WAIT = 30, "Wait", "Wait for some trigger"

    # action
    TOOL = 100, "Tool", "Delegate to a specific tool", "fas fa-screwdriver-wrench", ColorType.ORANGE
    CODE = 101, "Code", "Run some Code", "fas fa-code", ColorType.ORANGE
    DO = 200, "Do", "Perform an arbitrary action", "fas fa-hammer", ColorType.ORANGE

    # containers
    # GROUP, LOOP, ...

    @property
    def is_boundary(self) -> bool:
        return self < 40


@node_(NodeType.ACTION, has_subtypes=True)
class Action(
    IsComputable,
    IsClaimable,
    IsInstantiable,
    IsNamed,
    IsModal,
    IsFieldBase,
    IsRunnable,
    PackageNode[ActionData],
):
    """
    A unit of work to do, usually expressed with Code, some Flow or some other tool.
    """

    parent: Union["Flow", "Kit", "Action", None] = p_node_parent(
        4, NodeType.FLOW, NodeType.KIT, NodeType.ACTION
    )

    # common
    type: ActionType = p_regular(30, description="Type of this Action. Only dynamic for tools.")
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
    position: Optional["Vector2"] = p_regular(
        45, default=None, require=False, array=False, struct=StructType.VECTOR2
    )

    # content
    code: Optional["Code"] = p_regular(
        52,
        default=None,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="The implementation code for this action.",
    )
    tool: Union["Agent", "Flow", "Action", None] = p_regular(
        53,
        require=False,
        array=False,
        references=(NodeType.AGENT, NodeType.FLOW, NodeType.ACTION),
        description="The implementation for this action.",
    )
    inputs_packed: Any = p_value_packed(55)
    inputs: Any = p_value_runtime(
        55, type=FieldType.INPUT, typ=lambda self: cast(Action, self).input_type
    )
    if TYPE_CHECKING:
        tool_ptr: "NodeReference | None" = None
        tool_id: Optional[UUID] = None
        tool_ck: Optional[UUID] = None

    actions: LocalNodeList["Action"] = p_node_children(NodeType.ACTION)
    links: LocalNodeList["Link"] = p_node_children(NodeType.LINK)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    triggers: LocalNodeList["Trigger"] = p_node_children(NodeType.TRIGGER)
    claims: LocalNodeList["Claim"] = p_node_children(NodeType.CLAIM)

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
        )
        parent.links.append(link)
        return link

    def to_type_maybe(
        self,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "IsType | None":
        """Gets a type represented by this Action (if any)"""
        from bench.language import Agent, Kit, Type

        if of == "instance":
            return Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RUN)
        else:
            if self.type == ActionType.END:
                if field_types and FieldType.INPUT not in field_types:
                    return None
                base = self.parent
                assert not isinstance(base, Kit), f"{self!r} is invalid inside {base!r}"
                field_types = [FieldType.OUTPUT]  # remap to only output fields from Flow
            elif self.type == ActionType.TOOL:
                base = self.tool
                if isinstance(base, Agent):
                    base = base.main_flow
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
    ) -> "IsType":
        typ = self.to_type_maybe(of=of, field_types=field_types)
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @cached_property  # :CachedTypeInfo
    def input_type(self) -> "IsType | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.INPUT])

    @cached_property  # :CachedTypeInfo
    def output_type(self) -> "IsType | None":
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
