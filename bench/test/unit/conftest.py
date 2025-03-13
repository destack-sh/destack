# ruff: noqa: E402

import functools
import inspect
import warnings
from typing import Awaitable, Callable, Literal, Mapping

import pytest
import structlog
from opentelemetry import trace

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
    BenchStatus,
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
from bench.system import pg_engine_from_store
from bench.test.conftest import _setup_test_env
from bench.test.simulation.core import (
    BenchSpec,
    ClientSpec,
    ComputerSpec,
    HostSpec,
    NetworkSpec,
    RuntimeSpec,
    SimulatedEventLoopPolicy,
    Simulation,
    SimulationSpec,
    SupervisorSpec,
    UserSpec,
    run_simulation,
)
from bench.test.simulation.workload import (
    RuntimeLambdaWorkload,
    RuntimeLambdaWorkloadSpec,
    WorkloadSpec,
    WorkloadType,
)
from bench.test.strategies import draw_direct, from_object_type
from bench.utils.oracle import REAL_ORACLE, Oracle

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


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


# NOTE: we manually set session sync context since it's not propagated across pytest tasks (see above)
# https://github.com/pytest-dev/pytest-asyncio/issues/127#issuecomment-862817549 :PytestAsyncContext


@pytest.fixture  # :PytestAsyncContext
async def session_async(request):
    from bench.language import clean_name

    session = make_session(clean_name(request.node.name))
    await session.open(_set_in_context=False)
    yield session
    await session.close()


def make_package(session: Session):
    bench = Bench(name="test", slug="test", status=BenchStatus.ACTIVATED)
    package = bench.packages.create(type=PackageType.OPEN, name="Main", slug="main")
    bench.main_store = package.stores.create(name="Store")
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


# TODO :Test! :Performance: re-use Simulations somehow (databases?)


def make_simulation_spec(
    func: Callable,
    network: NetworkSpec | None = None,
    users: tuple[UserSpec, ...] = (),
    computers: tuple[ComputerSpec, ...] = (),
    clients: tuple[ClientSpec, ...] = (),
    supervisor: SupervisorSpec | None = None,
    benches: tuple[BenchSpec, ...] = (),
    hosts: tuple[HostSpec, ...] = (),
    runtimes: tuple[RuntimeSpec, ...] = (),
    workloads: tuple[WorkloadSpec, ...] = (),
    system: bool = False,
) -> SimulationSpec:
    users = users or (UserSpec(name="alice"),)
    computers = computers or (ComputerSpec(name="alice-computer", bench="alice"),)
    clients = clients or (
        ClientSpec(name="alice-client", parent=("user", "alice")),
        ClientSpec(name="alice-computer-client", parent=("computer", "alice-computer")),
    )
    benches = benches or (BenchSpec(name="alice", owner="alice"),)
    hosts = hosts or (HostSpec(bench="alice"),)
    spec = SimulationSpec(
        name=func.__name__,
        description=func.__doc__ or "",
        network=network or NetworkSpec(),
        users=users,
        computers=computers,
        clients=clients,
        supervisor=supervisor or SupervisorSpec(),
        benches=benches,
        hosts=hosts,
        runtimes=runtimes,
        workloads=workloads,
        system=system,
    )
    return spec


def simulated_runtime(
    *,
    network: NetworkSpec | None = None,
    users: tuple[UserSpec, ...] = (),
    computers: tuple[ComputerSpec, ...] = (),
    clients: tuple[ClientSpec, ...] = (),
    supervisor: SupervisorSpec | None = None,
    benches: tuple[BenchSpec, ...] = (),
    hosts: tuple[HostSpec, ...] = (),
    system: bool = False,
    runtimes: tuple[RuntimeSpec, ...] | Literal[True] = (),
):
    """Run a 'lambda workload' test as a Computer's Runtime inside a Simulation."""
    if runtimes is True:
        runtimes = (RuntimeSpec(name="alice-runtime", computer="alice-computer"),)

    def decorator(test_func: Callable[[Simulation, RuntimeLambdaWorkload], Awaitable[None]]):
        lambda_workload = RuntimeLambdaWorkloadSpec(
            client="alice-computer-client",
            bench="alice",
            name=test_func.__name__,
            type=WorkloadType.RUNTIME_LAMBDA,
            func=test_func,
        )
        spec = make_simulation_spec(
            func=test_func,
            network=network,
            users=users,
            computers=computers,
            clients=clients,
            supervisor=supervisor,
            benches=benches,
            hosts=hosts,
            runtimes=runtimes,
            workloads=(lambda_workload,),
            system=system,
        )

        @functools.wraps(test_func)
        async def test_func_in_simulation():
            await run_simulation(spec)

        # zero out signature so pytest doesn't try to get any fixture arguments
        test_func_in_simulation.__signature__ = inspect.Signature()  # type: ignore

        return test_func_in_simulation

    return decorator
