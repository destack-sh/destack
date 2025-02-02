import asyncio
from datetime import datetime
from random import Random
from typing import TYPE_CHECKING, final
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import NodeArea, Store
from bench.proto import SupervisorClient
from bench.system import StoreMap, pg_engine_from_store
from bench.utils.oracle import REAL_ORACLE
from bench.utils.task import TaskManager

from .bench import BenchHandle
from .client import ClientHandle
from .host import HostHandle
from .machine import MachineHandle
from .network import NetworkHandle
from .oracle import SimulatedEventLoop, SimulatedOracle
from .runtime import RuntimeHandle
from .service import ServiceHandle
from .spec import (
    BenchSpec,
    ClientSpec,
    HostSpec,
    MachineSpec,
    RuntimeSpec,
    SimulationSpec,
    UserSpec,
)
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
        self.global_pg_engine = pg_engine_from_store("pg-global", global_store, NodeArea.GLOBAL)
        self.regional_pg_engine = pg_engine_from_store(
            f"pg-regional-{regional_store.region.name.lower()}", regional_store, NodeArea.REGIONAL
        )
        self.store_map = store_map

        # system
        self.random = Random(spec.seed)
        self.network = NetworkHandle(spec.network, self)
        loop = asyncio.get_event_loop()
        assert isinstance(loop, SimulatedEventLoop), f"need simulated loop: {loop}"
        self.loop = loop
        self.oracle = SimulatedOracle(loop=self.loop, random=self.random)
        self.tasks = TaskManager(
            owner=self, logger=logger, oracle=self.oracle, on_error=self.on_error
        )
        self.errors: list[BaseException] = []
        self.has_error = asyncio.Event()

        # content
        self.supervisor = SupervisorHandle("supervisor", spec.supervisor, self.oracle, self)
        self.benches_by_name: dict[str, BenchHandle] = {}
        self.benches_by_id: dict[UUID, BenchHandle] = {}
        self.hosts_by_name: dict[str, HostHandle] = {}
        self.users_by_name: dict[str, UserHandle] = {}
        self.machines_by_name: dict[str, MachineHandle] = {}
        self.runtimes_by_name: dict[str, RuntimeHandle] = {}
        self.clients_by_name: dict[str, ClientHandle] = {}
        self.workloads: list[Workload] = []
        self.workloads_by_name: dict[str, Workload] = {}
        self.workloads_by_group: dict[str, list[Workload]] = {}
        self.services_by_id: dict[str, ServiceHandle] = {self.supervisor.id: self.supervisor}

        # status
        self.started_at: datetime | None = None
        self.terminated_at: datetime | None = None

    def __str__(self):
        return f"'{self.id}'"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    def on_error(self, error: BaseException):
        logger.error("simulation.error", simulation=self, exc_info=error)
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

    def get_bench(self, name: str | UUID) -> "BenchHandle":
        if isinstance(name, UUID):
            bench = self.benches_by_id.get(name)
        else:
            bench = self.benches_by_name.get(name)
        assert (
            bench is not None
        ), f"{self!r} has no bench: '{name}' (available: {list(self.benches_by_name)})"
        return bench

    def get_machine(self, name: str) -> "MachineHandle":
        machine = self.machines_by_name.get(name)
        assert (
            machine is not None
        ), f"{self!r} has no machine: '{name}' (available: {list(self.machines_by_name)})"
        return machine

    def get_runtime(self, name: str) -> "RuntimeHandle":
        runtime = self.runtimes_by_name.get(name)
        assert (
            runtime is not None
        ), f"{self!r} has no runtime: '{name}' (available: {list(self.runtimes_by_name)})"
        return runtime

    def get_host(self, name: str) -> "HostHandle":
        host = self.hosts_by_name.get(name)
        assert (
            host is not None
        ), f"{self!r} has no host: '{name}' (available: {list(self.hosts_by_name)})"
        return host

    def get_service(self, id: str) -> ServiceHandle:
        service = self.services_by_id.get(id)
        assert (
            service is not None
        ), f"{self!r} has no service: '{id}' (available: {list(self.services_by_id)})"
        return service

    def get_workload_group(self, name: str) -> list["Workload"]:
        if name not in self.workloads_by_group:
            raise ValueError(f"{self!r} has no workload group: '{name}'")
        return self.workloads_by_group[name]

    def get_bench_id(self, name: str) -> UUID:
        bench = self.get_bench(name)
        return bench.bench_id

    def prepare(self):
        """Initialize the simulation from ths spec."""

        def add_service(service: ServiceHandle) -> ServiceHandle:
            """Add a Service to the simulation."""
            if service.id in self.services_by_id:
                raise ValueError(f"duplicate service id: {service.id} in {self!r}")
            self.services_by_id[service.id] = service
            return service

        def add_user(user_spec: UserSpec) -> UserHandle:
            """Add a User to the simulation."""
            if user_spec.name in self.users_by_name:
                raise ValueError(f"duplicate user name: {user_spec.name} in {self!r}")
            user = UserHandle(
                id=f"user:{user_spec.name}",
                name=user_spec.name,
                spec=user_spec,
                oracle=self.oracle,
                simulation=self,
            )
            self.users_by_name[user_spec.name] = user
            return user

        def add_machine(machine_spec: MachineSpec) -> MachineHandle:
            """Add a Machine to the simulation."""
            if machine_spec.name in self.machines_by_name:
                raise ValueError(f"duplicate machine name: {machine_spec.name} in {self!r}")
            machine = MachineHandle(
                id=f"machine:{machine_spec.name}",
                spec=machine_spec,
                oracle=self.oracle,
                simulation=self,
            )
            self.machines_by_name[machine_spec.name] = machine
            return machine

        def add_client(client_spec: ClientSpec) -> ClientHandle:
            """Add a Client to the simulation."""
            if client_spec.name in self.clients_by_name:
                raise ValueError(f"duplicate client name: {client_spec.name} in {self!r}")
            if client_spec.parent[0] == "user":
                parent = self.users_by_name[client_spec.parent[1]]
            elif client_spec.parent[0] == "machine":
                parent = self.machines_by_name[client_spec.parent[1]]
            else:
                raise ValueError(f"invalid client parent: {client_spec.parent} in {self!r}")
            client = ClientHandle(
                id=f"client:{client_spec.name}",
                spec=client_spec,
                parent=parent,
                simulation=self,
            )
            self.clients_by_name[client_spec.name] = client
            parent.clients_by_name[client_spec.name] = client
            return client

        def add_bench(bench_spec: BenchSpec) -> BenchHandle:
            """Add a Bench to the simulation."""
            if bench_spec.name in self.benches_by_name:
                raise ValueError(f"duplicate bench name: {bench_spec.name} in {self!r}")
            bench = BenchHandle(
                id=f"bench:{bench_spec.name}",
                spec=bench_spec,
                oracle=self.oracle,
                simulation=self,
            )
            self.benches_by_name[bench_spec.name] = bench
            return bench

        def add_runtime(runtime_spec: RuntimeSpec) -> RuntimeHandle:
            """Add a Runtime to the simulation."""
            if runtime_spec.name in self.runtimes_by_name:
                raise ValueError(f"duplicate runtime name: {runtime_spec.name} in {self!r}")
            runtime = RuntimeHandle(
                id=f"runtime:{runtime_spec.name}",
                spec=runtime_spec,
                oracle=self.oracle,
                simulation=self,
            )
            self.runtimes_by_name[runtime_spec.name] = runtime
            add_service(runtime)
            return runtime

        def add_host(host_spec: HostSpec) -> HostHandle:
            """Add a Host to the simulation."""
            if host_spec.bench in self.hosts_by_name:
                raise ValueError(f"duplicate Host for Bench: {host_spec.bench} in {self!r}")
            host = HostHandle(
                id=f"host:{host_spec.bench}",
                spec=host_spec,
                oracle=self.oracle,
                simulation=self,
            )
            self.hosts_by_name[host_spec.bench] = host
            add_service(host)
            return host

        def add_workload(workload_spec: "WorkloadSpec") -> "Workload":
            """Add a Workload to the simulation."""
            from bench.test.simulation.workload import get_workload_cls

            if workload_spec.name in self.workloads_by_name:
                raise ValueError(f"duplicate workload name: {workload_spec.name} in {self!r}")
            workload_cls = get_workload_cls(workload_spec.type)
            workload = workload_cls(
                id=f"workload:{workload_spec.name}",
                spec=workload_spec,
                oracle=self.oracle,
                simulation=self,
            )
            self.workloads.append(workload)
            self.workloads_by_name[workload_spec.name] = workload
            if workload_spec.group:
                if workload_spec.group not in self.workloads_by_group:
                    self.workloads_by_group[workload_spec.group] = []
                self.workloads_by_group[workload_spec.group].append(workload)
            return workload

        # add everything from spec
        for user in self.spec.users:
            add_user(user)
        for bench in self.spec.benches:
            add_bench(bench)
        for machine in self.spec.machines:
            add_machine(machine)
        for runtime in self.spec.runtimes:
            add_runtime(runtime)
        for client in self.spec.clients:
            add_client(client)
        for host in self.spec.hosts:
            add_host(host)
        for workload in self.spec.workloads:
            add_workload(workload)

    async def run(self):
        """Run the simulation."""
        # prepare services and such
        self.prepare()
        # supervisor first (always needed)
        await self.supervisor.start()
        async with SimulatedChannel(
            self.supervisor.service, oracle=self.oracle
        ) as supervisor_channel:
            supervisor_client = SupervisorClient(supervisor_channel)
            # prepare users and their clients
            for user in self.users_by_name.values():
                await user.prepare(supervisor_client)
            for client in self.clients_by_name.values():
                if isinstance(client.parent, UserHandle):
                    await client.prepare(supervisor_client)
            # prepare benches
            for bench in self.benches_by_name.values():
                user = self.users_by_name.get(bench.spec.owner)
                assert user, f"no user {bench.spec.owner} for {bench!r} in {self!r}"
                await bench.prepare(supervisor_client, user.some_client)
                self.benches_by_id[bench.bench_id] = bench
            # prepare machines and their clients
            for machine in self.machines_by_name.values():
                await machine.prepare()
            for client in self.clients_by_name.values():
                if isinstance(client.parent, MachineHandle):
                    await client.prepare(supervisor_client)

        # start services
        await asyncio.gather(*(host.start() for host in self.hosts_by_name.values()))
        # start runtimes
        await asyncio.gather(*(runtime.start() for runtime in self.runtimes_by_name.values()))

        # run workloads until completion
        await asyncio.gather(*(workload.prepare() for workload in self.workloads))
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
            self.tasks.close()
            await self.supervisor.close()
            for host in self.hosts_by_name.values():
                await host.close()
            await self.tasks.wait_closed()
            self.terminated_at = REAL_ORACLE.utc()
