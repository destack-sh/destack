import asyncio
import gc
from dataclasses import replace
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
from bench.test.conftest import TestProfile
from bench.test.fixtures import create_test_db, make_system_store
from bench.test.simulation.client import ClientHandle, UserHandle
from bench.test.simulation.oracle import SimulatedEventLoop, SimulatedOracle
from bench.test.simulation.service import HostHandle, ServiceHandle, SupervisorHandle
from bench.test.simulation.spec import (
    BenchSpec,
    ClientSpec,
    HostSpec,
    NetworkSpec,
    SimulationSpec,
)
from bench.test.simulation.transport import SimulatedChannel
from bench.test.simulation.workload import (
    ReadPackageSpec,
    WatchLogsSpec,
    WorkloadBase,
    WriteBlockTreeSpec,
    get_workload_cls,
)
from bench.utils.func import group_by
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
        self._loop = asyncio.get_event_loop()
        assert isinstance(self._loop, SimulatedEventLoop), f"bad loop: {self._loop}"
        self._oracle = SimulatedOracle(loop=self._loop, random=self.random)
        self._tasks = TaskManager(
            owner=self, logger=logger, oracle=self._oracle, on_error=self.on_error
        )
        self._errors: list[Exception] = []
        self._has_error = asyncio.Event()

        # services
        self._supervisor = SupervisorHandle("supervisor", spec.supervisor, self._oracle, self)
        self._hosts_by_name: dict[str, HostHandle] = {}
        for host_spec in spec.hosts:
            if host_spec.bench.name in self._hosts_by_name:
                raise ValueError(f"duplicate host name: {host_spec.bench.name} in {self!r}")
            host = HostHandle(host_spec.bench.name, host_spec, self._oracle, self)
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
        self._workloads_by_group: dict[str, list[WorkloadBase]] = {}
        for workload_spec in spec.workloads:
            workload_cls = get_workload_cls(workload_spec.type)
            workload = workload_cls(workload_spec, self._oracle, self)
            self._workloads.append(workload)
            if workload_spec.name in self._workloads_by_name:
                raise ValueError(f"duplicate workload name: {workload_spec.name} in {self!r}")
            self._workloads_by_name[workload_spec.name] = workload
            if workload_spec.group:
                if workload_spec.group not in self._workloads_by_group:
                    self._workloads_by_group[workload_spec.group] = []
                self._workloads_by_group[workload_spec.group].append(workload)

        # runtime state
        self._started_at_ns = None
        self._terminated_at_ns = None

    def __str__(self):
        return f"'{self.id}'"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    def on_error(self, error: Exception):
        logger.error("simulation.error", simulation=self, exc_infoerror=error)
        self._errors.append(error)
        self._has_error.set()

    def get_user(self, name: str) -> "UserHandle":
        user = self._users_by_name.get(name)
        assert (
            user is not None
        ), f"{self!r} has no user: '{name}' (available: {list(self._users_by_name)})"
        return user

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

    def get_workload_group(self, name: str) -> list["WorkloadBase"]:
        if name not in self._workloads_by_group:
            raise ValueError(f"{self!r} has no workload group: '{name}'")
        return self._workloads_by_group[name]

    def resolve_bench_id(self, name: str) -> UUID:
        host = self.get_host(name)
        return host.bench_id

    async def run(self):
        """Run the simulation."""
        # prepare services and such
        #  (use direct supervisor channel to bootstrap)
        with tracer.start_as_current_span("simulation.prepare"):
            await self._supervisor.start()
            async with SimulatedChannel(
                services=(self._supervisor.service,), oracle=self._oracle
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
            logger.info("simulation.prepare", simulation=self, span="current")

        # run workloads until completion
        try:
            self._started_at_ns = REAL_ORACLE.time_ns()
            with tracer.start_as_current_span("simulation.run"):
                await asyncio.gather(*(workload.run() for workload in self._workloads))
                logger.info("simulation.run", simulation=self, span="current")
            # and run checks
            with tracer.start_as_current_span("simulation.check"):
                await asyncio.gather(*(workload.check() for workload in self._workloads))
                logger.info("simulation.check", simulation=self, span="current")
                if self._errors:
                    if len(self._errors) == 1:
                        raise self._errors[0]
                    else:
                        raise RuntimeError(f"multiple errors in {self!r}: {self._errors}")
        except Exception as e:
            logger.error("simulation.error", simulation=self, exc_info=e)
            raise
        finally:
            # cleanup
            self._close()
            await self._wait_closed()
            self._terminated_at_ns = REAL_ORACLE.time_ns()

    def _close(self):
        self._supervisor.close()
        for host in self._hosts_by_name.values():
            host.close()
        self._tasks.close()

    async def _wait_closed(self):
        await asyncio.gather(
            self._supervisor.wait_closed(),
            *(host.wait_closed() for host in self._hosts_by_name.values()),
            self._tasks.wait_closed(),
            return_exceptions=True,
        )


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
        channel = SimulatedChannel(services=(service.service,), oracle=self.simulation._oracle)
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
    # empty tests to sanity test simulation setup
    SimulationSpec(
        name="SingleClientEmpty",
        description="Create a single client with no workloads",
        profile=TestProfile.QUICK,
        clients=(ClientSpec(name="alice-1", username="alice"),),
    ),
    SimulationSpec(
        name="MultiClientEmpty",
        description="Create multiple clients with no workloads",
        profile=TestProfile.QUICK,
        clients=(
            ClientSpec(name="alice-1", username="alice"),
            ClientSpec(name="alice-2", username="alice"),
            ClientSpec(name="bob-1", username="bob"),
        ),
    ),
    SimulationSpec(
        name="SingleHostEmpty",
        description="Create a single host with no workloads",
        profile=TestProfile.QUICK,
        clients=(ClientSpec(name="alice-1", username="alice"),),
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
    ),
    SimulationSpec(
        name="MultiHostEmpty",
        description="Create multiple hosts with no workloads",
        profile=TestProfile.QUICK,
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
        name="SingleWriterBlockTree",
        description="Write a block tree with one client, read with another client",
        clients=(ClientSpec(name="alice-1", username="alice"),),
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
        workloads=(
            WriteBlockTreeSpec(bench="alice", client="alice-1", transactions=10),
            ReadPackageSpec(bench="alice", client="alice-1"),
        ),
    ),
    SimulationSpec(
        name="SingleWriterMultiReaderBlockTree",
        description="Write a block tree with one client, read with multiple clients",
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
        clients=(
            ClientSpec(name="alice-1", username="alice"),
            ClientSpec(name="alice-2", username="alice"),
            ClientSpec(name="alice-3", username="alice"),
        ),
        workloads=(
            WriteBlockTreeSpec(
                bench="alice", client="alice-1", transactions=10, group="alice-0-main"
            ),
            ReadPackageSpec(bench="alice", client="alice-1", group="alice-0-main"),
            ReadPackageSpec(bench="alice", client="alice-2", group="alice-0-main"),
            ReadPackageSpec(bench="alice", client="alice-3", group="alice-0-main"),
        ),
    ),
    SimulationSpec(
        name="SingleWriterLogTail",
        description="Write a block tree with one client, watch logs multiple clients",
        clients=(
            ClientSpec(name="alice-1", username="alice"),
            ClientSpec(name="alice-2", username="alice"),
        ),
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
        workloads=(
            WriteBlockTreeSpec(bench="alice", client="alice-1", transactions=10),
            WatchLogsSpec(bench="alice", client="alice-1", tail_user="alice", group="alice-0-main"),
            WatchLogsSpec(bench="alice", client="alice-2", tail_user="alice", group="alice-0-main"),
        ),
    ),
    # TODO :Test!: test multi-writer, various write patterns, latency, ...
]
SIMULATIONS_BY_PROFILE = group_by(AVAILABLE_SIMULATIONS, lambda s: s.profile)


