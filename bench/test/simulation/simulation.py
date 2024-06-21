import asyncio
import time
from itertools import chain
from random import Random
from typing import NamedTuple, final
from uuid import UUID

import pytest
import structlog
from opentelemetry import trace

from bench.language import Store
from bench.proto.wire import (
    SupervisorClient,
)
from bench.sql.engine import GLOBAL_SCHEMA
from bench.test.fixtures import create_test_db, make_global_store
from bench.test.simulation.client import ClientHandle, UserHandle
from bench.test.simulation.grpc import SimulatedChannel
from bench.test.simulation.oracle import SimulatedLoop, SimulatedOracle
from bench.test.simulation.service import HostHandle, ServiceHandle, SupervisorHandle
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
from bench.utils.task import TaskManager, wrap_task

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
        self._sim_loop = SimulatedLoop(base_time_ns=time.time_ns)
        self._sim_oracle = SimulatedOracle(loop=self._sim_loop, random=self.random)
        self._tasks = TaskManager(owner=self, logger=logger, oracle=self._sim_oracle)

        # services
        self._supervisor = SupervisorHandle("supervisor", spec.supervisor, self._sim_oracle, self)
        self._hosts_by_name: dict[str, HostHandle] = {}
        for host_spec in spec.hosts:
            if host_spec.bench.name in self._hosts_by_name:
                raise ValueError(f"duplicate host name: {host_spec.bench.name} in {self!r}")
            host = HostHandle(host_spec.bench.name, host_spec, self._sim_oracle, self)
            self._hosts_by_name[host_spec.bench.name] = host

        # clients
        self._users_by_name: dict[str, UserHandle] = {}
        self._clients_by_name: dict[str, ClientHandle] = {}
        for username in chain(client.username for client in spec.clients):
            if username in self._users_by_name:
                continue  # multiple clients for the same user
            user = UserHandle(username, self)
            self._users_by_name[username] = user
        for client_spec in spec.clients:
            user = self._users_by_name[client_spec.username]
            client = ClientHandle(client_spec, user, self)
            if client_spec.name in self._clients_by_name:
                raise ValueError(f"duplicate client name: {client_spec.name} in {self!r}")
            self._clients_by_name[client_spec.name] = client
            user._clients_by_name[client_spec.name] = client

        # workloads
        self._workloads: list[WorkloadBase] = []
        self._workloads_by_name: dict[str, WorkloadBase] = {}
        for workload_spec in spec.workloads:
            workload_cls = get_workload_cls(workload_spec.type)
            workload = workload_cls(workload_spec, self._sim_oracle, self)
            self._workloads.append(workload)
            if workload_spec.name in self._workloads_by_name:
                raise ValueError(f"duplicate workload name: {workload_spec.name} in {self!r}")
            self._workloads_by_name[workload_spec.name] = workload

        # runtime state
        self._started_at_ns = None
        self._terminated_at_ns = None

    def __str__(self):
        return f"{self.id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    def get_client(self, name: str) -> "ClientHandle":
        client = self._clients_by_name.get(name)
        assert (
            client is not None
        ), f"{self!r} has no client: '{name}' (available: {list(self._clients_by_name)})"
        return client

    def get_host(self, name: str) -> "HostHandle":
        host = self._hosts_by_name.get(name)
        assert (
            host is not None
        ), f"{self!r} has no host: '{name}' (available: {list(self._hosts_by_name)})"
        return host

    def resolve_bench_id(self, name: str) -> UUID:
        host = self.get_host(name)
        return host.bench_id

    async def run(self):
        """Run the simulation."""
        # start simulation loop
        await self._sim_loop.start()

        # prepare services and such
        #  (use direct supervisor channel to bootstrap)
        with tracer.start_as_current_span("simulation.prepare"):
            await self._supervisor.start()
            async with SimulatedChannel(
                services=(self._supervisor.service,), oracle=self._sim_oracle
            ) as supervisor_channel:
                supervisor_client = SupervisorClient(supervisor_channel)
                # prepare users & clients
                for user in self._users_by_name.values():
                    await user.prepare(supervisor_client)
                for client in self._clients_by_name.values():
                    await client.prepare(supervisor_client)
                # prepare hosts (create benches)
                for host in self._hosts_by_name.values():
                    user = self._users_by_name.get(host.spec.bench.owner)
                    assert user is not None, f"{host!r} owner has no clients in {self!r}"
                    await host.prepare(supervisor_client, user.some_client)
            # and run hosts
            await asyncio.gather(*(host.start() for host in self._hosts_by_name.values()))

            # prepare workloads
            await asyncio.gather(*(workload.prepare() for workload in self._workloads))
        logger.info("simulation.start", simulation=self)

        # run workloads until completion
        try:
            self._started_at_ns = REAL_ORACLE.time_ns()
            with tracer.start_as_current_span("simulation.run"):
                tasks = (
                    wrap_task(workload.run(), task_id=workload.name, logger=logger, owner=workload)
                    for workload in self._workloads
                )
                await asyncio.gather(*tasks)
            logger.info("simulation.run", simulation=self)
        except Exception as e:
            logger.error("simulation.error", simulation=self, exc_info=e)
            raise
        finally:
            # cleanup
            self._close()
            self._terminated_at_ns = REAL_ORACLE.time_ns()

    def _close(self):
        self._supervisor.close()
        for host in self._hosts_by_name.values():
            host.close()
        self._tasks.close()
        self._sim_loop.close()


