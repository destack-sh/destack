# ruff: noqa: E402

import warnings
from typing import Mapping

import pytest
import uvloop

from bench.language.machine import Machine
from bench.language.validation import clean_name
from bench.runtime.cache import MemoryCache
from bench.runtime.remote import RemoteEngine
from bench.runtime.runtime import Runtime
from bench.sql.client import pg_connection
from bench.sql.engine import sqlstr
from bench.test.conftest import _setup_test_env
from bench.test.fixtures import delete_test_db

# NOTE: must run setup before importing from bench
_setup_test_env()

from dataclasses import dataclass
from typing import Any

from bench.language import Session, Store
from bench.language.bench import Bench, Client, Package, ResourceStatus
from bench.language.block import Block
from bench.language.connection import NullEngine
from bench.language.const import (
    BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    NODE_TYPES,
    OBJECT_TYPES,
    STRUCT_TYPES,
    BlockType,
    ClientType,
    NodeMode,
    NodeType,
    ObjectType,
    Region,
    UserStatus,
    _active_session,
)
from bench.language.graph import NodeGraph, NodeSuperGraph
from bench.language.node import EMPTY_SCOPE_DATA, BuiltinObject, GraphScope
from bench.language.run import Run, RunnableNode
from bench.language.user import User
from bench.proto.wire import HostClient, RpcMetadata
from bench.runtime.runner import Runner
from bench.system.host.service import HostService
from bench.system.supervisor.service import create_default_bench
from bench.system.utils.session import pg_engine_from_store
from bench.test.simulation.transport import SimulatedChannel
from bench.test.strategies import draw_direct, from_object_type
from bench.utils.oracle import REAL_ORACLE, Oracle
from bench.utils.tenacity import RETRY_GRPC_FOREVER


# NOTE: unit tests are run in a shared event loop
@pytest.fixture(scope="session")  # scope=function!
def event_loop_policy():
    return uvloop.EventLoopPolicy()


def create_omni_session(omni_store: Store, oracle: Oracle):
    """Gets direct access to a per test global engine"""

    global_pg_engine = pg_engine_from_store(omni_store, node_types=NODE_TYPES)
    session = Session(
        parent=None,
        _default_scope=EMPTY_SCOPE_DATA,
        _engines=(global_pg_engine,),
        _local_epoch=0,
        _oracle=oracle,
        _supergraph=NodeSuperGraph(root_ptr=None),
    )
    return session


@pytest.fixture
def omni_session(omni_store: Store):
    """
    Gets the per test function global real session.
    Unfortunately we can't set this session as the active session in context because
     pytest-asyncio does not propagate contextvars across async tests/fixtures.
    (see https://github.com/pytest-dev/pytest-asyncio/issues/127#issuecomment-1777004844)
    """

    return create_omni_session(omni_store, REAL_ORACLE)


def make_session(name: str):
    """Make a 'fake' session for context"""
    supergraph = NodeSuperGraph(root_ptr=None)
    graph = NodeGraph(scope=EMPTY_SCOPE_DATA, node_types=NODE_TYPES, supergraph=supergraph)
    session = Session(
        _engines=(NullEngine(scope=EMPTY_SCOPE_DATA, node_types=NODE_TYPES),),
        _supergraph=supergraph,
        _graph=graph,
        _oracle=REAL_ORACLE,
    )
    user = User(
        status=UserStatus.REGISTERED,
        slug="test",
        email="test@symbolx.com",
        name=name,
        _graph=graph,
        _supergraph=supergraph,
        _session=session,
    )
    supergraph._root_ptr = user.to_ref()
    return session


# NOTE :Cleanup: manually set session sync context since it's not propagated across pytest tasks (see above)
# https://github.com/pytest-dev/pytest-asyncio/issues/127#issuecomment-862817549 :PytestAsyncContext


@pytest.fixture  # :PytestAsyncContext
async def session_async(request):
    session = make_session(clean_name(request.node.name))
    await session.open(set_in_context=False)
    yield session
    await session.close()


def make_package(session: Session):
    bench = Bench(name="test", slug="test")
    bench.main_server = bench.servers.create(name="Server")
    bench.main_store = bench.stores.create(name="Store")
    bench.main_drive = bench.drives.create(name="Drive")
    branch = bench.branches.create(name="Branch")
    package = branch.packages.create()
    session.parent = package
    session._graph.update(session, _force_update_parent=True)
    return package


@pytest.fixture
def session(session_async: Session):
    active_session_token = _active_session.set(session_async)
    yield session_async
    _active_session.reset(active_session_token)


@pytest.fixture
def package(session: Session):
    package = make_package(session)
    return package


SHARED_SESSION = make_session("shared")


