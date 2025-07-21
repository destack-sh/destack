from typing import TYPE_CHECKING

from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    Resource,
    Tenancy,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Icon


# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.DATABASE_TYPE)
class DatabaseType(Enum):
    POSTGRES = 1
    # CASSANDRA, CLICKHOUSE, ...


@builtin_node(NodeType.DATABASE)
class Database(Resource):
    """A primary storage Database of some flavor."""

    type: DatabaseType = builtin_property(100, is_repr=True)
    icon: "Icon | None" = builtin_property(102)

    galaxy_name: str | None = builtin_property(200, is_repr=True)
    external_name: str = builtin_property(201, is_repr=True)
    custom_schema_name: str | None = builtin_property(202, is_repr=True)
    tenancy: Tenancy = builtin_property(203, default=Tenancy.DEDICATED, is_repr=True)
    connection_url: str | None = builtin_property(204)
