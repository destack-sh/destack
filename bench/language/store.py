from typing import Optional

from bench.language.bench import StaticResource
from bench.language.const import VERSION, EnumType, NodeType, enum_
from bench.language.node import node_
from bench.language.property import p_kernel, p_system
from bench.proto.wire import StoreData
from bench.utils.func import IdEnum

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.STORE_TYPE)
class StoreType(IdEnum):
    POSTGRES = 1


@node_(NodeType.STORE)
class Store(StaticResource[StoreData]):
    """A trusty Postgres-compatible database."""

    type: StoreType = p_system(30, default=StoreType.POSTGRES)

    version: str = p_system(50, default=VERSION, default_sql=None)
    target_version: str = p_system(51, default=VERSION, default_sql=None)

    external_name: Optional[str] = p_kernel(60, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(61, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_kernel(
        62, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