@pytest.fixture(scope="session")
async def shared_session_async():
    await SHARED_SESSION.open(set_in_context=False)
    yield SHARED_SESSION
    await SHARED_SESSION.close()


@pytest.fixture(scope="session")
def shared_session(shared_session_async: Session):
    active_session_token = _active_session.set(shared_session_async)
    yield shared_session_async
    _active_session.reset(active_session_token)


@pytest.fixture(scope="session")
def shared_package(shared_session: Session):
    package = make_package(shared_session)
    return package


# init shared builtin objects (in shared session)
with warnings.catch_warnings(action="ignore"):
    _active_session_token = _active_session.set(SHARED_SESSION)
    BUILTIN_OBJECTS = [
        draw_direct(from_object_type(object_type, reject_invalid=False))
        for object_type in OBJECT_TYPES
    ]
    _active_session.reset(_active_session_token)

BUILTIN_OBJECTS_BY_TYPE: Mapping[ObjectType, BuiltinObject] = {
    obj.metatype: obj for obj in BUILTIN_OBJECTS
}
STRUCTS = [BUILTIN_OBJECTS_BY_TYPE[t] for t in STRUCT_TYPES]
NODES = [BUILTIN_OBJECTS_BY_TYPE[t] for t in NODE_TYPES]


def create_global_session(global_store: Store, oracle: Oracle):
    """Gets direct access to a per test global engine"""

    global_pg_engine = pg_engine_from_store(global_store)
    session = Session(
        parent=None,
        _default_scope=EMPTY_SCOPE_DATA,
        _engines=(global_pg_engine,),
        _local_epoch=0,
        _oracle=oracle,
        _supergraph=NodeSuperGraph(root_ptr=None),
    )
    return session


@dataclass(slots=True)
class RuntimeHandle:
    """All the stuff you need to do and run inside a Runtime."""

    supergraph: NodeSuperGraph
    user: User
    client: Client
    bench: Bench
    package: Package
    session: Session
    runtime: Runtime

    def page(self, name: str = "Page1") -> Block:
        """Gets or creates a page in the current package."""
        page = self.bench.main_package.blocks.get(name)
        if page is None:
            page = Block.new(BlockType.PAGE, name=name)
            self.bench.main_package.blocks.append(page)
        return page

    async def commit(self):
        """Commits the current session."""
        return await self.session.commit()

    async def run(
        self,
        run: Run | RunnableNode,
        *,
        inputs: Any | None = None,
        mode: NodeMode | None = None,
        return_error: bool = False,
    ) -> Runner:
        runner = await self.runtime.run(run, inputs=inputs, mode=mode, return_error=return_error)
        assert runner is not None, f"no runner for {run!r}"
        return runner


#
# Omni/local session
#


@pytest.fixture
async def local_runtime_async(global_store: Store):
    async with create_global_session(global_store, REAL_ORACLE) as session:
        # setup user/client
        user = User(
            slug="test",
            name="test",
            email="test@test.com",
            status=UserStatus.REGISTERED,
            last_logged_in_at=REAL_ORACLE.utc(),
            is_staff=True,
            _is_new=True,  # force create
        )
        session._create(user)
        await session.flush(optimistic=True)
        client = Client(
            parent=user,
            title="test",
            type=ClientType.BENCH_MACHINE,
            seen_at=REAL_ORACLE.utc(),
        )
        session._create(client)
        await session.flush(optimistic=True)
        user.main_handle = user.handles.create(slug=user.slug)
        await session.commit()

        # create bench
        bench = await create_default_bench(
            main_handle=user.main_handle,
            owner=user,
            region=Region.ZURICH,
            global_store=global_store,
            session=session,
        )

        user._untrack_rec()
        bench._untrack_rec()

    session = Session(
        parent=bench.main_package,
        user=user,
        client=client,
        _is_readonly=False,
        _default_scope=GraphScope(bench_id=bench.id)._to_data(),
        _engines=session._engines,
        _local_epoch=0,
        _oracle=REAL_ORACLE,
        _supergraph=bench._supergraph,
    )
    runtime = Runtime(session=session, cache=MemoryCache(bench), oracle=REAL_ORACLE)
    handle = RuntimeHandle(
        supergraph=bench._supergraph,
        user=user,
        client=client,
        bench=bench,
        package=bench.main_package,
        session=session,
        runtime=runtime,
    )
    try:
        async with session:
            yield handle
    finally:
        # delete DBs
        for store in bench.stores:
            if store.status == ResourceStatus.UP:
                await delete_test_db(store)


@pytest.fixture
def local_runtime(local_runtime_async: RuntimeHandle):  # :PytestAsyncContext
    active_session_token = _active_session.set(local_runtime_async.session)
    yield local_runtime_async
    _active_session.reset(active_session_token)


#
# Real/remote session
#


