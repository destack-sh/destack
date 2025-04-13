from functools import cached_property
from typing import TYPE_CHECKING, Any, Literal, Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    ColorType,
    CustomObject,
    FieldType,
    InlineNode,
    IsClaimable,
    IsComputable,
    IsFieldBase,
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
    StructType,
    TypeKind,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.pb2 import AgentData

if TYPE_CHECKING:
    from bench.language import (
        Channel,
        Claim,
        Cursor,
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
    IsFieldBase,
    IsRunnable,
    IsComputable,
    IsProcessable,
    IsSubject,
    IsNamed,
    InlineNode[AgentData],
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
    inputs_packed: Any = p_value_packed(50)
    inputs: "CustomObject | None" = p_value_runtime(
        50, type=FieldType.INPUT, typ=lambda self: cast("Agent", self).input_type
    )
    outputs_packed: Any = p_value_packed(51)
    outputs: "CustomObject | None" = p_value_runtime(
        51, type=FieldType.OUTPUT, typ=lambda self: cast("Agent", self).output_type
    )
    main_page: Optional["Page"] = p_regular(
        54,
        require=False,
        array=False,
        references=NodeType.PAGE,
        same_bench=True,
        description="The main or root Page used by this Agent (may be shared).",
    )
    main_plan: Optional["Plan"] = p_regular(
        55,
        require=False,
        array=False,
        references=NodeType.PLAN,
        same_bench=True,
        description="The main Plan to consider in this Agent (may be on the Page).",
    )
    main_cursor: Optional["Cursor"] = p_regular(
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
        main_page_ptr: Optional[NodeReference] = None
        main_page_id: Optional[UUID] = None
        main_plan_ptr: Optional[NodeReference] = None
        main_plan_id: Optional[UUID] = None
        main_cursor_ptr: Optional[NodeReference] = None
        main_cursor_id: Optional[UUID] = None

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

    @staticmethod
    def new(name: str, **kwargs) -> "Agent":
        agent = Agent(name=name, **kwargs)
        return agent
