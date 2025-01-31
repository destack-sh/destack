import asyncio
from datetime import datetime
from random import Random
from typing import TYPE_CHECKING, final
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import Store
from bench.proto import SupervisorClient
from bench.system import StoreMap
from bench.utils.oracle import REAL_ORACLE
from bench.utils.task import TaskManager

from .client import ClientHandle
from .host import HostHandle
from .machine import MachineHandle
from .network import Network
from .oracle import SimulatedEventLoop, SimulatedOracle
from .spec import ClientSpec, HostSpec, MachineSpec, SimulationSpec, UserSpec
from .supervisor import SupervisorHandle
from .transport import SimulatedChannel
from .user import UserHandle

if TYPE_CHECKING:
    from bench.test.simulation.workload import Workload, WorkloadSpec

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def get_simulation_id(simulation: SimulationSpec) -> str:
    return f"{simulation.name}-{simulation.seed}"


@final
class Simulation:
    """A Simulation of Hosts, Clients and Supervisors performing Workloads."""

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
        loop = asyncio.get_event_loop()
        assert isinstance(loop, SimulatedEventLoop), f"need simulated loop: {loop}"
        self.loop = loop
        self.oracle = SimulatedOracle(loop=self.loop, random=self.random)
        self.tasks = TaskManager(
            owner=self, logger=logger, oracle=self.oracle, on_error=self.on_error
        )
        self.errors: list[Exception] = []
        self.has_error = asyncio.Event()

        # content
        self.supervisor = SupervisorHandle("supervisor", spec.supervisor, self.oracle, self)
        self.hosts_by_name: dict[str, HostHandle] = {}
        self.users_by_name: dict[str, UserHandle] = {}
        self.machines_by_name: dict[str, MachineHandle] = {}
        self.clients_by_name: dict[str, ClientHandle] = {}
        self.workloads: list[Workload] = []
        self.workloads_by_name: dict[str, Workload] = {}
        self.workloads_by_group: dict[str, list[Workload]] = {}

        # status
        self.started_at: datetime | None = None
        self.terminated_at: datetime | None = None

    def __str__(self):
        return f"'{self.id}'"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    def add_user(self, user_spec: UserSpec) -> UserHandle:
        """Add a User to the simulation."""
        if user_spec.name in self.users_by_name:
            raise ValueError(f"duplicate user name: {user_spec.name} in {self!r}")
        user = UserHandle(user_spec.name, self)
        self.users_by_name[user_spec.name] = user
        return user

    def add_machine(self, machine_spec: MachineSpec) -> MachineHandle:
        """Add a Machine to the simulation."""
        if machine_spec.name in self.machines_by_name:
            raise ValueError(f"duplicate machine name: {machine_spec.name} in {self!r}")
        machine = MachineHandle(machine_spec, self)
        self.machines_by_name[machine_spec.name] = machine
        return machine

    def add_client(self, client_spec: ClientSpec) -> ClientHandle:
        """Add a Client to the simulation. The User will be created if not present."""
        if client_spec.name in self.clients_by_name:
            raise ValueError(f"duplicate client name: {client_spec.name} in {self!r}")
        if client_spec.parent[0] == "user":
            parent = self.users_by_name[client_spec.parent[1]]
        elif client_spec.parent[0] == "machine":
            parent = self.machines_by_name[client_spec.parent[1]]
        else:
            raise ValueError(f"invalid client parent: {client_spec.parent} in {self!r}")
        client = ClientHandle(client_spec, parent, self)
        self.clients_by_name[client_spec.name] = client
        parent.clients_by_name[client_spec.name] = client
        return client

    def add_host(self, host_spec: HostSpec) -> HostHandle:
        """Add a Host to the simulation."""
        if host_spec.bench.name in self.hosts_by_name:
            raise ValueError(f"duplicate host name: {host_spec.bench.name} in {self!r}")
        host = HostHandle(host_spec.bench.name, host_spec, self.oracle, self)
        self.hosts_by_name[host_spec.bench.name] = host
        return host

    def add_workload(self, workload_spec: "WorkloadSpec") -> "Workload":
        """Add a Workload to the simulation."""
        from bench.test.simulation.workload import get_workload_cls

        if workload_spec.name in self.workloads_by_name:
            raise ValueError(f"duplicate workload name: {workload_spec.name} in {self!r}")
        workload_cls = get_workload_cls(workload_spec.type)
        workload = workload_cls(workload_spec, self.oracle, self)
        self.workloads.append(workload)
        self.workloads_by_name[workload_spec.name] = workload
        if workload_spec.group:
            if workload_spec.group not in self.workloads_by_group:
                self.workloads_by_group[workload_spec.group] = []
            self.workloads_by_group[workload_spec.group].append(workload)
        return workload

    def on_error(self, error: Exception):
        logger.error("simulation.error", simulation=self, exc_infoerror=error)
        self.errors.append(error)
        self.has_error.set()

    def get_user(self, name: str) -> "UserHandle":
        user = self.users_by_name.get(name)
        assert (
            user is not None
        ), f"{self!r} has no user: '{name}' (available: {list(self.users_by_name)})"
        return user

    def get_client(self, name: str) -> "ClientHandle":
        client = self.clients_by_name.get(name)
        assert (
            client is not None
        ), f"{self!r} has no client: '{name}' (available: {list(self.clients_by_name)})"
        return client

    def get_host(self, name: str) -> "HostHandle":
        host = self.hosts_by_name.get(name)
        assert (
            host is not None
        ), f"{self!r} has no host: '{name}' (available: {list(self.hosts_by_name)})"
        return host

    def get_workload_group(self, name: str) -> list["Workload"]:
        if name not in self.workloads_by_group:
            raise ValueError(f"{self!r} has no workload group: '{name}'")
        return self.workloads_by_group[name]

    def resolve_bench_id(self, name: str) -> UUID:
        host = self.get_host(name)
        return host.bench_id

    async def run(self):
        """Run the simulation."""
        # nocheckin: reconsider (should add_xxx be async, should supervisor_channel be retained, ...)
        # add everything from spec
        for user in self.spec.users:
            self.add_user(user)
        for machine in self.spec.machines:
            self.add_machine(machine)
        for client in self.spec.clients:
            self.add_client(client)
        for host in self.spec.hosts:
            self.add_host(host)
        for workload in self.spec.workloads:
            self.add_workload(workload)

        # prepare services and such
        #  (use direct supervisor channel to bootstrap)
        with tracer.start_as_current_span("simulation.prepare"):
            await self.supervisor.start()
            async with SimulatedChannel(
                services=(self.supervisor.service,), oracle=self.oracle
            ) as supervisor_channel:
                supervisor_client = SupervisorClient(supervisor_channel)
                # prepare users and their clients
                for user in self.users_by_name.values():
                    await user.prepare(supervisor_client)
                for client in self.clients_by_name.values():
                    if isinstance(client.parent, UserHandle):
                        await client.prepare(supervisor_client)
                # prepare hosts (create benches)
                for host in self.hosts_by_name.values():
                    user = self.users_by_name.get(host.spec.bench.owner)
                    assert user is not None, f"{host!r} owner has no clients in {self!r}"
                    await host.prepare(supervisor_client, user.some_client)
                # prepare machines and their clients
                for machine in self.machines_by_name.values():
                    await machine.prepare()
                for client in self.clients_by_name.values():
                    if isinstance(client.parent, MachineHandle):
                        await client.prepare(supervisor_client)
            # and run hosts
            await asyncio.gather(*(host.start() for host in self.hosts_by_name.values()))
            # prepare workloads
            await asyncio.gather(*(workload.prepare() for workload in self.workloads))
            logger.info("simulation.prepare", simulation=self, span="current")

        # run workloads until completion
        try:
            self.started_at = REAL_ORACLE.utc()
            with tracer.start_as_current_span("simulation.run"):
                await asyncio.gather(*(workload.run() for workload in self.workloads))
                logger.info("simulation.run", simulation=self, span="current")
            # and run checks
            with tracer.start_as_current_span("simulation.check"):
                await asyncio.gather(*(workload.check() for workload in self.workloads))
                logger.info("simulation.check", simulation=self, span="current")
                if self.errors:
                    if len(self.errors) == 1:
                        raise self.errors[0]
                    else:
                        raise RuntimeError(f"multiple errors in {self!r}: {self.errors}")
        except Exception as e:
            logger.error("simulation.error", simulation=self, exc_info=e)
            raise
        finally:
            # cleanup
            self._close()
            await self._wait_closed()
            self.terminated_at = REAL_ORACLE.utc()

    def _close(self):
        self.supervisor.close()
        for host in self.hosts_by_name.values():
            host.close()
        self.tasks.close()

    async def _wait_closed(self):
        await asyncio.gather(
            self.supervisor.wait_closed(),
            *(host.wait_closed() for host in self.hosts_by_name.values()),
            self.tasks.wait_closed(),
            return_exceptions=True,
        )
