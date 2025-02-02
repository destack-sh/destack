import asyncio
from typing import TYPE_CHECKING, NamedTuple, final, override

from grpclib.client import Channel

from bench.proto.network import Network

from .service import ServiceHandle
from .spec import NetworkSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    from .simulation import Simulation


class ConnectionPair(NamedTuple):
    """A pair of connected services"""

    client_name: str
    service_id: str


class SimulatedNetwork(Network):
    """A Network for connecting Services and Clients"""

    def __init__(self, network: "NetworkHandle", simulation: "Simulation"):
        self.network = network
        self.simulation = simulation

    @override
    async def get_channel(self, connection_uri: str, *, source_id: str) -> Channel:
        # trim prefix and port (we expect something like simulation://<id>:<port>)
        service_id = connection_uri.rsplit("/", 1)[1]
        service_id = service_id.rsplit(":", 1)[0]
        assert service_id, f"unexpected connection uri: {connection_uri}"
        service = self.simulation.get_service(service_id)
        channel = await self.network.connect(source_id, service)
        return channel.channel


@final
class NetworkHandle:
    """A Network for connecting services and clients"""

    def __init__(self, spec: NetworkSpec, simulation: "Simulation"):
        self.spec = spec
        self.simulation = simulation
        self.network = SimulatedNetwork(self, simulation)
        self.channels: dict[ConnectionPair, SimulatedChannel] = {}

    def __str__(self):
        return f"{len(self.channels)} channels"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    async def connect(self, source_id: str, service: ServiceHandle) -> SimulatedChannel:
        """Connects a client to a service"""
        pair = ConnectionPair(source_id, service.id)
        channel = self.channels.get(pair)
        if channel is not None:
            return channel
        channel = SimulatedChannel(services=(service.service,), oracle=self.simulation.oracle)
        self.channels[pair] = channel
        await channel.open()
        return channel

    def close(self):
        for channel in self.channels.values():
            channel.close()

    async def wait_closed(self):
        await asyncio.gather(*(channel.wait_closed() for channel in self.channels.values()))
        self.channels.clear()
