# ruff: noqa: E402

import warnings
from typing import Mapping

import pytest
import uvloop

from bench.language import clean_name
from bench.runtime.core import Runtime
from bench.test.conftest import _setup_test_env

# NOTE: must run setup before importing from bench
_setup_test_env()

from dataclasses import dataclass
from typing import Any

from bench.language import (
    ACTIVE_SESSION,
    EMPTY_SCOPE_DATA,
    NODE_TYPES,
    OBJECT_TYPES,
    STRUCT_TYPES,
    Bench,
    Block,
    BlockType,
    BuiltinObject,
    NodeGraph,
    NodeMode,
    NodeSuperGraph,
    NullEngine,
    ObjectType,
    Package,
    PackageType,
    Region,
    Run,
    RunnableNode,
    Session,
    Store,
    User,
    UserStatus,
)
from bench.runtime import Runner
from bench.system import pg_engine_from_store
from bench.test.strategies import draw_direct, from_object_type
from bench.utils.oracle import REAL_ORACLE, Oracle


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


@dataclass(slots=True)
class RuntimeHandle:
    """All the stuff you need to do and run inside a Runtime."""

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


@pytest.fixture
def hosted_runtime(hosted_runtime_async: RuntimeHandle):  # :PytestAsyncContext
    ACTIVE_SESSION.set(hosted_runtime_async.session)
    yield hosted_runtime_async
    ACTIVE_SESSION.set(None)
