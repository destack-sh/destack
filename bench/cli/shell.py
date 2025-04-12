from typing import Any

import structlog
import typer

from bench.pb2 import GraphScopeData, HostClient, SupervisorClient
from bench.proto import get_rpc_metadata
from bench.proto.network import RealNetwork
from bench.utils.oracle import REAL_ORACLE
from bench.utils.tenacity import RETRY_GRPC_FOREVER
from bench.utils.utils import get_from_env

from .utils import async_to_sync, repl

app = typer.Typer(short_help="postgres management")
logger = structlog.get_logger(__name__)

_global_exec = exec


@app.callback(invoke_without_command=True)
@app.command()
@async_to_sync
async def shell(
    bench: str = typer.Option(..., help="Bench slug"),
    user: str | None = typer.Option(None, help="User slug"),
) -> None:  # type: ignore
    """Open a runtime-like shell to a Bench."""
    from bench import pb2
    from bench.language import (
        BENCH_NODE_TYPES,
        EMPTY_SCOPE_DATA,
        PUBLIC_NODE_TYPES,
        RESOURCE_NODE_TYPES,
        SOURCE_NODE_TYPES,
        Bench,
        Client,
        Context,
        GraphScope,
        NodeArea,
        NodeType,
        Package,
        RemoteEngine,
        Session,
        User,
    )
    from bench.runtime.code import STATIC_CODE_GLOBALS
    from bench.system import global_session, global_store_from_env, pg_engine_from_store

    supervisor_url = get_from_env("SUPERVISOR_URL")
    network = RealNetwork()
    supervisor = SupervisorClient(await network.get_channel(supervisor_url, source_id="shell"))
    host = HostClient(await network.get_channel(supervisor_url, source_id="shell"))

    # load global Bench
    global_store = global_store_from_env()
    global_pg_engine = pg_engine_from_store("pg-global", global_store, NodeArea.GLOBAL)
    async with global_session(global_store, (global_pg_engine,), REAL_ORACLE) as session:
        bench_node = await Bench.get(slug=bench)
        if user is not None:
            user_node = await User.get(slug=user)
        else:
            user_node = await User.get(id=bench_node.owned_by_id)

        client = await Client.select_all().where(parent=user_node).one_or_none()
        assert client is not None, f"no client for {user_node!r}"
        assert client.access_token, f"no access token for {client!r}"
        rpc_metadata = get_rpc_metadata(
            client_type=client.type,
            client_id=client.id,
            client_access_token=client.access_token,
        )

    # prepare real remote session
    supergraph = session._supergraph
    supergraph._root_ptr = bench_node.to_ref()
    bench_scope = GraphScopeData(
        metatype=pb2.ObjectType.OBJECT_TYPE_GRAPH_SCOPE, bench_id=str(bench_node.id)
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
        _default_scope=GraphScope(bench_id=bench_node.id)._to_data(),
        _engines=remote_engines,
        _supervisor=supervisor,
        _self_host=host,
        _supergraph=supergraph,
        _oracle=REAL_ORACLE,
    )
    async with session:
        # load full Bench/Packages
        bench_node = (
            await Bench.include_descendants(NodeType.HANDLE, NodeType.PACKAGE, *RESOURCE_NODE_TYPES)
            .select_all()
            .get(bench_node.to_ref(), mode="both")
        )
        assert bench_node.main_store is not None, f"{bench_node!r} has no main store"
        session.parent = bench_node  # patch in bench for pg context
        session._default_scope = GraphScope(bench_id=bench_node.id)._to_data()
        main_package = await (
            Package.include_ancestors(Bench).include_descendants(*SOURCE_NODE_TYPES).select_all()
        ).get(bench_node.main_package_ptr, mode="both")
        # reload User in session
        session.user = await User.get(id=user_node.id)
        session.client = client
        session._subject = session.user
        session._origin = client.to_origin(nonce=None)._to_data()

        # prepare repl context :CodeGlobals
        context = Context()
        glbls: dict[str, Any] = {
            **STATIC_CODE_GLOBALS,
            "session": session,
            "bench": bench_node,
            "package": main_package,
            "user": session.user,
        }

        # enter repl
        banner = "=" * 40 + f"\nBench: {bench_node.slug}\n" + f"User: {user_node.slug}\n" + "=" * 40
        vars = globals()
        vars.update(locals())
        vars.update(glbls)
        await repl(banner, vars)
