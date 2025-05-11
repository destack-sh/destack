from datetime import datetime
from uuid import UUID

import cachetools

from bench.language import (
    EMPTY_SCOPE_DATA,
    NODE_TYPES,
    NODE_TYPES_BY_AREA,
    REGION,
    VERSION,
    Bench,
    BenchStatus,
    Database,
    Engine,
    GraphScope,
    Node,
    NodeArea,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    Package,
    PackageType,
    Region,
    Session,
)
from bench.proto import GraphScopeData
from bench.sql import BenchSqlContext
from bench.system.graph.postgres import PostgresEngine
from bench.utils.oracle import Oracle
from bench.utils.utils import get_from_env

BEGINNING_OF_TIME = datetime.fromisoformat("1970-01-01T00:00:00+00:00")


def make_system_database(region: Region, pg_url: str) -> Database:
    system_bench_ptr = NodeReference(node_type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = NodeSuperGraph(name="Global", root_ptr=system_bench_ptr)
    system_bench_stub = Bench(
        id=UUID(int=0),
        name="System",
        slug="system",
        region=region,
        status=BenchStatus.ACTIVATED,
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    system_package_stub = Package(
        parent=system_bench_stub,
        type=PackageType.OPEN,
        id=UUID(int=1),
        name="Main",
        slug="main",
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    database = Database(
        parent=system_package_stub,
        name="Database",
        version=VERSION,
        sql_url=pg_url,
        _supergraph=supergraph,
        created_at=BEGINNING_OF_TIME,
        updated_at=BEGINNING_OF_TIME,
    )
    return database


@cachetools.cached(cache={})
def global_database_from_env() -> Database:
    """Get the default global database configured in the environment"""
    pg = get_from_env("GLOBAL_PG_URL", description="Global Postgres connection string")
    pg_url = pg.split("|", maxsplit=1)[0]
    return make_system_database(REGION, pg_url)


def regional_database_from_env(region: Region = REGION) -> Database:
    """Get the default regional database configured in the environment"""
    from bench.system.core import DATABASE_MAP

    return DATABASE_MAP.get(region)


def pg_engine_from_database(
    name: str,
    database: Database,
    area: NodeArea | None,
    *,
    scope: GraphScopeData | None = None,
):
    """Get the postgres engine for a database"""
    bench = database.bench
    assert bench is not None, f"missing bench for {database!r}"
    return PostgresEngine(
        name=name,
        database=database,
        bench=bench,
        scope=scope or EMPTY_SCOPE_DATA,
        node_types=NODE_TYPES_BY_AREA[area] if area is not None else NODE_TYPES,
        context=BenchSqlContext(bench),
    )


def local_pg_engine_from_database(name: str, database: Database):
    """Get the postgres engine for a local database"""
    assert database.bench is not None, f"missing bench for {database!r}"
    return pg_engine_from_database(
        name=name,
        database=database,
        area=NodeArea.LOCAL,
        scope=GraphScope(bench_id=database.bench.id)._to_data(),
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
    """Create a Session in a global database"""
    if node is None:
        assert supergraph is not None, "must provide supergraph if no node"
    else:
        supergraph = node._supergraph.instance(name="Global")
    return Session(
        parent=None,
        _default_scope=EMPTY_SCOPE_DATA,
        _engines=engines,
        _local_epoch=epoch,
        _supergraph=supergraph,
        _oracle=oracle,
        _split_read=split_read,
    )
