from functools import cached_property
from typing import TYPE_CHECKING, Literal, Optional, Union

from bench.language.core import (
    FieldType,
    InlineSourceNode,
    LocalNodeList,
    NodeType,
    StructType,
    TypeBase,
    TypeKind,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.pb2 import FlowData

if TYPE_CHECKING:
    from bench.language import Action, Field, Page, Pipe, RunOptions

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.FLOW, passthrough_get=("fields",))
class Flow(InlineSourceNode[FlowData]):
    """A building block with logic, types, UI, state, auth, AI, ..."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)

    run_options: Optional["RunOptions"] = p_regular(
        40, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )

    actions: LocalNodeList["Action"] = p_node_children(NodeType.ACTION)
    pipes: LocalNodeList["Pipe"] = p_node_children(NodeType.PIPE)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)

    def __content_str__(self):
        return ""

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
    def variable_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.VARIABLE])

    @cached_property  # :CachedTypeInfo
    def input_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.INPUT])

    @cached_property  # :CachedTypeInfo
    def output_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.OUTPUT])

    @staticmethod
    def new(name: str, **kwargs) -> "Flow":
        flow = Flow(name=name, **kwargs)
        return flow
