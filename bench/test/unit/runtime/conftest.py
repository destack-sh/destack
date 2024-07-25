import pytest

from bench.language import Bench, Session, Store
from bench.language.bench import Client
from bench.language.block import Block
from bench.language.connection import RemoteEngine
from bench.language.const import (
    BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    BlockType,
    ClientType,
    Region,
    UserStatus,
    _active_session,
)
from bench.language.graph import NodeSuperGraph
from bench.language.node import EMPTY_SCOPE, GraphScope
from bench.language.user import User
from bench.proto.wire import HostClient, RpcMetadata
from bench.runtime.runner import RuntimeRunner
from bench.system.core import pg_engine_from_store
from bench.system.host import Host
from bench.system.supervisor import create_default_bench
from bench.test.simulation.transport import SimulatedChannel
from bench.utils.oracle import REAL_ORACLE, Oracle
from bench.utils.tenacity import RETRY_GRPC_FOREVER


def create_global_session(global_store: Store, oracle: Oracle):
    """Gets direct access to a per test global engine"""

    global_pg_engine = pg_engine_from_store(global_store)
    session = Session(
        parent=None,
        _default_scope=EMPTY_SCOPE._to_data(),
        _engines=(global_pg_engine,),
        _system_epoch=0,
        _oracle=oracle,
        _supergraph=NodeSuperGraph(root_ptr=None),
    )
    return session


#
# Omni/local session
#


@pytest.fixture()
async def omni_bench_async(global_store: Store, request: pytest.FixtureRequest):
    async with create_global_session(global_store, REAL_ORACLE) as session:
        # setup user/client
        user = User(
            slug="user",
            name="test",
            email="test@test.com",
            status=UserStatus.REGISTERED,
            last_logged_in_at=REAL_ORACLE.utc(),
            is_staff=True,
            _is_new=True,  # force create
        )
        session._create(user)
        await session.flush()
        client = Client(
            parent=user,
            name="test",
            type=ClientType.BENCH_SERVER,
            seen_at=REAL_ORACLE.utc(),
        )
        session._create(client)
        await session.flush()
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
        session.parent = bench.main_package  # patch in main package
        yield bench


@pytest.fixture()
def omni_bench(omni_bench_async: Bench):
    session = omni_bench_async.active_session
    active_session_token = _active_session.set(session)
    yield omni_bench_async
    _active_session.reset(active_session_token)


@pytest.fixture()
def session(omni_bench: Bench):
    return omni_bench.active_session


@pytest.fixture()
def page(omni_bench: Bench):
    package = omni_bench.main_package
    page = Block.new(BlockType.PAGE, name="Page")
    package.blocks.append(page)
    return page


@pytest.fixture()
def runner(omni_bench: Bench):
    runner = RuntimeRunner(session=omni_bench.active_session, oracle=REAL_ORACLE)
    return runner


#
# Real/remote session
#


@pytest.fixture()
async def bench(global_store: Store):
    async with create_global_session(global_store, REAL_ORACLE) as session:
        user = User(
            slug="test",
            name="Test",
            email="test@symbolx.com",
            status=UserStatus.REGISTERED,
            last_logged_in_at=REAL_ORACLE.utc(),
            is_staff=True,
            _is_new=True,  # force create
        )
        session._create(user)
        await session.flush()
        client = Client(
            parent=user,
            name="Test",
            type=ClientType.BENCH_WEB,
            seen_at=REAL_ORACLE.utc(),
            access_token="test",
            _is_new=True,  # force create
        )
        session._create(client)
        await session.flush()
        user.main_handle = user.handles.create(slug=user.slug)

        bench = await create_default_bench(
            main_handle=user.main_handle,
            owner=user,
            region=Region.ZURICH,
            global_store=global_store,
            session=session,
        )
        await session.commit()
    return bench


@pytest.fixture()
async def host_service(global_store: Store, bench: Bench):
    host = Host(bench_id=bench.id, global_store=global_store, oracle=REAL_ORACLE)
    await host.start()
    try:
        yield host
    finally:
        host.close()
        await host.wait_closed()


@pytest.fixture()
async def host(host_service: Host):
    async with SimulatedChannel(services=(host_service,), oracle=REAL_ORACLE) as channel:
        yield HostClient(channel)


@pytest.fixture()
async def real_session_async(bench: Bench, host: HostClient):
    user = bench.owner
    assert isinstance(user, User), f"unexpected bench owner: {user!r}"
    client = user.clients[0]
    client_data = client._to_data()
    rpc_metadata = RpcMetadata(
        client_type=client_data.type,
        client_id=client_data.id,
        client_nonce=client_data.id,
        client_access_token="test",
    )
    engines = (
        RemoteEngine(
            scope=GraphScope(bench_id=bench.id)._to_data(),
            node_types=BENCH_NODE_TYPES | IN_PACKAGE_NODE_TYPES,
            remote=host,
            write_retry=RETRY_GRPC_FOREVER,
            rpc_metadata=rpc_metadata,
        ),
    )
    session = Session(
        parent=bench.main_package,
        user=user,
        client=client,
        _is_readonly=False,
        _default_scope=GraphScope(bench_id=bench.id)._to_data(),
        _engines=engines,
        _system_epoch=0,
        _supergraph=bench._supergraph,
        _split_read=True,
        _oracle=REAL_ORACLE,
        _subject=user,
        _host=host,
        _origin=client.to_origin(nonce=None)._to_data(),
    )
    async with session:
        yield session


@pytest.fixture()
def real_session(real_session_async: Session):
    active_session_token = _active_session.set(real_session_async)
    yield real_session_async
    _active_session.reset(active_session_token)
