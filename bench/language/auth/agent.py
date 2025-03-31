from functools import cached_property
from typing import TYPE_CHECKING, Any, Literal, Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    ColorType,
    CustomObject,
    FieldType,
    InlineNode,
    IsClaimable,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsSubject,
    LocalNodeList,
    NodeReference,
    NodeType,
    StructType,
    TypeBase,
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
    from bench.language import Channel, Claim, Field, Flow, Page, Run, Text, Thread

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.AGENT)
class Agent(
    IsInstantiable,
    IsOwnable,
    IsClaimable,
    IsModal,
    IsSubject,
    IsNamed,
    InlineNode[AgentData],
):
    """An Agent is an autonomous entity that implements Plans/Tasks using Actions and Resources."""

    # meta
    parent: Union["Page", "Channel", "Thread", None] = p_node_parent(
        4, NodeType.PAGE, NodeType.CHANNEL, NodeType.THREAD
    )

    main_flow: Optional["Flow"] = p_regular(
        40,
        require=False,
        references=NodeType.FLOW,
        description="The main Flow backing this Agent.",
    )
    implemented_by: Optional["Run"] = p_regular(
        41,
        require=False,
        references=NodeType.RUN,
        description="The Run implementing this Agent.",
    )
    color: ColorType | None = p_regular(45)
    if TYPE_CHECKING:
        main_flow_ptr: Optional[NodeReference] = None
        main_flow_id: Optional[UUID] = None
        implemented_by_ptr: Optional[NodeReference] = None
        implemented_by_id: Optional[UUID] = None

    # content
    inputs_packed: Any = p_value_packed(61)
    inputs: "CustomObject | None" = p_value_runtime(
        61, type=FieldType.INPUT, typ=lambda self: cast("Run", self).input_type
    )
    outputs_packed: Any = p_value_packed(62)
    outputs: "CustomObject | None" = p_value_runtime(
        62, type=FieldType.OUTPUT, typ=lambda self: cast("Run", self).output_type
    )
    text: Optional["Text"] = p_regular(65, default=None, struct=StructType.TEXT)

    claims: LocalNodeList["Claim"] = p_node_children(NodeType.CLAIM)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)

    def to_type_maybe(
        self,
        *,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase":
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
    ) -> "TypeBase":
        typ = self.to_type_maybe(of=of, field_types=field_types)
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @cached_property  # :CachedTypeInfo
    def input_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.INPUT])

    @cached_property  # :CachedTypeInfo
    def output_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.OUTPUT])

    @staticmethod
    def new(name: str, **kwargs) -> "Agent":
        agent = Agent(name=name, **kwargs)
        return agent
