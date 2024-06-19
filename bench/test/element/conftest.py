# ruff: noqa: E402

import pytest

from bench.test.conftest import setup_test_env
from bench.utils.oracle import REAL_ORACLE

# NOTE: must run setup_test() before importing from bench
setup_test_env()

from bench.language import Session
from bench.language.bench import Bench, ServerProfile
from bench.language.connection import NullEngine
from bench.language.const import NODE_TYPES, UserStatus, _active_session
from bench.language.graph import NodeGraph, NodeSuperGraph
from bench.language.user import User
from bench.proto.wire import GraphScope


def make_session(name: str):
    """Make a 'fake' session for context"""
    supergraph = NodeSuperGraph(root_ptr=None)
    graph = NodeGraph(scope=GraphScope(), node_types=NODE_TYPES, supergraph=supergraph)
    session = Session(
        _engines=(NullEngine(scope=GraphScope(), node_types=NODE_TYPES),),
        _supergraph=supergraph,
        _graph=graph,
        _oracle=REAL_ORACLE,
    )
    user = User(
        status=UserStatus.REGISTERED,
        slug=f"test-{name}",
        email=f"test-{name}@symbolx.com",
        name=name,
        _graph=graph,
        _supergraph=supergraph,
        _session=session,
    )
    supergraph._root_ptr = user.to_ref()
    return session


# NOTE :Cleanup: manually set session sync context since it's not propagated across pytest tasks
# https://github.com/pytest-dev/pytest-asyncio/issues/127#issuecomment-862817549


@pytest.fixture()
async def session_async(request):
    session = make_session(request.node.name)
    await session.open(set_in_context=False)
    yield session
    await session.close()


@pytest.fixture(scope="module")
async def shared_session_async():
    session = make_session("shared")
    await session.open(set_in_context=False)
    yield session
    await session.close()


@pytest.fixture()
def session(session_async: Session):
    active_session_token = _active_session.set(session_async)
    yield session_async
    _active_session.reset(active_session_token)


@pytest.fixture(scope="module")
def shared_session(shared_session_async: Session):
    active_session_token = _active_session.set(shared_session_async)
    yield shared_session_async
    _active_session.reset(active_session_token)


def make_package(session: Session):
    bench = Bench(name="test", slug="test")
    server = bench.servers.create(name="Server", profile=ServerProfile.TINY)
    store = bench.stores.create(name="Store")
    drive = bench.drives.create(name="Drive")
    environment = bench.environments.create(
        name="Environment", server=server, store=store, drive=drive
    )
    branch = bench.branches.create(name="Branch")
    package = branch.packages.create(environment=environment)
    session.parent = package
    return package


@pytest.fixture()
def package(session: Session):
    package = make_package(session)
    return package


@pytest.fixture(scope="module")
def shared_package(shared_session: Session):
    package = make_package(shared_session)
    return package
