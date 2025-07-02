from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Enum,
    EnumType,
    HasName,
    IsSpatial,
    NodeType,
    Region,
    Resource,
    RoleType,
    StructFrozen,
    StructType,
    Tenancy,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
    builtin_struct,
)

if TYPE_CHECKING:
    from destack.language import Space


# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.DATABASE_TYPE)
class DatabaseType(Enum):
    POSTGRES = 1
    # CASSANDRA, CLICKHOUSE, ...


@builtin_struct(StructType.DATABASE_INFO, frozen=True)
class DatabaseInfo(StructFrozen):
    type: DatabaseType = builtin_property(30, can_write=RoleType.SYSTEM, is_repr=True)
    region: Region = builtin_property(50, can_write=RoleType.SYSTEM, is_repr=True)
    galaxy_name: str | None = builtin_property(51, can_write=RoleType.SYSTEM, is_repr=True)
    external_name: str = builtin_property(
        52, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM, is_repr=True
    )
    custom_schema_name: str | None = builtin_property(
        53, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM, is_repr=True
    )
    tenancy: Tenancy = builtin_property(55, default=Tenancy.DEDICATED, is_repr=True)
    connection_url: str | None = builtin_property(
        58, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM
    )

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


@builtin_node(NodeType.DATABASE)
class Database(IsSpatial, HasName, Resource):
    """A primary storage Database of some flavor."""

    parent: Optional["Space"] = builtin_property_parent(node_is_extensible=False)

    type: DatabaseType = builtin_property(30, can_write=RoleType.SYSTEM, is_repr=True)
    region: Region = builtin_property(50, can_write=RoleType.SYSTEM, is_repr=True)
    galaxy_name: str | None = builtin_property(51, can_write=RoleType.SYSTEM, is_repr=True)
    external_name: str = builtin_property(
        52, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM, is_repr=True
    )
    custom_schema_name: str | None = builtin_property(
        53, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM, is_repr=True
    )
    tenancy: Tenancy = builtin_property(55, default=Tenancy.DEDICATED, is_repr=True)
    connection_url: str | None = builtin_property(
        58, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM
    )

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
