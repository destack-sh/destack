import abc
import enum
from typing import TYPE_CHECKING, final, override
from uuid import UUID

from bench.language import NodeReference
from bench.proto import wire
from bench.proto.services import ServiceBase
from bench.proto.wire import CreateBenchRequest, HostClient, SupervisorClient
from bench.sql.client import pg_connection
from bench.sql.engine import sqlstr
from bench.system.host.service import HostService
from bench.system.supervisor.service import SupervisorService
from bench.system.utils.sharding import HostMap
from bench.test.simulation.spec import HostSpec, ServiceSpec, SupervisorSpec
from bench.test.simulation.transport import SimulatedChannel
from bench.utils.oracle import Oracle

if TYPE_CHECKING:
    from bench.test.simulation.simulation import ClientHandle, Simulation


class ServiceStatus(enum.Enum):
    UP = 1
    DEGRADING = 2
    DOWN = 3
    RECOVERING = 4


class ServiceHandle[SpecT: ServiceSpec, S: ServiceBase, C: object](abc.ABC):
    """Wrapper for a simulated service we can monkey around with"""

    service_cls: type[S]
    client_cls: type[C]

    def __init__(self, id: str, spec: SpecT, oracle: Oracle, simulation: "Simulation"):
        self.id = id
        self.spec = spec
        self.oracle = oracle
        self.simulation = simulation
        self._service: S | None = None

    @property
    def service(self) -> S:
        assert self._service is not None, f"f{self!r} not started yet"
        return self._service

    @abc.abstractmethod
    async def _do_start(self) -> S: ...

    @abc.abstractmethod
    async def _make_client(self, channel: SimulatedChannel) -> C: ...

    async def connect(self, client: "ClientHandle") -> C:
        channel = await self.simulation.network.connect(client, self)
        return await self._make_client(channel)

    @final
    async def start(self):
        # TODO :Test: degrade, fail & recover services according to spec
        self._service = await self._do_start()

    def close(self):
        if self._service is not None:
            self._service.close()

    async def wait_closed(self):
        if self._service is not None:
            await self._service.wait_closed()


@final
class SupervisorHandle(ServiceHandle[SupervisorSpec, SupervisorService, SupervisorClient]):
    """A global Supervisor"""

    service_cls = SupervisorService
    client_cls = SupervisorClient

    def __repr__(self) -> str:
        return "<SupervisorHandle>"

    @override
    async def _do_start(self):
        service = SupervisorService(
            global_store=self.simulation.global_store, oracle=self.oracle, host_map=HostMap({})
        )
        await service.start()
        return service

    @override
    async def _make_client(self, channel: SimulatedChannel):
        return SupervisorClient(channel=channel.channel)


@final
class HostHandle(ServiceHandle[HostSpec, HostService, HostClient]):
    """A Host for a Bench"""

    def __init__(self, id: str, spec: HostSpec, oracle: Oracle, simulation: "Simulation"):
        super().__init__(id, spec, oracle, simulation)
        self._bench_id: UUID | None = None

    def __str__(self) -> str:
        return f"{self.spec.bench.name}"

    def __repr__(self) -> str:
        return f"<HostHandle {self!s}>"

    @property
    def bench_id(self) -> UUID:
        assert self._bench_id is not None, f"{self!r} not ready"
        return self._bench_id

    async def prepare(self, supervisor_client: SupervisorClient, client: "ClientHandle"):
        # create bench in supervisor
        create_bench_req = CreateBenchRequest(
            owner=NodeReference._ref_data_from_node_data(client.user.user_data),
            is_main=True,
            slug=self.spec.bench.name,
            region=wire.Region.REGION_ZURICH,
        )
        create_bench_rep = await supervisor_client.create_bench(
            create_bench_req, metadata=client.rpc_headers
        )
        self._bench_id = UUID(create_bench_rep.bench.id)

    @override
    async def _do_start(self) -> HostService:
        service = HostService(
            bench_id=self.bench_id, global_store=self.simulation.global_store, oracle=self.oracle
        )
        await service.start()
        return service

    @override
    async def _make_client(self, channel: SimulatedChannel) -> HostClient:
        return HostClient(channel=channel.channel)

    @override
    async def wait_closed(self):
        await super().wait_closed()
        # manually decommission stores (bootstrapping problem since the Host session uses the store)
        async with pg_connection(self.simulation.global_store, autocommit=True) as conn:
            for store in self.service.bench.stores:
                await conn.execute(sqlstr(f'DROP DATABASE "{store.external_name}"'))
