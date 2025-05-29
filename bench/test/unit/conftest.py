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
    NODE_TYPES,
    STRUCT_TYPES,
    Bench,
    BenchStatus,
    BuiltinObjectBase,
    Database,
    NodeMode,
    NodeType,
    Package,
    PackageType,
    Session,
    StructType,
)
from bench.test.conftest import _setup_test_env
from bench.test.simulation.core import (
    BenchSpec,
    ClientSpec,
    HostSpec,
    MachineSpec,
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
from bench.utils.oracle import REAL_ORACLE

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


# NOTE: simulation tests must be run with one event loop per function to isolate
@pytest.fixture
def event_loop_policy():
    return SimulatedEventLoopPolicy()


def make_session(name: str):
    """Make a 'fake' session for context"""
    session = Session(mode=NodeMode.MAIN, oracle=REAL_ORACLE)
    return session


# NOTE: we manually set session sync context since it's not propagated across pytest tasks (see above)
# https://github.com/pytest-dev/pytest-asyncio/issues/127#issuecomment-862817549 :PytestAsyncContext


@pytest.fixture  # :PytestAsyncContext
async def session_async(request):
    session = make_session(request.node.name)
    await session.open()
    yield session
    await session.close()


def make_package(session: Session):
    bench = Bench(name="test", slug="test", status=BenchStatus.ACTIVE)
    package = Package(type=PackageType.HOME, name="Home", slug="home")
    bench.add_child(package)
    database = Database(name="Database")
    package.add_child(database)
    bench.database = database
    session.bench = bench
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
        # draw_direct(from_object_type(object_type, reject_invalid=False))
        # for object_type in OBJECT_TYPES
    ]
    ACTIVE_SESSION.set(None)

BUILTIN_OBJECTS_BY_TYPE: Mapping[StructType | NodeType, BuiltinObjectBase] = {
    obj.metatype: obj for obj in BUILTIN_OBJECTS
}
STRUCTS = [BUILTIN_OBJECTS_BY_TYPE[t] for t in STRUCT_TYPES if t in BUILTIN_OBJECTS_BY_TYPE]
NODES = [BUILTIN_OBJECTS_BY_TYPE[t] for t in NODE_TYPES if t in BUILTIN_OBJECTS_BY_TYPE]


# TODO :Test! :Performance: re-use Simulations somehow (databases?)


def make_simulation_spec(
    func: Callable,
    network: NetworkSpec | None = None,
    users: tuple[UserSpec, ...] = (),
    machines: tuple[MachineSpec, ...] = (),
    clients: tuple[ClientSpec, ...] = (),
    supervisor: SupervisorSpec | None = None,
    benches: tuple[BenchSpec, ...] = (),
    hosts: tuple[HostSpec, ...] = (),
    runtimes: tuple[RuntimeSpec, ...] = (),
    workloads: tuple[WorkloadSpec, ...] = (),
    system: bool = False,
) -> SimulationSpec:
    users = users or (UserSpec(name="alice"),)
    machines = machines or (MachineSpec(name="alice-machine", bench="alice"),)
    clients = clients or (
        ClientSpec(name="alice-client", parent=("user", "alice")),
        ClientSpec(name="alice-machine-client", parent=("machine", "alice-machine")),
    )
    benches = benches or (BenchSpec(name="alice", owner="alice"),)
    hosts = hosts or (HostSpec(bench="alice"),)
    spec = SimulationSpec(
        name=func.__name__,
        description=func.__doc__ or "",
        network=network or NetworkSpec(),
        users=users,
        machines=machines,
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
    machines: tuple[MachineSpec, ...] = (),
    clients: tuple[ClientSpec, ...] = (),
    supervisor: SupervisorSpec | None = None,
    benches: tuple[BenchSpec, ...] = (),
    hosts: tuple[HostSpec, ...] = (),
    system: bool = False,
    runtimes: tuple[RuntimeSpec, ...] | Literal[True] = (),
):
    """Run a 'lambda workload' test as a Machine's Runtime inside a Simulation."""
    if runtimes is True:
        runtimes = (RuntimeSpec(name="alice-runtime", machine="alice-machine"),)

    def decorator(test_func: Callable[[Simulation, RuntimeLambdaWorkload], Awaitable[None]]):
        lambda_workload = RuntimeLambdaWorkloadSpec(
            client="alice-machine-client",
            bench="alice",
            name=test_func.__name__,
            type=WorkloadType.RUNTIME_LAMBDA,
            func=test_func,
            system=system,
        )
        spec = make_simulation_spec(
            func=test_func,
            network=network,
            users=users,
            machines=machines,
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
