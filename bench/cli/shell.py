from uuid import UUID

import structlog
import typer

from bench.cli.utils import async_to_sync_blocking
from bench.language import Bench
from bench.language.connection import RemoteEngine
from bench.language.const import (
    BENCH_NODE_TYPES,
    PUBLIC_NODE_TYPES,
    STATIC_RESOURCE_NODE_TYPES,
    ClientType,
    NodeArea,
    NodeType,
)
from bench.language.graph import NodeSuperGraph
from bench.language.node import EMPTY_SCOPE_DATA, GraphScope
from bench.language.session import Session
from bench.proto import wire
from bench.proto.services import get_channel, get_rpc_metadata
from bench.proto.wire.lang_pb2 import GraphScopeData
from bench.proto.wire.system_grpc import HostClient, SupervisorClient
from bench.utils.oracle import REAL_ORACLE
from bench.utils.tenacity import RETRY_GRPC_FOREVER
from bench.utils.utils import get_from_env

app = typer.Typer(short_help="postgres management")
logger = structlog.get_logger(__name__)

_global_exec = exec


@app.callback(invoke_without_command=True)
@app.command()
@async_to_sync_blocking
async def local(bench_slug: str, user: str | None = None):  # type: ignore
    """Open a runtime-like shell to a Bench."""
    from bench.system.utils.session import (
        global_session,
        global_store_from_env,
        pg_engine_from_store,
    )

    # nocheckin: just pick a client?
    supervisor_url = get_from_env("SUPERVISOR_URL")
    client_type = get_from_env("CLIENT_TYPE", typ=ClientType)
    client_id = get_from_env("CLIENT_ID", typ=UUID)
    client_access_token = get_from_env("CLIENT_ACCESS_TOKEN")
    rpc_metadata = get_rpc_metadata(
        client_type=client_type,
        client_id=client_id,
        client_access_token=client_access_token,
    )
    supervisor = SupervisorClient(get_channel(supervisor_url))
    host = HostClient(get_channel(supervisor_url))

    # load global Bench
    global_store = global_store_from_env()
    global_pg_engine = pg_engine_from_store("pg-global", global_store, NodeArea.GLOBAL)
    async with global_session(global_store, (global_pg_engine,), REAL_ORACLE) as session:
        bench = await Bench.get(slug=bench_slug)

    # prepare real remote session
    supergraph = NodeSuperGraph(bench.to_ref())
    bench_scope = GraphScopeData(
        metatype=wire.ObjectType.OBJECT_TYPE_GRAPH_SCOPE, bench_id=str(bench.id)
    )
    remote_engines = (
        # global engine
        RemoteEngine(
            name="global",
            scope=EMPTY_SCOPE_DATA,
            node_types=PUBLIC_NODE_TYPES,
            remote=supervisor,
            write_retry=RETRY_GRPC_FOREVER,
            rpc_metadata=rpc_metadata,
        ),
        # bench engine
        RemoteEngine(
            name="bench",
            scope=bench_scope,
            node_types=BENCH_NODE_TYPES,
            remote=host,
            write_retry=RETRY_GRPC_FOREVER,
            rpc_metadata=rpc_metadata,
        ),
    )
    session = Session(
        _is_readonly=False,
        _default_scope=GraphScope(bench_id=bench.id)._to_data(),
        _engines=remote_engines,
        _supervisor=supervisor,
        _host=host,
        _supergraph=supergraph,
        _oracle=REAL_ORACLE,
    )
    async with session:
        # load full bench
        bench = (
            await Bench.include_descendants(
                NodeType.HANDLE, NodeType.PACKAGE, *STATIC_RESOURCE_NODE_TYPES
            )
            .select_all()
            .get(bench.to_ref(), mode="both")
        )
        assert bench.main_store, f"{bench_slug!r} has no main store"
        session.parent = bench  # patch in bench for pg context
        session._default_scope = GraphScope(bench_id=bench.id)._to_data()

        # enter a repl, commit on
        glbls = {"bench": bench_slug, "session": session}

        # execute and commit code in an eval loop from stdin
        # await session.commit()
