from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    VERSION,
    BuiltinEnum,
    BuiltinObjectMutable,
    EnumType,
    IsInBench,
    IsResource,
    Node,
    NodeType,
    Region,
    StructMutable,
    StructType,
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


@enum_(EnumType.TENANCY)
class Tenancy(BuiltinEnum):
    DEDICATED = 1
    SHARED = 2


@object_()
class DatabaseBase(BuiltinObjectMutable):
    region: Region = property_(50, can_write="system", is_repr=True)
    cell_name: str | None = property_(51, can_write="system", is_repr=True)
    external_id: str = property_(52, can_read="system", can_write="system", is_repr=True)
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
    version: str = property_(35, default=VERSION, can_write="system")

    def to_info(self) -> DatabaseInfo:
        return DatabaseInfo(
            region=self.region,
            cell_name=self.cell_name,
            external_id=self.external_id,
            custom_schema_name=self.custom_schema_name,
            tenancy=self.tenancy,
            sql_url=self.sql_url,
        )