@pytest.fixture
async def hosted_bench(global_store: Store):
    async with create_global_session(global_store, REAL_ORACLE) as session:
        user = User(
            slug="user",
            name="User",
            email="user@symbolx.com",
            status=UserStatus.REGISTERED,
            last_logged_in_at=REAL_ORACLE.utc(),
            is_staff=True,
            _is_new=True,  # force create
        )
        session._create(user)
        await session.flush(optimistic=True)
        user_client = Client(
            parent=user,
            title="User",
            type=ClientType.BENCH_WEB,
            seen_at=REAL_ORACLE.utc(),
            access_token="user",
            _is_new=True,  # force create
        )
        session._create(user_client)
        await session.flush(optimistic=True)
        user.main_handle = user.handles.create(slug=user.slug)

        bench = await create_default_bench(
            main_handle=user.main_handle,
            owner=user,
            region=Region.ZURICH,
            global_store=global_store,
            session=session,
        )
        server = bench.main_server
        assert server is not None, f"no main server for {bench!r}"
        server_client = Client(
            parent=server,
            title="Server",
            type=ClientType.BENCH_MACHINE,
            access_token="server",
            _is_new=True,  # force create
        )
        session._create(server_client)
        machine = Machine(parent=server, title="Machine1", cpu=0.5, ram=0.5, client=server_client)
        session._create(machine)
        await session.flush(optimistic=True)
        user.main_bench = bench
        await session.commit()

        bench._untrack_rec()
        user._untrack_rec()
        user_client._untrack_rec()

    return bench


@pytest.fixture
async def host_service(global_store: Store, hosted_bench: Bench):
    host = HostService(bench_id=hosted_bench.id, global_store=global_store, oracle=REAL_ORACLE)
    await host.start()
    try:
        yield host
    finally:
        host.close()
        await host.wait_closed()

        # manually decommission stores (bootstrapping problem since the Host session uses the store)
        async with pg_connection(host.global_store, autocommit=True) as conn:
            for store in hosted_bench.stores:
                await conn.execute(sqlstr(f'DROP DATABASE "{store.external_name}"'))


@pytest.fixture
async def host(host_service: HostService):
    async with SimulatedChannel(services=(host_service,), oracle=REAL_ORACLE) as channel:
        yield HostClient(channel)


@pytest.fixture
async def hosted_runtime_async(hosted_bench: Bench, host: HostClient):
    user = hosted_bench.owner
    assert isinstance(user, User), f"unexpected bench owner: {user!r}"
    server = hosted_bench.main_server
    assert server is not None, f"no main server for {hosted_bench!r}"
    machine = next(iter(server._graph.get_descendants(server, NodeType.MACHINE)))
    assert isinstance(machine, Machine), f"unexpected machine: {machine!r}"

    # get server client
    client = next(iter(server._graph.get_descendants(server, NodeType.CLIENT)))
    assert isinstance(client, Client), f"unexpected client: {client!r}"
    client_data = client._to_data()
    assert client_data.access_token, f"no access token for {client!r}"

    rpc_metadata = RpcMetadata(
        client_type=client_data.type,
        client_id=client_data.id,
        client_nonce=client_data.id,
        client_access_token=client_data.access_token,
    )
    engines = (
        RemoteEngine(
            scope=GraphScope(bench_id=hosted_bench.id)._to_data(),
            node_types=BENCH_NODE_TYPES | IN_PACKAGE_NODE_TYPES,
            remote=host,
            write_retry=RETRY_GRPC_FOREVER,
            rpc_metadata=rpc_metadata,
        ),
    )
    session = Session(
        parent=hosted_bench.main_package,
        server=server,
        machine=machine,
        client=client,
        _is_readonly=False,
        _default_scope=GraphScope(bench_id=hosted_bench.id)._to_data(),
        _engines=engines,
        _local_epoch=0,
        _supergraph=hosted_bench._supergraph,
        _split_read=True,
        _oracle=REAL_ORACLE,
        _subject=server,
        _host=host,
        _origin=client.to_origin(nonce=None)._to_data(),
    )
    session.track(hosted_bench)
    runner = Runtime(session=session, cache=MemoryCache(hosted_bench), oracle=REAL_ORACLE)
    handle = RuntimeHandle(
        supergraph=hosted_bench._supergraph,
        user=user,
        client=client,
        bench=hosted_bench,
        package=hosted_bench.main_package,
        session=session,
        runtime=runner,
    )
    async with session:
        yield handle


@pytest.fixture
def hosted_runtime(hosted_runtime_async: RuntimeHandle):  # :PytestAsyncContext
    active_session_token = _active_session.set(hosted_runtime_async.session)
    yield hosted_runtime_async
    _active_session.reset(active_session_token)
