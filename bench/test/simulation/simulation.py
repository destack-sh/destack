import asyncio
import time
from itertools import chain
from random import Random
from typing import final
from uuid import UUID

import pytest
import structlog
from opentelemetry import trace

from bench.language import Store
from bench.proto import wire
from bench.proto.wire import (
    ClientDataIn,
    RpcMetadata,
    SignupUserRequest,
    SupervisorClient,
)
from bench.sql.engine import GLOBAL_SCHEMA
from bench.test.fixtures import create_test_db, make_global_store
from bench.test.simulation.grpc import SimulatedChannel
from bench.test.simulation.oracle import SimulatedLoop
from bench.test.simulation.service import HostHandle, SupervisorHandle
from bench.test.simulation.spec import (
    BenchSpec,
    ClientSpec,
    HostSpec,
    NetworkSpec,
    SimulationSpec,
)
from bench.test.simulation.workload import (
    ReadPackageSpec,
    WorkloadBase,
    WriteBlockTreeSpec,
    get_workload_cls,
)
from bench.utils.oracle import REAL_ORACLE
from bench.utils.task import TaskManager

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def get_simulation_id(simulation: SimulationSpec) -> str:
    return f"{simulation.name}-{simulation.seed}"


@final
class Simulation:
    """An active simulation"""

    def __init__(self, id: str, spec: SimulationSpec, global_store: Store):
        self.id = id
        self.spec = spec
        self.global_store = global_store

        # system
        self.random = Random(spec.seed)
        self.network = Network(spec.network, self)
        self._oracle = REAL_ORACLE  # nocheckin: ???
        self._tasks = TaskManager(owner=self, logger=logger, oracle=self._oracle)
        self._loop = SimulatedLoop(base_time_ns=time.time_ns)

        # services
        self._supervisor = SupervisorHandle(spec.supervisor, self._oracle, self)
        self._hosts_by_name: dict[str, HostHandle] = {}
        for host_spec in spec.hosts:
            bench_id = UUID(int=self.random.getrandbits(128))
            host = HostHandle(bench_id, host_spec, self._oracle, self)
            self._hosts_by_name[host_spec.bench.name] = host

        # clients
        self._users_by_name: dict[str, UserHandle] = {}
        self._clients_by_name: dict[str, ClientHandle] = {}
        for username in chain(
            *(client.username for client in spec.clients),
            *(host.bench.owner for host in spec.hosts),
        ):
            if username in self._users_by_name:
                continue
            user = UserHandle(username, self)
            self._users_by_name[username] = user
        for client_spec in spec.clients:
            user = self._users_by_name[client_spec.username]
            client = ClientHandle(client_spec, user, self)
            self._clients_by_name[client_spec.name] = client

        # workloads
        self._workloads: list[WorkloadBase] = []
        self._workloads_by_name: dict[str, WorkloadBase] = {}
        for workload_spec in spec.workloads:
            workload_cls = get_workload_cls(workload_spec.type)
            workload = workload_cls(workload_spec, self)
            self._workloads.append(workload)
            if workload_spec.name in self._workloads_by_name:
                raise ValueError(f"duplicate workload name: {workload_spec.name} in {self!r}")
            self._workloads_by_name[workload_spec.name] = workload

        # runtime state
        self._started_at_ns = None
        self._finished_at_ns = None

    def __str__(self):
        return f"{self.id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    @property
    def services(self):
        return chain((self._supervisor,), self._hosts_by_name.values())

    def get_client(self, name: str) -> "ClientHandle":
        client = self._clients_by_name.get(name)
        assert (
            client is not None
        ), f"{self!r} has no client: {name} (available: {list(self._clients_by_name)})"
        return client

    def get_host(self, name: str) -> "HostHandle":
        host = self._hosts_by_name.get(name)
        assert (
            host is not None
        ), f"{self!r} has no host: {name} (available: {list(self._hosts_by_name)})"
        return host

    async def run(self):
        """Run the simulation."""
        with tracer.start_as_current_span("simulation.prepare"):
            # start services
            for service in self.services:
                await service.run()

            # prepare users & clients
            async with SimulatedChannel(
                services=(self._supervisor.service,), oracle=self._oracle
            ) as supervisor_channel:  # (use direct channel here to bootstrap clients)
                supervisor_client = SupervisorClient(supervisor_channel)
                for user in self._users_by_name.values():
                    await user.prepare(supervisor_client)
                for client in self._clients_by_name.values():
                    await client.prepare(supervisor_client)

            # prepare workloads
            await asyncio.gather(*(workload.prepare() for workload in self._workloads))
        logger.info("simulation.start", simulation=self)

        self._started_at_ns = REAL_ORACLE.time_ns()
        with tracer.start_as_current_span("simulation.run"):
            # run workloads
            await asyncio.gather(*(workload.run() for workload in self._workloads))
        logger.info("simulation.run", simulation=self)
        self._finished_at_ns = REAL_ORACLE.time_ns()


