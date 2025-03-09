from typing import TYPE_CHECKING, assert_never
from uuid import UUID

from bench.language import (
    BENCH_NODE_TYPES,
    EMPTY_SCOPE_DATA,
    Client,
    GraphScope,
    Machine,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    RemoteEngine,
    Session,
    User,
)
from bench.language.core.const import PUBLIC_NODE_TYPES
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
):
    """Create a Session to a remote Bench's Host"""
    from .machine import MachineHandle
    from .user import UserHandle

    nonce = str(UUID(int=oracle.random.getrandbits(128)))
    supervisor_client = await simulation.supervisor.connect(source_id)
    host_client = await host.connect(source_id)
    engines = (
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
            name="remote-bench",
            scope=GraphScope(bench_id=bench_id)._to_data(),
            node_types=BENCH_NODE_TYPES,
            remote=host_client,
            write_retry=RETRY_GRPC_FOREVER,
            rpc_metadata=client.rpc_metadata,
        ),
    )
    if supergraph is None:
        root_ptr = NodeReference(node_type=NodeType.BENCH, id=bench_id, ck=bench_id)
        supergraph = NodeSuperGraph(name="Remote", root_ptr=root_ptr)
    session = Session(
        _default_scope=GraphScope(bench_id=bench_id)._to_data(),
        _engines=engines,
        _origin=client.to_origin(nonce=nonce),
        _supervisor=supervisor_client,
        _host=host_client,
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
    elif isinstance(client.parent, MachineHandle):
        session.machine = unpack_builtin_object(
            client.parent.machine_data,
            session=session,
            supergraph=supergraph,
            expect=Machine,
            skip_add_self=False,
        )
        session._subject = session.machine
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
