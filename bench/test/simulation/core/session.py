from typing import TYPE_CHECKING, assert_never
from uuid import UUID

from bench.language import (
    BENCH_ID,
    BENCH_NODE_TYPES,
    BENCH_SLUG,
    EMPTY_SCOPE_DATA,
    PUBLIC_NODE_TYPES,
    Client,
    Computer,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    RemoteEngine,
    Scope,
    Session,
    User,
)
from bench.proto.wiring import unpack_builtin_object
from bench.utils.oracle import Oracle
from bench.utils.tenacity import RETRY_GRPC_FOREVER

if TYPE_CHECKING:
    from .client import ClientHandle
    from .host import HostHandle
    from .simulation import Simulation


def make_pg_session(
    simulation: "Simulation",
    supergraph: NodeSuperGraph | None = None,
):
    """Create a Session to the global and regional Postgres"""
    if supergraph is None:
        root_ptr = NodeReference(node_type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
        supergraph = NodeSuperGraph(name="Global", root_ptr=root_ptr)
    session = Session(
        _default_scope=EMPTY_SCOPE_DATA,
        _engines=(simulation.global_pg_engine, simulation.regional_pg_engine),
        _oracle=simulation.oracle,
        _supergraph=supergraph,
        _split_read=True,
    )
    return session


async def make_remote_session(
    simulation: "Simulation",
    source_id: str,
    bench_id: UUID,
    client: "ClientHandle",
    host: "HostHandle",
    oracle: Oracle,
    supergraph: NodeSuperGraph | None = None,
    system: bool = False,
):
    """Create a Session to a remote Bench's Host"""
    from .computer import ComputerHandle
    from .user import UserHandle

    nonce = str(UUID(int=oracle.random.getrandbits(128)))
    supervisor_client = await simulation.supervisor.connect(source_id)
    self_host_client = await host.connect(source_id)
    engines = [
        # global engine
        RemoteEngine(
            name="remote-global",
            scope=EMPTY_SCOPE_DATA,
            node_types=PUBLIC_NODE_TYPES,
            remote=supervisor_client,
            write_retry=RETRY_GRPC_FOREVER,
            rpc_metadata=client.rpc_metadata,
        ),
        # bench engine
        RemoteEngine(
            name="remote-self-bench",
            scope=Scope(bench_id=bench_id)._to_data(),
            node_types=BENCH_NODE_TYPES,
            remote=self_host_client,
            write_retry=RETRY_GRPC_FOREVER,
            rpc_metadata=client.rpc_metadata,
        ),
    ]
    if system:
        bench_host = simulation.get_host(BENCH_SLUG)
        bench_host_client = await bench_host.connect(source_id)
        engines.append(
            RemoteEngine(
                name="remote-bench-bench",
                scope=Scope(bench_id=BENCH_ID)._to_data(),
                node_types=BENCH_NODE_TYPES,
                remote=bench_host_client,
                write_retry=RETRY_GRPC_FOREVER,
                rpc_metadata=client.rpc_metadata,
            )
        )
    if supergraph is None:
        root_ptr = NodeReference(node_type=NodeType.BENCH, id=bench_id, ck=bench_id)
        supergraph = NodeSuperGraph(name="Remote", root_ptr=root_ptr)
    session = Session(
        _default_scope=Scope(bench_id=bench_id)._to_data(),
        _engines=tuple(engines),
        _origin=client.to_origin(nonce=nonce),
        _supervisor=supervisor_client,
        _self_host=self_host_client,
        _oracle=oracle,
        _supergraph=supergraph,
    )
    if isinstance(client.parent, UserHandle):
        session.user = unpack_builtin_object(
            client.parent.user_data,
            session=session,
            supergraph=supergraph,
            expect=User,
            skip_add_self=False,
        )
        session._subject = session.user
    elif isinstance(client.parent, ComputerHandle):
        session.computer = unpack_builtin_object(
            client.parent.computer_data,
            session=session,
            supergraph=supergraph,
            expect=Computer,
            skip_add_self=False,
        )
        session._subject = session.computer
    else:
        assert_never(client.parent)
    session.client = unpack_builtin_object(
        client.client_data,
        session=session,
        supergraph=supergraph,
        expect=Client,
        skip_add_self=False,
    )
    return session
