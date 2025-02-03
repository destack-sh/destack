import abc
import enum
from typing import TYPE_CHECKING

from bench.proto import ServiceBase
from bench.utils.oracle import Oracle

from .spec import ServiceSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    from .simulation import Simulation


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
    async def start(self) -> S:
        """Start the service (*and* set self._service)."""
        ...

    @abc.abstractmethod
    async def get_client(self, channel: SimulatedChannel) -> C:
        """Get a client for the service."""
        ...

    async def connect(self, source_id: str) -> C:
        """Connect to the service via a client."""
        channel = await self.simulation.network.connect(source_id, self)
        return await self.get_client(channel)

    async def close(self):
        """Close the service."""
        if self._service is not None:
            self._service.stop()
            await self._service.wait_stopped()
