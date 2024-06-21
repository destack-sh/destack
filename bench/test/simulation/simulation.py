import asyncio
import time
from itertools import chain
from random import Random
from typing import final

import pytest
import structlog
from opentelemetry import trace

from bench.language import Store
from bench.proto import wire
from bench.proto.wire import (
    ClientData,
    ClientDataIn,
    LoginUserRequest,
    RpcMetadata,
    SignupUserRequest,
    SupervisorClient,
    UserData,
)
from bench.proto.wiring import pack_rpc_headers
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
        self._oracle = REAL_ORACLE
        self._tasks = TaskManager(owner=self, logger=logger, oracle=self._oracle)
        self._loop = SimulatedLoop(base_time_ns=time.time_ns)

        # services
        self._supervisor = SupervisorHandle(spec.supervisor, self._oracle, self)
        self._hosts_by_name: dict[str, HostHandle] = {}
        for host_spec in spec.hosts:
            host = HostHandle(host_spec, self._oracle, self)
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
            # prepare services and such
            #  (use direct channel to supervisor to bootstrap)
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
        self._clients_by_name: dict[str, ClientHandle] = {}
        self._user_data: UserData | None = None

    def __str__(self):
        return self.name

    def __repr__(self):
        return f"<{self.__class__.__name__} {self.name}>"

    @property
    def some_client(self) -> "ClientHandle":
        return next(iter(self._clients_by_name.values()))

    @property
    def user_data(self) -> UserData:
        assert self._user_data is not None, f"{self!r} not ready"
        return self._user_data

    async def prepare(self, supervisor_client: SupervisorClient):
        """Creates the User"""
        client_in = ClientDataIn(
            type=wire.ClientType.BENCH_SERVER, name=f"{self.name}-signup", device_name="test"
        )
        signup_req = SignupUserRequest(
            slug=self.name,
            name=self.name,
            email=f"{self.name}@test.com",
            password=self.name,
            client=client_in,
        )
        signup_rep = await supervisor_client.signup_user(signup_req)
        self._user_data = signup_rep.user


@final
class ClientHandle:
    """A Client to a Bench"""

    def __init__(self, spec: ClientSpec, user: UserHandle, simulation: Simulation):
        self.spec = spec
        self.user = user
        self.simulation = simulation
        self._client_data: ClientData | None = None
        self._access_token: str | None = None
        self._rpc_metadata: RpcMetadata | None = None
        self._rpc_headers: dict[str, str] | None = None

    def __str__(self):
        return self.spec.name

    def __repr__(self):
        return f"<{self.__class__.__name__} {self.spec.name}>"

    @property
    def rpc_metadata(self) -> RpcMetadata:
        assert self._rpc_metadata is not None, f"{self!r} not ready"
        return self._rpc_metadata

    @property
    def rpc_headers(self):
        assert self._rpc_headers is not None, f"{self!r} not ready"
        return self._rpc_headers

    async def prepare(self, supervisor_client: SupervisorClient):
        """Logs in this Client as the User"""
        client_in = ClientDataIn(
            type=wire.ClientType.BENCH_SERVER, name=self.spec.name, device_name="test"
        )
        login_req = LoginUserRequest(
            slug=self.spec.username,
            password=self.spec.username,
            client=client_in,
        )
        login_rep = await supervisor_client.login_user(login_req)
        self._client_data = login_rep.client
        self._access_token = login_rep.access_token
        self._rpc_metadata = RpcMetadata(
            client_type=self._client_data.type,
            client_id=self._client_data.id,
            client_nonce=self._client_data.id,
            client_access_token=self._access_token,
        )
        self._rpc_headers = pack_rpc_headers(self._rpc_metadata)


@final
class Network:
    """A Network for connecting services and clients"""

    def __init__(self, spec: NetworkSpec, simulation: Simulation):
        self.spec = spec
        self.simulation = simulation
        self._channels = {}

    def __str__(self):
        return f"{len(self._channels)} channels"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"


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
        workloads=(WriteBlockTreeSpec(client="alice-1"), ReadPackageSpec(client="alice-1")),
    ),
    SimulationSpec(
        name="multi_client_rw_block_tree",
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
        clients=(
            ClientSpec(name="alice-1", username="alice"),
            ClientSpec(name="alice-2", username="alice"),
        ),
        workloads=(WriteBlockTreeSpec(client="alice-1"), ReadPackageSpec()),
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