@final
class UserHandle:
    """A User"""

    def __init__(self, name: str, simulation: Simulation):
        self.name = name
        self.simulation = simulation

    async def prepare(self, supervisor_client: SupervisorClient):
        client_in = ClientDataIn(
            type=wire.ClientType.BENCH_SERVER, name=f"{self.name}-signup", device_name="test"
        )
        signup_req = SignupUserRequest(
            slug=self.name, name=self.name, email=f"{self.name}@test.com", client=client_in
        )
        await supervisor_client.signup_user(signup_req)


@final
class ClientHandle:
    """A Client to a Bench"""

    def __init__(self, spec: ClientSpec, user: UserHandle, simulation: Simulation):
        self.spec = spec
        self.user = user
        self.simulation = simulation
        self._rpc_metadata: RpcMetadata | None = None

    async def prepare(self, supervisor_client: SupervisorClient):
        pass


@final
class Network:
    """A Network for connecting services and clients"""

    def __init__(self, spec: NetworkSpec, simulation: Simulation):
        self.spec = spec
        self.simulation = simulation
        self._channels = {}


#
# Read and mark simulations from disk
#

# TODO :Test: read simulation specs from disk (some JSON schema + toml thing?)
AVAILABLE_SIMULATIONS: list[SimulationSpec] = [
    # smoke
    SimulationSpec(
        name="single_client_smoke",
        clients=(ClientSpec(name="testuser-1", username="testuser"),),
    ),
    SimulationSpec(
        name="multi_client_smoke",
        clients=(
            ClientSpec(name="testuser-1", username="testuser"),
            ClientSpec(name="testuser-2", username="testuser"),
        ),
    ),
    SimulationSpec(
        name="single_host_smoke",
        hosts=(HostSpec(bench=BenchSpec(name="testbench", owner="testuser")),),
    ),
    SimulationSpec(
        name="multi_host_smoke",
        hosts=(
            HostSpec(bench=BenchSpec(name="testbench-1", owner="testuser")),
            HostSpec(bench=BenchSpec(name="testbench-2", owner="testuser")),
        ),
    ),
    # simple
    SimulationSpec(
        name="single_client_rw_block_tree",
        hosts=(HostSpec(bench=BenchSpec(name="testbench", owner="testuser")),),
        clients=(ClientSpec(name="testuser-1", username="testuser"),),
        workloads=(WriteBlockTreeSpec(client="testuser-1"), ReadPackageSpec()),
    ),
    SimulationSpec(
        name="multi_client_rw_block_tree",
        hosts=(HostSpec(bench=BenchSpec(name="testbench", owner="testuser")),),
        clients=(
            ClientSpec(name="testuser-1", username="testuser"),
            ClientSpec(name="testuser-2", username="testuser"),
        ),
        workloads=(WriteBlockTreeSpec(client="testuser-1"), ReadPackageSpec()),
    ),
]


@pytest.mark.parametrize("spec", AVAILABLE_SIMULATIONS, ids=lambda s: s.name)
async def test_simulation(spec: SimulationSpec):
    simulation_id = get_simulation_id(spec)
    global_store = make_global_store(f"test_{simulation_id}")
    await create_test_db(global_store, GLOBAL_SCHEMA)
    simulation = Simulation(simulation_id, spec, global_store)
    try:
        await simulation.run()
    except Exception as e:
        logger.exception("simulation.error", simulation=simulation, error=e)
        raise
