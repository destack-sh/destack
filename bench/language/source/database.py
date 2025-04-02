from typing import TYPE_CHECKING, Literal

from bench.language.core import (
    FieldType,
    InlineNode,
    IsClaimable,
    IsFieldBase,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsTemplatable,
    IsType,
    LocalNodeList,
    NodeType,
    RecordNodeList,
    TypeKind,
    node_,
    p_node_children,
)
from bench.pb2 import DatabaseData

if TYPE_CHECKING:
    from bench.language import Field

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.DATABASE, passthrough_get=("fields",))
class Database(
    IsInstantiable,
    IsTemplatable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsClaimable,
    IsFieldBase,
    InlineNode[DatabaseData],
):
    """A Database of Records."""

    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    records: RecordNodeList = p_node_children(NodeType.RECORD, list=RecordNodeList)

    def __content_str__(self):
        return ""

    def to_type_maybe(
        self,
        *,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "IsType | None":
        """Get a type represented by this Block (if any)"""
        from bench.language.core import Type

        if of == "instance":
            return Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RECORD)
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
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "IsType":
        typ = self.to_type_maybe(of=of, field_types=field_types)
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @staticmethod
    def new(name: str, *fields: "Field", **kwargs) -> "Database":
        db = Database(name=name, **kwargs)
        for field in fields:
            db.fields.append(field)
        return db
