from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    BuiltinObjectMutable,
    Enum,
    EnumType,
    HasName,
    Node,
    NodeType,
    Region,
    Resource,
    RoleType,
    Spatial,
    StructMutable,
    StructType,
    Tenancy,
    builtin_enum,
    builtin_node,
    builtin_struct,
    object_,
    property_,
    property_parent_,
)
from destack.proto import DatabaseProto

if TYPE_CHECKING:
    from destack.language import Space


# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.DATABASE_TYPE)
class DatabaseType(Enum):
    POSTGRES = 1
    # CASSANDRA, CLICKHOUSE, ...


@object_()
class DatabaseBase(BuiltinObjectMutable):
    type: DatabaseType = property_(30, can_write=RoleType.SYSTEM, is_repr=True)
    region: Region = property_(50, can_write=RoleType.SYSTEM, is_repr=True)
    galaxy_name: str | None = property_(51, can_write=RoleType.SYSTEM, is_repr=True)
    external_name: str = property_(
        52, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM, is_repr=True
    )
    custom_schema_name: str | None = property_(
        53, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM, is_repr=True
    )
    tenancy: Tenancy = property_(55, default=Tenancy.DEDICATED, is_repr=True)
    connection_url: str | None = property_(58, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM)

    def to_info(self) -> "DatabaseInfo":
        return DatabaseInfo(
            type=self.type,
            region=self.region,
            galaxy_name=self.galaxy_name,
            external_name=self.external_name,
            custom_schema_name=self.custom_schema_name,
            tenancy=self.tenancy,
            connection_url=self.connection_url,
        )


@builtin_struct(StructType.DATABASE_INFO)
class DatabaseInfo(DatabaseBase, StructMutable):
    pass


@builtin_node(NodeType.DATABASE)
class Database(
    Spatial,
    Resource,
    HasName,
    DatabaseBase,
    Node[DatabaseProto],
):
    """A primary storage Database of some flavor."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
