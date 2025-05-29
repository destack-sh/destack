from bench.language.core import (
    VERSION,
    BuiltinEnum,
    EnumType,
    IsInPackage,
    IsProvisionable,
    IsRegional,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
)
from bench.pb2 import DatabaseData

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.DATABASE_TYPE)
class DatabaseType(BuiltinEnum):
    POSTGRES = 1


@node_(NodeType.DATABASE)
class Database(IsProvisionable, IsRegional, IsInPackage, Node[DatabaseData]):
    """A trusty Postgres-compatible database."""

    type: DatabaseType = property_(30, default=DatabaseType.POSTGRES)

    version: str = property_(60, default=VERSION)
    schema_name: str | None = property_(61, can_read="system", can_write="system")
    external_name: str | None = property_(62, can_read="system", can_write="system")
    external_id: str | None = property_(63, can_read="system", can_write="system")
    sql_url: str | None = property_(64, can_read="system", can_write="system")
    # Database.sql_url should maybe be :RealSecrets?
