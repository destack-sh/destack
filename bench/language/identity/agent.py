from functools import cached_property
from typing import TYPE_CHECKING, Literal, Optional, Union
from uuid import UUID

from bench.language.core import (
    ColorType,
    FieldType,
    IsClaimable,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsProcessable,
    IsRunnable,
    IsSubject,
    IsTimed,
    IsType,
    LocalNodeList,
    NodeReference,
    NodeType,
    PageNode,
    StructType,
    TypeKind,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.pb2 import AgentData

if TYPE_CHECKING:
    from bench.language import (
        Channel,
        Claim,
        Cursor,
        CursorType,
        Field,
        Page,
        Plan,
        Text,
        Thread,
    )

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.AGENT)
class Agent(
    IsTimed,
    IsInstantiable,
    IsOwnable,
    IsClaimable,
    IsModal,
    IsRunnable,
    IsProcessable,
    IsSubject,
    IsNamed,
    PageNode[AgentData],
):
    """
    An Agent is an autonomous entity that creates and implements Plans and Tasks.
    Agents run Flows using Actions, Resources, Pages, and other Bench stuff.
    Agents are not directly runnable; Agent instances are implemented by running their Flow.
    """

    # meta
    parent: Union["Page", "Channel", "Thread", "Agent", None] = p_node_parent(
        4, NodeType.PAGE, NodeType.CHANNEL, NodeType.THREAD, NodeType.AGENT
    )

    # content
    page: Optional["Page"] = p_regular(
        54,
        require=False,
        array=False,
        references=NodeType.PAGE,
        same_bench=True,
        description="The main or root Page used by this Agent (may be shared).",
    )
    plan: Optional["Plan"] = p_regular(
        55,
        require=False,
        array=False,
        references=NodeType.PLAN,
        same_bench=True,
        description="The main Plan to consider in this Agent (may be on the Page).",
    )
    cursor: Optional["Cursor"] = p_regular(
        56,
        require=False,
        array=False,
        references=NodeType.CURSOR,
        same_bench=True,
        description="The main Cursor for this Agent.",
    )
    text: Optional["Text"] = p_regular(57, default=None, struct=StructType.TEXT)
    color: ColorType | None = p_regular(59)
    if TYPE_CHECKING:
        page_ptr: Optional[NodeReference] = None
        page_id: Optional[UUID] = None
        plan_ptr: Optional[NodeReference] = None
        plan_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None
        cursor_id: Optional[UUID] = None

    # ...IsProcessable[80-]

    claims: LocalNodeList["Claim"] = p_node_children(NodeType.CLAIM)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    cursors: LocalNodeList["Cursor"] = p_node_children(NodeType.CURSOR)

    def to_type_maybe(
        self,
        *,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "IsType":
        """Get a type represented by this Block (if any)"""
        from bench.language.core import Type

        if of == "instance":
            return Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RUN)
        else:
            field_types = field_types or []
            return Type(
                kind=TypeKind.CUSTOM_OBJECT,
                base_type=self,
                base_field_types=field_types,
                property_field_types=field_types,
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

    def get_cursor(self, *, type: "CursorType") -> "Cursor | None":
        """Get a Cursor of the given type."""
        for cursor in self.cursors:
            if cursor.type == type:
                return cursor
        return None

    @staticmethod
    def new(name: str, **kwargs) -> "Agent":
        agent = Agent(name=name, **kwargs)
        return agent
