from functools import cached_property
from typing import TYPE_CHECKING, Literal, Optional
from uuid import UUID

from bench.language.core import (
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
    NodeReference,
    NodeType,
    PageNode,
    TypeBase,
    TypeKind,
    node_,
    p_regular,
)
from bench.pb2 import AgentData

if TYPE_CHECKING:
    from bench.language import Cursor, Page

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
    An Agent is an autonomous entity.
    """

    # content
    page: Optional["Page"] = p_regular(
        54,
        same_bench=True,
        description="The main Page used by this Agent.",
    )
    cursor: Optional["Cursor"] = p_regular(
        55,
        same_bench=True,
        description="The main Cursor for this Agent.",
    )
    if TYPE_CHECKING:
        page_ptr: Optional[NodeReference] = None
        page_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None
        cursor_id: Optional[UUID] = None

    # ...IsProcessable[80-]

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
