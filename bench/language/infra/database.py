from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    VERSION,
    BuiltinEnum,
    EnumType,
    IsInBench,
    IsRegional,
    IsResource,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import DatabaseData

if TYPE_CHECKING:
    from bench.language import Bench


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.DATABASE_TYPE)
class DatabaseType(BuiltinEnum):
    DEDICATED = 1
    SHARED = 2


@node_(NodeType.DATABASE)
class Database(IsResource, IsRegional, IsInBench, Node[DatabaseData]):
    """A Postgres-compatible Database."""

    type: DatabaseType = property_(30, default=DatabaseType.DEDICATED)
    parent: Optional["Bench"] = property_parent_()

    version: str = property_(60, default=VERSION, can_write="system")
    custom_schema_name: str | None = property_(61, can_read="system", can_write="system")
    sql_url: str | None = property_(64, can_read="system", can_write="system")
