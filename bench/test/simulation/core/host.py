from typing import TYPE_CHECKING, final, override
from uuid import UUID

from bench import pb2
from bench.language import NodeReference
from bench.proto import CreateBenchRequest, HostClient, SupervisorClient
from bench.sql import pg_connection, sqlstr
from bench.system import HostService
from bench.test.simulation.core.user import UserHandle
from bench.utils.oracle import Oracle

from .service import ServiceHandle
from .spec import HostSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    from .simulation import ClientHandle, Simulation


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
        assert isinstance(client.parent, UserHandle), f"{client!r} is not from a User"
        create_bench_req = CreateBenchRequest(
            owner=NodeReference._ref_data_from_node_data(client.parent.user_data),
            is_main=True,
            slug=self.spec.bench.name,
            region=pb2.Region.REGION_ZURICH,
        )
        create_bench_rep = await supervisor_client.create_bench(
            create_bench_req, metadata=client.rpc_headers
        )
        self._bench_id = UUID(create_bench_rep.bench.id)

    @override
    async def _do_start(self) -> HostService:
        service = HostService(
            bench_id=self.bench_id,
            global_store=self.simulation.global_store,
            regional_store=self.simulation.regional_store,
            oracle=self.oracle,
            on_error=self.simulation.on_error,
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
