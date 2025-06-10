from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    BuiltinEnum,
    BuiltinObjectMutable,
    EnumType,
    IsInSpace,
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
from destack.pb2 import DatabaseData

if TYPE_CHECKING:
    from destack.language import Space


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.DATABASE_TYPE)
class DatabaseType(BuiltinEnum):
    POSTGRES = 1
    # CASSANDRA, CLICKHOUSE, ...


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
    connection_url: str | None = property_(58, can_read="system", can_write="system")

    def to_info(self) -> "DatabaseInfo":
        return DatabaseInfo(
            type=self.type,
            region=self.region,
            cell_name=self.cell_name,
            external_name=self.external_name,
            custom_schema_name=self.custom_schema_name,
            tenancy=self.tenancy,
            connection_url=self.connection_url,
        )


@struct_(StructType.DATABASE_INFO)
class DatabaseInfo(DatabaseBase, StructMutable):
    pass


@node_(NodeType.DATABASE)
class Database(
    IsResource,
    IsInSpace,
    IsTracked,
    DatabaseBase,
    Node[DatabaseData],
):
    """A primary storage Database of some flavor."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
