from functools import cached_property
from typing import TYPE_CHECKING, Literal, Union

from bench.language.core import (
    FieldType,
    IsModal,
    IsNamed,
    IsTemplatable,
    NodeType,
    PageNode,
    TypeBase,
    TypeKind,
    node_,
    p_node_parent,
)
from bench.pb2 import BlockData

if TYPE_CHECKING:
    from bench.language import Field, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.CLASS)
class Class(
    IsTemplatable,
    IsModal,
    IsNamed,
    PageNode[BlockData],
):
    """A Class with Fields."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)

    def __content_str__(self):
        return ""

    def to_type_maybe(
        self,
        *,
        of: Literal["instance", "value"] = "value",
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase":
        """Get a type represented by this Block (if any)"""
        from bench.language.core import Type

        if of == "instance":
            return Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.FIELD)
        else:
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
    def member_type(self) -> "TypeBase | None":
        return self.to_type_maybe(field_types=[FieldType.MEMBER])

    @staticmethod
    def new(name: str, *fields: "Field", **kwargs) -> "Class":
        cls = Class(name=name, **kwargs)
        for field in fields:
            cls.add_child(field)
        return cls