class ConnectionPair(NamedTuple):
    """A pair of connected services"""

    client_name: str
    service_id: str


@final
class Network:
    """A Network for connecting services and clients"""

    def __init__(self, spec: NetworkSpec, simulation: Simulation):
        self.spec = spec
        self.simulation = simulation
        self._channels: dict[ConnectionPair, SimulatedChannel] = {}

    def __str__(self):
        return f"{len(self._channels)} channels"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    async def connect(self, client: ClientHandle, service: ServiceHandle) -> SimulatedChannel:
        """Connects a client to a service"""
        pair = ConnectionPair(client.spec.name, service.id)
        channel = self._channels.get(pair)
        if channel is not None:
            return channel
        channel = SimulatedChannel(services=(service.service,), oracle=self.simulation._sim_oracle)
        self._channels[pair] = channel
        await channel.open()
        return channel

    def close(self):
        for channel in self._channels.values():
            channel.close()

    async def wait_closed(self):
        await asyncio.gather(*(channel.wait_closed() for channel in self._channels.values()))
        self._channels.clear()


#
# Read and mark simulations from disk
#

# NOTE :Test: read simulation specs from disk (some JSON schema + toml thing?)
#  probably also mark and categorize them?
AVAILABLE_SIMULATIONS: list[SimulationSpec] = [
    # sanity
    SimulationSpec(
        name="single_client_sanity",
        clients=(ClientSpec(name="alice-1", username="alice"),),
    ),
    SimulationSpec(
        name="multi_client_sanity",
        clients=(
            ClientSpec(name="alice-1", username="alice"),
            ClientSpec(name="alice-2", username="alice"),
            ClientSpec(name="bob-1", username="bob"),
        ),
    ),
    SimulationSpec(
        name="single_host_sanity",
        clients=(ClientSpec(name="alice-1", username="alice"),),
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
    ),
    SimulationSpec(
        name="multi_host_sanity",
        clients=(
            ClientSpec(name="alice-1", username="alice"),
            ClientSpec(name="bob-1", username="bob"),
        ),
        hosts=(
            HostSpec(bench=BenchSpec(name="alice", owner="alice")),
            HostSpec(bench=BenchSpec(name="bob", owner="bob")),
        ),
    ),
    # simple
    SimulationSpec(
        name="single_client_rw_block_tree",
        clients=(ClientSpec(name="alice-1", username="alice"),),
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
        workloads=(
            WriteBlockTreeSpec(bench="alice", client="alice-1"),
            ReadPackageSpec(bench="alice", client="alice-1"),
        ),
    ),
    SimulationSpec(
        name="multi_client_rw_block_tree",
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
        clients=(
            ClientSpec(name="alice-1", username="alice"),
            ClientSpec(name="alice-2", username="alice"),
        ),
        workloads=(
            WriteBlockTreeSpec(bench="alice", client="alice-1"),
            ReadPackageSpec(bench="alice", client="alice-1"),
            ReadPackageSpec(bench="alice", client="alice-2"),
        ),
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
