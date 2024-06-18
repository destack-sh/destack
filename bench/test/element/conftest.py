# ruff: noqa: E402

import pytest

from bench.test.conftest import setup_test_env

# NOTE: must run setup_test() before importing from bench
setup_test_env()

from bench.language import Session
from bench.language.bench import Bench, ServerProfile
from bench.language.connection import NullEngine
from bench.language.const import NODE_TYPES, UserStatus, _active_session
from bench.language.graph import NodeSuperGraph
from bench.language.user import User
from bench.proto.wire import GraphScope


@pytest.fixture()
async def session_async(request):
    supergraph = NodeSuperGraph(root_ptr=None)
    user = User(
        status=UserStatus.REGISTERED,
        slug=f"test-{request.node.name}",
        email=f"test-{request.node.name}@symbolx.com",
        name=request.node.name,
        _supergraph=supergraph,
    )
    supergraph._root_ptr = user.to_ref()
    session = Session(
        _supergraph=user._supergraph,
        _graph=user._graph,
        _engines=(NullEngine(scope=GraphScope(), node_types=NODE_TYPES),),
    )
    async with session:
        yield session


@pytest.fixture()
def session(session_async: Session):
    # manually set session context since it's not propagated across tasks right now
    # https://github.com/pytest-dev/pytest-asyncio/issues/127#issuecomment-862817549
    active_session_token = _active_session.set(session_async)
    yield session_async
    _active_session.reset(active_session_token)


@pytest.fixture()
def package(session: Session):
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
