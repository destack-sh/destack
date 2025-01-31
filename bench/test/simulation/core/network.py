import asyncio
from typing import TYPE_CHECKING, NamedTuple, final

from .client import ClientHandle
from .service import ServiceHandle
from .spec import NetworkSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    from .simulation import Simulation


class ConnectionPair(NamedTuple):
    """A pair of connected services"""

    client_name: str
    service_id: str


@final
class Network:
    """A Network for connecting services and clients"""

    def __init__(self, spec: NetworkSpec, simulation: "Simulation"):
        self.spec = spec
        self.simulation = simulation
        self._channels: dict[ConnectionPair, SimulatedChannel] = {}

    def __str__(self):
        return f"{len(self._channels)} channels"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    async def connect(self, client: ClientHandle, service: ServiceHandle) -> SimulatedChannel:
        """Connects a client to a service"""
        pair = ConnectionPair(client.spec.name, service.id)
        channel = self._channels.get(pair)
        if channel is not None:
            return channel
        channel = SimulatedChannel(services=(service.service,), oracle=self.simulation.oracle)
        self._channels[pair] = channel
        await channel.open()
        return channel

    def close(self):
        for channel in self._channels.values():
            channel.close()

    async def wait_closed(self):
        await asyncio.gather(*(channel.wait_closed() for channel in self._channels.values()))
        self._channels.clear()
