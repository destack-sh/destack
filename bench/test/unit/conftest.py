# ruff: noqa: E402

import functools
import gc
import inspect
import warnings
from typing import Awaitable, Callable, Mapping

import pytest

from bench.test.conftest import _setup_test_env

# NOTE: must run setup before importing from bench
_setup_test_env()


from bench.language import (
    ACTIVE_SESSION,
    EMPTY_SCOPE_DATA,
    NODE_TYPES,
    OBJECT_TYPES,
    STRUCT_TYPES,
    Bench,
    BuiltinObject,
    NodeGraph,
    NodeSuperGraph,
    NullEngine,
    ObjectType,
    PackageType,
    Region,
    Session,
    Store,
    User,
    UserStatus,
)
from bench.sql.graph import BUILTIN_GLOBAL_SCHEMA, BUILTIN_REGIONAL_SCHEMA
from bench.system import pg_engine_from_store
from bench.system.core.sharding import StoreMap
from bench.test.conftest import _setup_test_env
from bench.test.fixtures import (
    create_test_db,
    delete_test_db,
    make_global_store,
    make_regional_store,
)
from bench.test.simulation.core import (
    BenchSpec,
    ClientSpec,
    HostSpec,
    MachineSpec,
    NetworkSpec,
    SimulatedEventLoopPolicy,
    Simulation,
    SimulationSpec,
    SupervisorSpec,
    UserSpec,
    get_simulation_id,
)
from bench.test.simulation.workload import (
    RuntimeLambdaWorkload,
    RuntimeLambdaWorkloadSpec,
    WorkloadSpec,
    WorkloadType,
)
from bench.test.strategies import draw_direct, from_object_type
from bench.utils.oracle import REAL_ORACLE, Oracle


# NOTE: simulation tests must be run with one event loop per function to isolate
@pytest.fixture
def event_loop_policy():
    return SimulatedEventLoopPolicy()


def create_omni_session(omni_store: Store, oracle: Oracle):
    """Gets direct access to a per test global engine"""

    omni_pg_engine = pg_engine_from_store(name="pg-omni", store=omni_store, area=None)
    session = Session(
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
    from bench.language import clean_name

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


async def run_dynamic_simulation(spec: SimulationSpec):
    """Run a 'dynamic' Simulation"""

    simulation_id = get_simulation_id(spec)
    global_store = make_global_store(f"test-{simulation_id}-global")
    regional_store = make_regional_store(f"test-{simulation_id}-regional")
    store_map = StoreMap({"*": regional_store})
    await create_test_db(global_store, BUILTIN_GLOBAL_SCHEMA)
    await create_test_db(regional_store, BUILTIN_REGIONAL_SCHEMA)
    simulation = Simulation(
        id=simulation_id,
        spec=spec,
        global_store=global_store,
        regional_store=regional_store,
        store_map=store_map,
    )

    try:
        await simulation.run()
        await delete_test_db(global_store)
        await delete_test_db(regional_store)
    finally:
        gc.collect()


# nocheckin :Test :Performance: re-use Simulations somehow (like databases?)


def simulated_runtime(
    network: NetworkSpec | None = None,
    users: tuple[UserSpec, ...] = (),
    machines: tuple[MachineSpec, ...] = (),
    clients: tuple[ClientSpec, ...] = (),
    supervisor: SupervisorSpec | None = None,
    benches: tuple[BenchSpec, ...] = (),
    hosts: tuple[HostSpec, ...] = (),
    workloads: tuple[WorkloadSpec, ...] = (),
):
    """Run a test in a simulated Runtime."""

    users = users or (UserSpec(name="user"),)
    machines = machines or (MachineSpec(name="user-machine", bench="user"),)
    clients = clients or (
        ClientSpec(name="user-client", parent=("user", "user")),
        ClientSpec(name="user-machine-client", parent=("machine", "user-machine")),
    )
    benches = benches or (BenchSpec(name="user", owner="user"),)
    hosts = hosts or (HostSpec(bench="user"),)

    def decorator(test_func: Callable[[RuntimeLambdaWorkload], Awaitable[None]]):
        lambda_workload = RuntimeLambdaWorkloadSpec(
            client="user-machine-client",
            bench="user",
            name=test_func.__name__,
            type=WorkloadType.RUNTIME_LAMBDA,
            func=test_func,
        )
        spec = SimulationSpec(
            name=test_func.__name__,
            description=test_func.__doc__ or "",
            network=network or NetworkSpec(),
            users=users,
            machines=machines,
            clients=clients,
            supervisor=supervisor or SupervisorSpec(),
            hosts=hosts,
            workloads=(*workloads, lambda_workload),
        )

        @functools.wraps(test_func)
        async def test_func_in_simulation():
            await run_dynamic_simulation(spec)

        # zero out signature so pytest doesn't try to get any fixture arguments
        test_func_in_simulation.__signature__ = inspect.Signature()  # type: ignore

        return test_func_in_simulation

    return decorator
