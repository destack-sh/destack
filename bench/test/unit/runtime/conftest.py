import pytest

from bench.language import Bench, Session, Store
from bench.language.bench import Client
from bench.language.block import Block
from bench.language.const import (
    BlockType,
    ClientType,
    Region,
    UserStatus,
    _active_session,
)
from bench.language.graph import NodeSuperGraph
from bench.language.node import EMPTY_SCOPE
from bench.language.user import User
from bench.runtime.runner import RuntimeRunner
from bench.system.core import pg_engine_from_store
from bench.system.supervisor import create_default_bench
from bench.utils.oracle import REAL_ORACLE, Oracle


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


@pytest.fixture()
def global_session(global_store: Store):
    return create_global_session(global_store, REAL_ORACLE)


@pytest.fixture()
async def omni_bench_async(global_store: Store, global_session, request: pytest.FixtureRequest):
    async with global_session as session:
        # setup user/client
        user = User(
            slug="user",
            name="test",
            email="test@test.com",
            status=UserStatus.REGISTERED,
            last_logged_in_at=REAL_ORACLE.utc(),
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
        global_session.parent = bench.main_package  # patch in main package
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
