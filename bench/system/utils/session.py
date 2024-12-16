from datetime import datetime
from uuid import UUID

import cachetools

from bench.language import Bench, NodeReference, NodeType, Store
from bench.language.connection import Engine
from bench.language.const import (
    NODE_TYPES,
    NODE_TYPES_BY_AREA,
    REGION,
    VERSION,
    NodeArea,
    Region,
)
from bench.language.graph import NodeSuperGraph
from bench.language.node import EMPTY_SCOPE_DATA, GraphScope, Node
from bench.language.session import Session
from bench.proto.wire import GraphScopeData
from bench.sql.graph import BenchSqlContext
from bench.system.graph.postgres import PostgresEngine
from bench.utils.oracle import Oracle
from bench.utils.utils import get_from_env

BEGINNING_OF_TIME = datetime.fromisoformat("1970-01-01T00:00:00+00:00")


def make_system_store(region: Region, pg_url: str, pg_crypto_key: str) -> Store:
    system_bench_ptr = NodeReference(node_type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = NodeSuperGraph(root_ptr=system_bench_ptr)
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        region=region,
        encryption_key=pg_crypto_key,
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    store = Store(
        parent=system_bench_stub,
        name="Store",
        version=VERSION,
        connection_uri=pg_url,
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    return store


@cachetools.cached(cache={})
def global_store_from_env() -> Store:
    """Get the default global store configured in the environment"""
    pg = get_from_env("GLOBAL_PG", description="Global Postgres connection string")
    pg_url, pg_crypto_key = pg.split("|", maxsplit=1)
    return make_system_store(REGION, pg_url, pg_crypto_key)


def regional_store_from_env(region: Region = REGION) -> Store:
    """Get the default regional store configured in the environment"""
    from bench.system.utils.sharding import STORE_MAP

    return STORE_MAP.get(region)


def pg_engine_from_store(
    name: str,
    store: Store,
    area: NodeArea | None,
    *,
    scope: GraphScopeData | None = None,
):
    """Get the postgres engine for a store"""
    bench = store.parent
    assert bench is not None, f"missing parent for {store!r}"
    return PostgresEngine(
        name=name,
        store=store,
        bench=bench,
        scope=scope or EMPTY_SCOPE_DATA,
        node_types=NODE_TYPES_BY_AREA[area] if area is not None else NODE_TYPES,
        context=BenchSqlContext(bench),
    )


def local_pg_engine_from_store(name: str, store: Store):
    """Get the postgres engine for a local store"""
    assert store.bench is not None, f"missing bench for {store!r}"
    return pg_engine_from_store(
        name=name,
        store=store,
        area=NodeArea.LOCAL,
        scope=GraphScope(bench_id=store.bench.id)._to_data(),
    )


def global_session(
    node: Node | None,
    engines: tuple[Engine, ...],
    oracle: Oracle,
    *,
    supergraph: NodeSuperGraph | None = None,
    epoch: int | None = None,
    readonly: bool = False,
    split_read: bool = True,
):
    """Create a Session in a global store"""
    if node is None:
        assert supergraph is not None, "must provide supergraph if no node"
    else:
        supergraph = node._supergraph.instance()
    return Session(
        parent=None,
        _default_scope=EMPTY_SCOPE_DATA,
        _engines=engines,
        _local_epoch=epoch,
        _supergraph=supergraph,
        _oracle=oracle,
        _is_readonly=readonly,
        _split_read=split_read,
    )
