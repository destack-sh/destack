from functools import cached_property
from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    NAME_CONSTRAINT,
    FieldType,
    LocalNodeList,
    NodeType,
    SourceNode,
    StructType,
    TypeBase,
    TypeKind,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.pb2 import BlockData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Action, Field, Icon, Page, Pipe, Text

# pyright: reportIncompatibleVariableOverride=false

_type = type


@node_(NodeType.FLOW, passthrough_get=("fields",))
class Flow(SourceNode[BlockData]):
    """A building block with logic, types, UI, state, auth, AI, ..."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)

    # content
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.ICON
    )
    text: Optional["Text"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.TEXT
    )

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
    def new(name: str, **kwargs) -> "Flow":
        return Flow(name=name, **kwargs)
