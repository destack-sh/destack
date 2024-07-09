# ruff: noqa: E402

import warnings

import pytest
import uvloop

from bench.language.validation import clean_name
from bench.test.conftest import _setup_test_env

# NOTE: must run setup before importing from bench
_setup_test_env()

from bench.language import Session, Store
from bench.language.bench import Bench, ServerProfile
from bench.language.connection import NullEngine
from bench.language.const import NODE_TYPES, OBJECT_TYPES, UserStatus, _active_session
from bench.language.graph import NodeGraph, NodeSuperGraph
from bench.language.node import EMPTY_SCOPE
from bench.language.user import User
from bench.system.core import pg_engine_from_store
from bench.test.strategies import draw_direct, from_object_type
from bench.utils.oracle import REAL_ORACLE, Oracle


# NOTE: unit tests are run in a shared event loop
@pytest.fixture(scope="session")  # scope=function!
def event_loop_policy():
    return uvloop.EventLoopPolicy()


def create_omni_session(omni_store: Store, oracle: Oracle):
    """Gets direct access to a per test global engine"""

    global_pg_engine = pg_engine_from_store(omni_store, node_types=NODE_TYPES)
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
    graph = NodeGraph(scope=EMPTY_SCOPE._to_data(), node_types=NODE_TYPES, supergraph=supergraph)
    session = Session(
        _engines=(NullEngine(scope=EMPTY_SCOPE._to_data(), node_types=NODE_TYPES),),
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
# https://github.com/pytest-dev/pytest-asyncio/issues/127#issuecomment-862817549


@pytest.fixture()
async def session_async(request):
    session = make_session(clean_name(request.node.name))
    await session.open(set_in_context=False)
    yield session
    await session.close()


def make_package(session: Session):
    bench = Bench(name="test", slug="test")
    bench.main_server = bench.servers.create(name="Server", profile=ServerProfile.TINY)
    bench.main_store = bench.stores.create(name="Store")
    bench.main_drive = bench.drives.create(name="Drive")
    branch = bench.branches.create(name="Branch")
    package = branch.packages.create()
    session.parent = package
    session._graph.update(session, _force_update_parent=True)
    return package


@pytest.fixture()
def session(session_async: Session):
    active_session_token = _active_session.set(session_async)
    yield session_async
    _active_session.reset(active_session_token)


@pytest.fixture()
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
    BUILTIN_OBJECTS_OF_EVERY_TYPE = [
        draw_direct(from_object_type(object_type, reject_invalid=False))
        for object_type in OBJECT_TYPES
    ]
    _active_session.reset(_active_session_token)
