from typing import Optional

from bench.language.core import (
    VERSION,
    BuiltinEnum,
    EnumType,
    NodeType,
    ProvisionableResourceBase,
    enum_,
    node_,
    p_kernel,
    p_system,
)
from bench.pb2 import DatabaseData

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.DATABASE_TYPE)
class DatabaseType(BuiltinEnum):
    POSTGRES = 1


@node_(NodeType.DATABASE)
class Database(ProvisionableResourceBase[DatabaseData]):
    """A trusty Postgres-compatible database."""

    type: DatabaseType = p_system(30, default=DatabaseType.POSTGRES)

    version: str = p_system(60, default=VERSION, default_sql=None)
    external_name: Optional[str] = p_kernel(62, sensitive=True)
    external_id: Optional[str] = p_kernel(63, sensitive=True)
    sql_url: Optional[str] = p_kernel(
        64, defer=True, sensitive=True
    )  # Store.sql_url should probably be :RealSecrets
