from datetime import datetime
from uuid import UUID

from bench.language import Bench, NodeReference, NodeType, Store
from bench.language.connection import GraphEngine
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


def global_store_from_env() -> Store:
    """Get the default global store configured in the environment"""
    host = get_from_env("GLOBAL_PG_HOST", description="Global Postgres host")
    name = get_from_env("GLOBAL_PG_NAME", description="Global Postgres database name")
    username = get_from_env("GLOBAL_PG_USERNAME", description="Global Postgres username")
    password = get_from_env("GLOBAL_PG_PASSWORD", description="Global Postgres password")
    encryption_key = get_from_env("GLOBAL_PG_CRYPTO_KEY", description="Global encryption key")

    system_bench_ptr = NodeReference(node_type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = NodeSuperGraph(root_ptr=system_bench_ptr)
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        region=Region.ZURICH,
        encryption_key=encryption_key,
        _supergraph=supergraph,
        # set timestamps to avoid drawing from oracle (which we don't have here)
        # (also these are technically 'eternal' nodes anyway)
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    store = Store(
        parent=system_bench_stub,
        name="Global Store",
        version=VERSION,
        external_name=name,
        connection_uri=f"postgresql://{username}:{password}@{host}/{name}",
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    return store


def regional_store_from_env(region: Region = REGION) -> Store:  # nocheckin
    """Get the default regional store configured in the environment"""
    host = get_from_env("REGIONAL_PG_HOST", description="Regional Postgres host")
    name = get_from_env("REGIONAL_PG_NAME", description="Regional Postgres database name")
    username = get_from_env("REGIONAL_PG_USERNAME", description="Regional Postgres username")
    password = get_from_env("REGIONAL_PG_PASSWORD", description="Regional Postgres password")
    encryption_key = get_from_env("REGIONAL_PG_CRYPTO_KEY", description="Regional encryption key")

    system_bench_ptr = NodeReference(node_type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = NodeSuperGraph(root_ptr=system_bench_ptr)
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        region=Region.ZURICH,
        encryption_key=encryption_key,
        _supergraph=supergraph,
        # set timestamps to avoid drawing from oracle (which we don't have here)
        # (also these are technically 'eternal' nodes anyway)
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    store = Store(
        parent=system_bench_stub,
        name="Regional Store",
        version=VERSION,
        external_name=name,
        connection_uri=f"postgresql://{username}:{password}@{host}/{name}",
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    return store


def pg_engine_from_store(
    store: Store,
    area: NodeArea | None,
    *,
    scope: GraphScopeData | None = None,
):
    """Get the postgres engine for a store"""
    bench = store.parent
    assert bench is not None, f"missing parent for {store!r}"
    return PostgresEngine(
        store=store,
        bench=bench,
        scope=scope or EMPTY_SCOPE_DATA,
        node_types=NODE_TYPES_BY_AREA[area] if area is not None else NODE_TYPES,
        context=BenchSqlContext(bench),
    )


def local_pg_engine_from_store(store: Store):
    """Get the postgres engine for a local store"""
    assert store.bench is not None, f"missing bench for {store!r}"
    return pg_engine_from_store(
        store, area=NodeArea.LOCAL, scope=GraphScope(bench_id=store.bench.id)._to_data()
    )


def global_session(
    node: Node,
    engines: tuple[GraphEngine, ...],
    oracle: Oracle,
    *,
    supergraph: NodeSuperGraph | None = None,
    epoch: int | None = None,
    readonly: bool = False,
    split_read: bool = True,
):
    """Create a Session in a global store"""
    return Session(
        parent=None,
        _default_scope=EMPTY_SCOPE_DATA,
        _engines=engines,
        _local_epoch=epoch,
        _supergraph=supergraph or node._supergraph.instance(),
        _oracle=oracle,
        _is_readonly=readonly,
        _split_read=split_read,
    )
