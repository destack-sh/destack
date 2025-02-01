from typing import TYPE_CHECKING, final
from uuid import UUID

from bench import pb2
from bench.language import NodeReference
from bench.proto import CreateBenchRequest, SupervisorClient
from bench.test.simulation.core.user import UserHandle
from bench.utils.oracle import Oracle

from .spec import BenchSpec

if TYPE_CHECKING:
    from .simulation import ClientHandle, Simulation


@final
class BenchHandle:
    """A Host for a Bench"""

    def __init__(self, spec: BenchSpec, oracle: Oracle, simulation: "Simulation"):
        self.spec = spec
        self.oracle = oracle
        self.simulation = simulation
        self._bench_id: UUID | None = None

    def __str__(self) -> str:
        return f"{self.spec.name}"

    def __repr__(self) -> str:
        return f"<BenchHandle {self!s}>"

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
            slug=self.spec.name,
            region=pb2.Region.REGION_ZURICH,
        )
        create_bench_rep = await supervisor_client.create_bench(
            create_bench_req, metadata=client.rpc_headers
        )
        self._bench_id = UUID(create_bench_rep.bench.id)
