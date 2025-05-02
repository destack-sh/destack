import asyncio
import gc
from contextlib import contextmanager
from datetime import datetime
from random import Random
from typing import TYPE_CHECKING, final
from uuid import UUID

import pytest
import structlog
from opentelemetry import trace

from bench.language import (
    BENCH_ID,
    BENCH_SLUG,
    REGION,
    SYSTEM_ID,
    SYSTEM_SLUG,
    NodeArea,
    Store,
)
from bench.proto import SupervisorClient
from bench.sql.client import get_pg_pool_by_external_name, pg_connection
from bench.sql.engine import sqlstr
from bench.sql.graph import BUILTIN_GLOBAL_SCHEMA, BUILTIN_REGIONAL_SCHEMA
from bench.system import (
    StoreMap,
    create_system_benches,
    global_store_from_env,
    pg_engine_from_store,
)
from bench.utils.oracle import REAL_ORACLE
from bench.utils.task import TaskManager

from .bench import BenchHandle
from .client import ClientHandle
from .computer import ComputerHandle
from .host import HostHandle
from .network import NetworkHandle
from .oracle import SimulatedEventLoop, SimulatedOracle
from .runtime import RuntimeHandle
from .service import ServiceHandle
from .spec import (
    BenchSpec,
    ClientSpec,
    ComputerSpec,
    HostSpec,
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

        # content
        self.supervisor = SupervisorHandle("supervisor", spec.supervisor, self.oracle, self)
        self.benches_by_name: dict[str, BenchHandle] = {}
        self.benches_by_id: dict[UUID, BenchHandle] = {}
        self.hosts_by_name: dict[str, HostHandle] = {}
        self.users_by_name: dict[str, UserHandle] = {}
        self.computers_by_name: dict[str, ComputerHandle] = {}
        self.runtimes_by_name: dict[str, RuntimeHandle] = {}
        self.clients_by_name: dict[str, ClientHandle] = {}
        self.workloads: list[Workload] = []
        self.workloads_by_name: dict[str, Workload] = {}
        self.workloads_by_group: dict[str, list[Workload]] = {}
        self.services_by_id: dict[str, ServiceHandle] = {self.supervisor.id: self.supervisor}

        # status
        self.started_at: datetime | None = None
        self.terminated_at: datetime | None = None
        self.suppressed_error_types: list[type[BaseException]] = []
        self.suppressed_errors: list[BaseException] = []
        self.errors: list[BaseException] = []
        self.has_error = asyncio.Event()

    def __str__(self):
        return f"'{self.id}'"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    @contextmanager
    def raises[E: BaseException](self, *exc_types: type[E]):
        """Wraps pytest.raises to also suppress expected errors in the simulation."""
        prev_suppressed = list(self.suppressed_error_types)
        self.suppressed_error_types.extend(exc_types)
        with pytest.raises(exc_types) as exc_info:
            yield exc_info
        self.suppressed_error_types = prev_suppressed

    def on_error(self, error: BaseException):
        if any(isinstance(error, ex) for ex in self.suppressed_error_types):
            logger.debug("simulation.error.suppressed", simulation=self, exc_info=error)
            self.suppressed_errors.append(error)
        else:
            logger.error("simulation.error", simulation=self, exc_info=error)
            self.errors.append(error)
            self.has_error.set()

    def get_user(self, name: str) -> "UserHandle":
        user = self.users_by_name.get(name)
        if user is None:
            raise LookupError(
                f"{self!r} has no user: '{name}' (available: {list(self.users_by_name)})"
            )
        return user

    def get_client(self, name: str) -> "ClientHandle":
        client = self.clients_by_name.get(name)
        if client is None:
            raise LookupError(
                f"{self!r} has no client: '{name}' (available: {list(self.clients_by_name)})"
            )
        return client

    def get_bench(self, name: str | UUID) -> "BenchHandle":
        if isinstance(name, UUID):
            bench = self.benches_by_id.get(name)
        else:
            bench = self.benches_by_name.get(name)
        if bench is None:
            raise LookupError(
                f"{self!r} has no bench: '{name}' (available: {list(self.benches_by_name)})"
            )
        return bench

    def get_computer(self, name: str) -> "ComputerHandle":
        computer = self.computers_by_name.get(name)
        if computer is None:
            raise LookupError(
                f"{self!r} has no computer: '{name}' (available: {list(self.computers_by_name)})"
            )
        return computer

    def get_runtime(self, name: str) -> "RuntimeHandle":
        runtime = self.runtimes_by_name.get(name)
        if runtime is None:
            raise LookupError(
                f"{self!r} has no runtime: '{name}' (available: {list(self.runtimes_by_name)})"
            )
        return runtime

    def get_host(self, name: str) -> "HostHandle":
        host = self.hosts_by_name.get(name)
        if host is None:
            raise LookupError(
                f"{self!r} has no host: '{name}' (available: {list(self.hosts_by_name)})"
            )
        return host

    def get_service(self, id: str) -> ServiceHandle:
        service = self.services_by_id.get(id)
        if service is None:
            raise LookupError(
                f"{self!r} has no service: '{id}' (available: {list(self.services_by_id)})"
            )
        return service

    def get_workload_group(self, name: str) -> list["Workload"]:
        if name not in self.workloads_by_group:
            raise ValueError(f"{self!r} has no workload group: '{name}'")
        return self.workloads_by_group[name]

    def get_bench_id(self, name: str) -> UUID:
        bench = self.get_bench(name)
        return bench.bench_id

    async def prepare(self):
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

        def add_computer(computer_spec: ComputerSpec) -> ComputerHandle:
            """Add a Computer to the simulation."""
            if computer_spec.name in self.computers_by_name:
                raise ValueError(f"duplicate computer name: {computer_spec.name} in {self!r}")
            computer = ComputerHandle(
                id=f"computer:{computer_spec.name}",
                spec=computer_spec,
                oracle=self.oracle,
                simulation=self,
            )
            self.computers_by_name[computer_spec.name] = computer
            return computer

        def add_client(client_spec: ClientSpec) -> ClientHandle:
            """Add a Client to the simulation."""
            if client_spec.name in self.clients_by_name:
                raise ValueError(f"duplicate client name: {client_spec.name} in {self!r}")
            if client_spec.parent[0] == "user":
                parent = self.users_by_name[client_spec.parent[1]]
            elif client_spec.parent[0] == "computer":
                parent = self.computers_by_name[client_spec.parent[1]]
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
        for computer in self.spec.computers:
            add_computer(computer)
        for runtime in self.spec.runtimes:
            add_runtime(runtime)
        for client in self.spec.clients:
            add_client(client)
        for host in self.spec.hosts:
            add_host(host)
        for workload in self.spec.workloads:
            add_workload(workload)

        # system
        if self.spec.system:
            # maybe this should happen in a SystemHandle or such?
            add_bench(BenchSpec(name=SYSTEM_SLUG, owner="system"))
            add_host(HostSpec(bench=SYSTEM_SLUG))
            add_bench(BenchSpec(name=BENCH_SLUG, owner="system"))
            add_host(HostSpec(bench=BENCH_SLUG))

    async def wait_idle(self, min_idle_time: float = 0.1):
        """Wait until all services are idle for at least min_idle_time."""
        idle_since = None
        while not self.has_error.is_set():
            if all(service.is_idle for service in self.services_by_id.values()):
                if idle_since is None:
                    idle_since = self.oracle.utc()
                elif (self.oracle.utc() - idle_since).total_seconds() >= min_idle_time:
                    break
            else:
                idle_since = None
            await asyncio.sleep(0.01)  # ('busy' waiting is easiest here)

    async def run(self):
        """Run the simulation."""

        # system
        if self.spec.system:
            await create_system_benches(
                region=REGION,
                global_store=self.global_store,
                global_pg_engine=self.global_pg_engine,
                regional_store=self.regional_store,
                regional_pg_engine=self.regional_pg_engine,
            )
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
                await bench.prepare(supervisor_client, user.some_client if user else None)
                self.benches_by_id[bench.bench_id] = bench
            # prepare computers and their clients
            for computer in self.computers_by_name.values():
                await computer.prepare()
            for client in self.clients_by_name.values():
                if isinstance(client.parent, ComputerHandle):
                    await client.prepare(supervisor_client)

        # start hosts
        await asyncio.gather(*(host.start() for host in self.hosts_by_name.values()))
        # start runtimes
        await asyncio.gather(*(runtime.start() for runtime in self.runtimes_by_name.values()))

        # run workloads until completion
        await asyncio.gather(*(workload.prepare() for workload in self.workloads))
        try:
            self.started_at = REAL_ORACLE.utc()
            with tracer.start_as_current_span("simulation.run"):
                await asyncio.gather(*(workload.run() for workload in self.workloads))
                # wait until services (and network?) are idle (at the same time)
                await self.wait_idle()
                logger.info("simulation.run", simulation=self, span="current")
            # and run checks
            with tracer.start_as_current_span("simulation.check"):
                await asyncio.gather(*(workload.check() for workload in self.workloads))
                logger.info("simulation.check", simulation=self, span="current")
        except Exception as e:
            logger.error("simulation.error", simulation=self, exc_info=e)
            raise
        finally:
            # cleanup
            for runtime in self.runtimes_by_name.values():
                await runtime.close()
            for host in self.hosts_by_name.values():
                await host.close()
            await self.supervisor.close()
            self.tasks.close()
            await self.tasks.wait_closed()
            self.terminated_at = REAL_ORACLE.utc()
            # raise for any? errors
            if self.errors:
                if len(self.errors) == 1:
                    raise self.errors[0]
                else:
                    raise RuntimeError(f"multiple errors in {self!r}: {self.errors}")


@tracer.start_as_current_span("simulation")
async def run_simulation(spec: SimulationSpec):
    """Run a Simulation"""
    from bench.test.fixtures import (
        create_test_db,
        make_global_store,
        make_regional_store,
    )

    simulation_id = get_simulation_id(spec)
    span = trace.get_current_span()
    span.set_attribute("simulation.name", spec.name)
    span.set_attribute("simulation.id", simulation_id)
    log = logger.bind(simulation=spec.name)

    # config
    global_store = make_global_store(f"test-{simulation_id}-global")
    regional_store = make_regional_store(f"test-{simulation_id}-regional")
    store_map = StoreMap({"*": regional_store})
    simulation = Simulation(
        id=simulation_id,
        spec=spec,
        global_store=global_store,
        regional_store=regional_store,
        store_map=store_map,
    )

    try:
        # setup
        with tracer.start_as_current_span("simulation.prepare"):
            await simulation.prepare()
            await create_test_db(global_store, BUILTIN_GLOBAL_SCHEMA)
            await create_test_db(regional_store, BUILTIN_REGIONAL_SCHEMA)
            log.info("simulation.prepare", span="current")
        # run
        with tracer.start_as_current_span("simulation.run"):
            await simulation.run()
            log.info("simulation.run", span="current")
    finally:
        # teardown
        for database_name in (
            global_store.external_name,
            regional_store.external_name,
            f"test-{BENCH_ID}",
            f"test-{SYSTEM_ID}",
        ):
            if not database_name:
                continue
            pool = get_pg_pool_by_external_name(database_name)
            if pool:
                await pool.close()
            async with pg_connection(
                global_store_from_env(), owner=global_store, autocommit=True
            ) as conn:
                await conn.execute(sqlstr(f'DROP DATABASE IF EXISTS "{database_name}"'))

        # cleanup
        del spec
        del global_store
        del regional_store
        del store_map
        del simulation
        gc.collect()
        log.info("simulation.terminate", span="current")
