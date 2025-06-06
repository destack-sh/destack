from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinObjectMutable,
    IsInBench,
    IsResource,
    Node,
    NodeType,
    Region,
    StructMutable,
    StructType,
    Tenancy,
    node_,
    object_,
    property_,
    property_parent_,
    struct_,
)
from bench.pb2 import DatabaseData

if TYPE_CHECKING:
    from bench.language import Bench


# pyright: reportIncompatibleVariableOverride=false


@object_()
class DatabaseBase(BuiltinObjectMutable):
    region: Region = property_(50, can_write="system", is_repr=True)
    cell_name: str | None = property_(51, can_write="system", is_repr=True)
    external_name: str = property_(52, can_read="system", can_write="system", is_repr=True)
    custom_schema_name: str | None = property_(
        53, can_read="system", can_write="system", is_repr=True
    )
    tenancy: Tenancy = property_(55, default=Tenancy.DEDICATED, is_repr=True)
    sql_url: str | None = property_(58, can_read="system", can_write="system")


@struct_(StructType.DATABASE_INFO)
class DatabaseInfo(DatabaseBase, StructMutable):
    pass


@node_(NodeType.DATABASE)
class Database(IsResource, IsInBench, DatabaseBase, Node[DatabaseData]):
    """A Postgres-compatible Database."""

    parent: Optional["Bench"] = property_parent_()
    # type?

    def to_info(self) -> DatabaseInfo:
        return DatabaseInfo(
            region=self.region,
            cell_name=self.cell_name,
            external_name=self.external_name,
            custom_schema_name=self.custom_schema_name,
            tenancy=self.tenancy,
            sql_url=self.sql_url,
        )
