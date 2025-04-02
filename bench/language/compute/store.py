from typing import Optional

from bench.language.core import (
    VERSION,
    BuiltinEnum,
    EnumType,
    NodeType,
    Resource,
    enum_,
    node_,
    p_kernel,
    p_system,
)
from bench.pb2 import StoreData

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.STORE_TYPE)
class StoreType(BuiltinEnum):
    POSTGRES = 1


@node_(NodeType.STORE)
class Store(Resource[StoreData]):
    """A trusty Postgres-compatible database."""

    type: StoreType = p_system(30, default=StoreType.POSTGRES)

    version: str = p_system(60, default=VERSION, default_sql=None)
    external_name: Optional[str] = p_kernel(62, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(63, require=False, default=None, sensitive=True)
    sql_url: Optional[str] = p_kernel(
        64, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
