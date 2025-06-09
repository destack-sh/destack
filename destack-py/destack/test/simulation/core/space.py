from typing import TYPE_CHECKING, final

from fastuuid import UUID

from destack.language import (
    DESTACK_ID,
    DESTACK_SLUG,
    SYSTEM_ID,
    SYSTEM_SLUG,
)
from destack.proto import SupervisorClient
from destack.test.simulation.core.user import UserHandle
from destack.utils.oracle import Oracle

from .spec import DestackSpec

if TYPE_CHECKING:
    from .simulation import ClientHandle, Simulation


@final
class SpaceHandle:
    """A Host for a Destack"""

    def __init__(self, id: str, spec: DestackSpec, oracle: Oracle, simulation: "Simulation"):
        self.id = id
        self.spec = spec
        self.oracle = oracle
        self.simulation = simulation
        self._space_id: UUID | None = None

    def __str__(self) -> str:
        return f"{self.spec.name}"

    def __repr__(self) -> str:
        return f"<SpaceHandle {self!s}>"

    @property
    def space_id(self) -> UUID:
        assert self._space_id is not None, f"{self!r} not ready"
        return self._space_id

    @property
    def name(self) -> str:
        return self.spec.name

    async def prepare(self, supervisor_client: SupervisorClient, client: "ClientHandle | None"):
        # create destack in supervisor
        if self.spec.name == DESTACK_SLUG:
            self._space_id = DESTACK_ID
        elif self.spec.name == SYSTEM_SLUG:
            self._space_id = SYSTEM_ID
        else:
            assert client is not None, f"need client for {self!r}"
            assert isinstance(client.parent, UserHandle), f"{client!r} is not from a User"
            raise NotImplementedError
