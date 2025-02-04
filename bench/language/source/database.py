from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    NAME_CONSTRAINT,
    FieldType,
    LocalNodeList,
    NodeType,
    RemoteNodeList,
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
from bench.pb2 import BlockData, RecordData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Field, Icon, Package, Page, Record, Text

# pyright: reportIncompatibleVariableOverride=false

_type = type


@node_(NodeType.DATABASE, passthrough_get=("fields",))
class Database(SourceNode[BlockData]):
    """A Database of Records."""

    parent: Union["Page", "Package", None] = p_node_parent(4, NodeType.PAGE, NodeType.PACKAGE)

    # content
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.ICON
    )
    text: Optional["Text"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.TEXT
    )

    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    records: RemoteNodeList["Record", RecordData] = p_node_children(
        NodeType.RECORD, list=RemoteNodeList
    )

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

    @staticmethod
    def new(name: str, **kwargs) -> "Database":
        return Database(name=name, **kwargs)
