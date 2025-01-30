import asyncio
from itertools import chain
from random import Random
from typing import NamedTuple, final
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import Store
from bench.proto import SupervisorClient
from bench.system import StoreMap
from bench.test.simulation.client import ClientHandle, UserHandle
from bench.test.simulation.oracle import SimulatedEventLoop, SimulatedOracle
from bench.test.simulation.service import HostHandle, ServiceHandle, SupervisorHandle
from bench.test.simulation.spec import NetworkSpec, SimulationSpec
from bench.test.simulation.transport import SimulatedChannel
from bench.test.simulation.workload import WorkloadBase, get_workload_cls
from bench.utils.oracle import REAL_ORACLE
from bench.utils.task import TaskManager

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def get_simulation_id(simulation: SimulationSpec) -> str:
    return f"{simulation.name}-{simulation.seed}"


@final
class Simulation:
    """A Simulation of Hosts, Clients and Supervisors performing some Workloads."""

    def __init__(
        self,
        id: str,
        spec: SimulationSpec,
        global_store: Store,
        regional_store: Store,
        store_map: StoreMap,
    ):
        self.id = id
        self.spec = spec
        self.global_store = global_store
        self.regional_store = regional_store
        self.store_map = store_map

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
