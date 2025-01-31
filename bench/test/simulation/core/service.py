import abc
import enum
from typing import TYPE_CHECKING, final

from bench.proto import ServiceBase
from bench.utils.oracle import Oracle

from .spec import ServiceSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    from .simulation import ClientHandle, Simulation


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
