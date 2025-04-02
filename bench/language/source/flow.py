from functools import cached_property
from typing import TYPE_CHECKING, Literal, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    FieldType,
    InlineNode,
    IsClaimable,
    IsComputable,
    IsFieldBase,
    IsModal,
    IsNamed,
    IsOwnable,
    IsRunnable,
    IsTemplatable,
    IsType,
    LocalNodeList,
    NodeReference,
    NodeType,
    StructType,
    Text,
    TypeKind,
    enum_,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.pb2 import FlowData

if TYPE_CHECKING:
    from bench.language import Action, Agent, Claim, Field, Link, Page, RunOptions

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.FLOW_TYPE)
class FlowType(BuiltinEnum):
    ACTION = 10, "Action", "Link Actions into a procedural Flow"
    # PLAN? (lay out a sequence of Tasks declaratively)
    # MESSAGE? (define communication links between agents)
    # ESCALATION/AUTH?


@node_(NodeType.FLOW)
class Flow(
    IsComputable,
    IsTemplatable,
    IsOwnable,
    IsClaimable,
    IsModal,
    IsNamed,
    IsRunnable,
    IsFieldBase,
    InlineNode[FlowData],
):
    """A building block with logic, types, UI, state, auth, AI, ..."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)
    type: FlowType = p_regular(30, default=FlowType.ACTION, default_sql=None)

    # meta
    options: Optional["RunOptions"] = p_regular(
        40, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )
    text: Optional["Text"] = p_regular(
        41, default=None, require=False, array=False, struct=StructType.TEXT
    )

    # auth
    default_agent: Optional["Agent"] = p_regular(
        50,
        require=False,
        array=False,
        references=NodeType.AGENT,
        description="The default Agent to use for this Flow.",
    )
    if TYPE_CHECKING:
        default_agent_ptr: Optional[NodeReference] = None
        default_agent_id: Optional[UUID] = None

    actions: LocalNodeList["Action"] = p_node_children(NodeType.ACTION)
    links: LocalNodeList["Link"] = p_node_children(NodeType.LINK)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    claims: LocalNodeList["Claim"] = p_node_children(NodeType.CLAIM)

    def __content_str__(self):
        return ""

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
    def new(name: str, **kwargs) -> "Flow":
        flow = Flow(name=name, **kwargs)
        return flow
