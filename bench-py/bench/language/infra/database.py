from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObjectMutable,
    EnumType,
    IsInBench,
    IsResource,
    IsTracked,
    Node,
    NodeType,
    Region,
    StructMutable,
    StructType,
    Tenancy,
    enum_,
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


@enum_(EnumType.DATABASE_TYPE)
class DatabaseType(BuiltinEnum):
    POSTGRES = 1
    # CASSANDRA?


@object_()
class DatabaseBase(BuiltinObjectMutable):
    type: DatabaseType = property_(30, can_write="system", is_repr=True)
    region: Region = property_(50, can_write="system", is_repr=True)
    cell_name: str | None = property_(51, can_write="system", is_repr=True)
    external_name: str = property_(52, can_read="system", can_write="system", is_repr=True)
    custom_schema_name: str | None = property_(
        53, can_read="system", can_write="system", is_repr=True
    )
    tenancy: Tenancy = property_(55, default=Tenancy.DEDICATED, is_repr=True)
    sql_url: str | None = property_(58, can_read="system", can_write="system")

    def to_info(self) -> "DatabaseInfo":
        return DatabaseInfo(
            type=self.type,
            region=self.region,
            cell_name=self.cell_name,
            external_name=self.external_name,
            custom_schema_name=self.custom_schema_name,
            tenancy=self.tenancy,
            sql_url=self.sql_url,
        )


@struct_(StructType.DATABASE_INFO)
class DatabaseInfo(DatabaseBase, StructMutable):
    pass


@node_(NodeType.DATABASE)
class Database(
    IsResource,
    IsInBench,
    IsTracked,
    DatabaseBase,
    Node[DatabaseData],
):
    """A relational Database."""

    parent: Optional["Bench"] = property_parent_()