async def _do_test_simulation(spec: SimulationSpec):
    simulation_id = get_simulation_id(spec)
    global_store = make_system_store(f"test_{simulation_id}")
    await create_test_db(global_store, GLOBAL_SCHEMA)
    simulation = Simulation(simulation_id, spec, global_store)
    try:
        await simulation.run()
    except Exception as e:
        logger.exception("simulation.error", simulation=simulation, error=e)
        raise
    finally:
        # force gc for simulation isolation
        gc.collect()


# NOTE: we lay out the simulation tests like below so so pytest collects them nicely
#  (organized by category and parameterized by simulation)


@pytest.mark.quick()
@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.QUICK, ()), ids=lambda s: s.name
)
async def test_simulation_quick(spec: SimulationSpec):
    await _do_test_simulation(spec)


@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.DEFAULT, ()), ids=lambda s: s.name
)
async def test_simulation_default(spec: SimulationSpec):
    await _do_test_simulation(spec)


@pytest.mark.careful()
@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.CAREFUL, ()), ids=lambda s: s.name
)
async def test_simulation_careful(spec: SimulationSpec):
    await _do_test_simulation(spec)


@pytest.mark.paranoid()
@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.PARANOID, ()), ids=lambda s: s.name
)
async def test_simulation_paranoid(spec: SimulationSpec, num_seeds: int = 1):
    for i in range(0, num_seeds):
        subspec = replace(spec, seed=spec.seed + i)
        await _do_test_simulation(subspec)
