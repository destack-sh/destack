from functools import cached_property
from typing import TYPE_CHECKING, Union

from bench.language.core import (
    FieldType,
    InlineSourceNode,
    LocalNodeList,
    NodeType,
    TypeBase,
    TypeKind,
    node_,
    p_node_children,
    p_node_parent,
)
from bench.pb2 import FlowData

if TYPE_CHECKING:
    from bench.language import Action, Field, Page, Pipe

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.FLOW, passthrough_get=("fields",))
class Flow(InlineSourceNode[FlowData]):
    """A building block with logic, types, UI, state, auth, AI, ..."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)

    actions: LocalNodeList["Action"] = p_node_children(NodeType.ACTION)
    pipes: LocalNodeList["Pipe"] = p_node_children(NodeType.PIPE)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)

    def __content_str__(self):
        return ""

    def to_type_maybe(
        self,
        *,
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase":
        """Get a type represented by this Block (if any)"""
        from bench.language.core import Type

        field_types = field_types or [FieldType.MEMBER]
        return Type(
            kind=TypeKind.CUSTOM_OBJECT,
            base_type=self,
            base_field_types=field_types,
            property_field_types=field_types,
        )

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

    @cached_property  # :CachedTypeInfo
    def input_type(self) -> "TypeBase | None":
        return self.to_type_maybe(field_types=[FieldType.INPUT])

    @cached_property  # :CachedTypeInfo
    def output_type(self) -> "TypeBase | None":
        return self.to_type_maybe(field_types=[FieldType.OUTPUT])

    @staticmethod
    def new(name: str, *fields: "Field", **kwargs) -> "Flow":
        flow = Flow(name=name, **kwargs)
        for field in fields:
            flow.fields.append(field)
        return flow
