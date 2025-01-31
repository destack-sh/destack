# ruff: noqa: E402

import warnings
from typing import Mapping

import pytest
import uvloop

from bench.language import Machine, MachineType, clean_name
from bench.runtime.core import MemoryCache, Runtime
from bench.sql import pg_connection, sqlstr
from bench.test.conftest import _setup_test_env

# NOTE: must run setup before importing from bench
_setup_test_env()

from dataclasses import dataclass
from typing import Any

from bench.language import (
    ACTIVE_SESSION,
    BENCH_NODE_TYPES,
    EMPTY_SCOPE_DATA,
    NODE_TYPES,
    OBJECT_TYPES,
    STRUCT_TYPES,
    Bench,
    Block,
    BlockType,
    BuiltinObject,
    Client,
    ClientType,
    GraphScope,
    NodeArea,
    NodeGraph,
    NodeMode,
    NodeSuperGraph,
    NodeType,
    NullEngine,
    ObjectType,
    Package,
    PackageType,
    Region,
    RemoteEngine,
    Run,
    RunnableNode,
    Session,
    Store,
    User,
    UserStatus,
)
from bench.proto import HostClient, RpcMetadata
from bench.runtime import Runner
from bench.system import HostService, create_default_bench, pg_engine_from_store
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

    omni_pg_engine = pg_engine_from_store(name="pg-omni", store=omni_store, area=None)
    session = Session(
        parent=None,
        _default_scope=EMPTY_SCOPE_DATA,
        _engines=(omni_pg_engine,),
        _local_epoch=0,
        _oracle=oracle,
        _supergraph=NodeSuperGraph(name="Omni", root_ptr=None),
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
    supergraph = NodeSuperGraph(name=name, root_ptr=None)
    graph = NodeGraph(scope=EMPTY_SCOPE_DATA, node_types=NODE_TYPES, supergraph=supergraph)
    session = Session(
        _engines=(NullEngine(name="fake", scope=EMPTY_SCOPE_DATA, node_types=NODE_TYPES),),
        _supergraph=supergraph,
        _graph=graph,
        _oracle=REAL_ORACLE,
    )
    user = User(
        status=UserStatus.REGISTERED,
        region=Region.ZURICH,
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
    await session.open(_set_in_context=False)
    yield session
    await session.close()


def make_package(session: Session):
    bench = Bench(name="test", slug="test")
    bench.main_store = bench.stores.create(name="Store")
    package = bench.packages.create(type=PackageType.ROOT, name="Main", slug="main")
    session.parent = bench
    session._graph.update(session, _force_update_parent=True)
    return package


@pytest.fixture
def session(session_async: Session):
    ACTIVE_SESSION.set(session_async)
    yield session_async
    ACTIVE_SESSION.set(None)


@pytest.fixture
def package(session: Session):
    package = make_package(session)
    return package


SHARED_SESSION = make_session("shared")


# init shared builtin objects (in shared session)
with warnings.catch_warnings(action="ignore"):
    ACTIVE_SESSION.set(SHARED_SESSION)
    BUILTIN_OBJECTS = [
        draw_direct(from_object_type(object_type, reject_invalid=False))
        for object_type in OBJECT_TYPES
    ]
    ACTIVE_SESSION.set(None)

BUILTIN_OBJECTS_BY_TYPE: Mapping[ObjectType, BuiltinObject] = {
    obj.metatype: obj for obj in BUILTIN_OBJECTS
}
STRUCTS = [BUILTIN_OBJECTS_BY_TYPE[t] for t in STRUCT_TYPES if t in BUILTIN_OBJECTS_BY_TYPE]
NODES = [BUILTIN_OBJECTS_BY_TYPE[t] for t in NODE_TYPES if t in BUILTIN_OBJECTS_BY_TYPE]


def create_global_session(global_store: Store, regional_store: Store, oracle: Oracle):
    """Gets direct access to a per test global engine"""

    global_pg_engine = pg_engine_from_store(
        name="pg-global", store=global_store, area=NodeArea.GLOBAL
    )
    regional_pg_engine = pg_engine_from_store(
        name=f"pg-regional-{regional_store.region.name.lower()}",
        store=regional_store,
        area=NodeArea.REGIONAL,
    )
    session = Session(
        parent=None,
        _default_scope=EMPTY_SCOPE_DATA,
        _engines=(global_pg_engine, regional_pg_engine),
        _local_epoch=0,
        _oracle=oracle,
        _supergraph=NodeSuperGraph(name="Global", root_ptr=None),
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
        page = self.package.blocks.get(name)
        if page is None:
            page = Block.new(BlockType.PAGE, name=name)
            self.package.blocks.append(page)
        return page

    async def commit(self):
        """Commits the current session."""
        return await self.session.commit()

    async def run(
        self,
        run: Run | RunnableNode,
        *,
        variables: Any | None = None,
        inputs: Any | None = None,
        mode: NodeMode | None = None,
        return_error: bool = False,
    ) -> Runner:
        runner = await self.runtime.run(
            run, variables=variables, inputs=inputs, mode=mode, return_error=return_error
        )
        assert runner is not None, f"no runner for {run!r}"
        return runner


#
# Real/remote session
#


@pytest.fixture
async def hosted_bench(global_store: Store, regional_store: Store):
    ACTIVE_SESSION.set(None)
    async with create_global_session(global_store, regional_store, REAL_ORACLE) as session:
        user = User(
            slug="user",
            name="User",
            email="user@symbolx.com",
            status=UserStatus.REGISTERED,
            region=Region.ZURICH,
            last_logged_in_at=REAL_ORACLE.utc(),
            is_staff=True,
            _is_new=True,  # force create
        )
        session._create(user)
        await session.flush(optimistic=True)
        user_client = Client(
            parent=user,
            name="User",
            type=ClientType.WEB,
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
        server_client = Client(
            parent=bench,
            name="Server",
            type=ClientType.MACHINE,
            access_token="server",
            _is_new=True,  # force create
        )
        machine = Machine(
            type=MachineType.RUNTIME,
            parent=bench,
            name="Runtime1",
            cpu=0.5,
            ram=0.5,
            client=server_client,
        )
        session._create(machine)
        server_client.machine = machine
        session._create(server_client)
        await session.flush(optimistic=True)
        user.main_bench = bench
        await session.commit()

        bench._untrack_rec()
        user._untrack_rec()
        user_client._untrack_rec()

    return bench


@pytest.fixture
async def host_service(global_store: Store, regional_store: Store, hosted_bench: Bench):
    ACTIVE_SESSION.set(None)
    host = HostService(
        bench_id=hosted_bench.id,
        global_store=global_store,
        regional_store=regional_store,
        oracle=REAL_ORACLE,
    )
    await host.start()
    try:
        yield host
    finally:
        host.close()
        await host.wait_closed()

        # decommission resources
        # TODO :Test: decommission Bench resources after test

        # manually decommission stores
        # (afterwards the others since since the Host session uses the store)
        async with pg_connection(host.global_store, autocommit=True) as conn:
            for store in hosted_bench.stores:
                await conn.execute(sqlstr(f'DROP DATABASE "{store.external_name}"'))


@pytest.fixture
async def host_client(host_service: HostService):
    async with SimulatedChannel(services=(host_service,), oracle=REAL_ORACLE) as channel:
        yield HostClient(channel)


@pytest.fixture
async def hosted_runtime_async(hosted_bench: Bench, host_client: HostClient):
    user = hosted_bench.owner
    assert isinstance(user, User), f"unexpected bench owner: {user!r}"
    machine = next(iter(hosted_bench._graph.get_descendants(hosted_bench, NodeType.MACHINE)))
    assert isinstance(machine, Machine), f"unexpected machine: {machine!r}"

    # get server client
    client = next(iter(hosted_bench._graph.get_descendants(hosted_bench, NodeType.CLIENT)))
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
            name="remote-bench",
            scope=GraphScope(bench_id=hosted_bench.id)._to_data(),
            node_types=BENCH_NODE_TYPES,
            remote=host_client,
            write_retry=RETRY_GRPC_FOREVER,
            rpc_metadata=rpc_metadata,
        ),
    )
    session = Session(
        parent=hosted_bench,
        machine=machine,
        client=client,
        _is_readonly=False,
        _default_scope=GraphScope(bench_id=hosted_bench.id)._to_data(),
        _engines=engines,
        _local_epoch=0,
        _supergraph=hosted_bench._supergraph,
        _oracle=REAL_ORACLE,
        _host=host_client,
        _origin=client.to_origin(nonce=None)._to_data(),
        _subject=machine,
    )
    session._track(hosted_bench)
    runner = Runtime(session=session, cache=MemoryCache(hosted_bench), oracle=REAL_ORACLE)
    assert hosted_bench.main_package is not None, f"no main package for {hosted_bench!r}"
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
    ACTIVE_SESSION.set(hosted_runtime_async.session)
    yield hosted_runtime_async
    ACTIVE_SESSION.set(None)
