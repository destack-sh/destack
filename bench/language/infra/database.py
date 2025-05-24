from typing import Optional

from bench.language.core import (
    VERSION,
    BuiltinEnum,
    EnumType,
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
class Database(IsProvisionable, IsRegional, Node[DatabaseData]):
    """A trusty Postgres-compatible database."""

    type: DatabaseType = property_(30, default=DatabaseType.POSTGRES)

    version: str = property_(60, default=VERSION)
    external_name: Optional[str] = property_(62, can_read="system", can_write="system")
    external_id: Optional[str] = property_(63, can_read="system", can_write="system")
    sql_url: Optional[str] = property_(64, can_read="system", can_write="system")
    # Database.sql_url should probably be :RealSecrets
