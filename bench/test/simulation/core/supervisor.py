from typing import TYPE_CHECKING, final, override

from bench.proto import SupervisorClient
from bench.system import HostMap, SupervisorService

from .service import ServiceHandle
from .spec import SupervisorSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    pass


@final
class SupervisorHandle(ServiceHandle[SupervisorSpec, SupervisorService, SupervisorClient]):
    """A global Supervisor"""

    service_cls = SupervisorService
    client_cls = SupervisorClient

    def __repr__(self) -> str:
        return "<SupervisorHandle>"

    @override
    async def start(self) -> SupervisorService:
        service = SupervisorService(
            global_store=self.simulation.global_store,
            store_map=self.simulation.store_map,
            oracle=self.oracle,
            host_map=HostMap({}),
            on_error=self.simulation.on_error,
        )
        await service.start()
        self._service = service
        return service

    @override
    async def get_client(self, channel: SimulatedChannel) -> SupervisorClient:
        return SupervisorClient(channel=channel.channel)
