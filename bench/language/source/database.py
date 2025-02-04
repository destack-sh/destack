from typing import TYPE_CHECKING

from bench.language.core import (
    FieldType,
    InlineSourceNode,
    LocalNodeList,
    NodeType,
    RemoteNodeList,
    TypeBase,
    TypeKind,
    node_,
    p_node_children,
)
from bench.pb2 import DatabaseData, RecordData

if TYPE_CHECKING:
    from bench.language import Field, Record

# pyright: reportIncompatibleVariableOverride=false

_type = type


@node_(NodeType.DATABASE, passthrough_get=("fields",))
class Database(InlineSourceNode[DatabaseData]):
    """A Database of Records."""

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
    def new(name: str, *fields: "Field", **kwargs) -> "Database":
        db = Database(name=name, **kwargs)
        for field in fields:
            db.fields.append(field)
        return db
