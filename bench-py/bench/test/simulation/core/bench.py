from typing import TYPE_CHECKING, final

from fastuuid import UUID

from bench.language import (
    BENCH_ID,
    BENCH_SLUG,
    SYSTEM_ID,
    SYSTEM_SLUG,
)
from bench.proto import SupervisorClient
from bench.test.simulation.core.user import UserHandle
from bench.utils.oracle import Oracle

from .spec import BenchSpec

if TYPE_CHECKING:
    from .simulation import ClientHandle, Simulation


@final
class BenchHandle:
    """A Host for a Bench"""

    def __init__(self, id: str, spec: BenchSpec, oracle: Oracle, simulation: "Simulation"):
        self.id = id
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

    @property
    def name(self) -> str:
        return self.spec.name

    async def prepare(self, supervisor_client: SupervisorClient, client: "ClientHandle | None"):
        # create bench in supervisor
        if self.spec.name == BENCH_SLUG:
            self._bench_id = BENCH_ID
        elif self.spec.name == SYSTEM_SLUG:
            self._bench_id = SYSTEM_ID
        else:
            assert client is not None, f"need client for {self!r}"
            assert isinstance(client.parent, UserHandle), f"{client!r} is not from a User"
            raise NotImplementedError
