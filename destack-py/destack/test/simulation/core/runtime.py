from typing import TYPE_CHECKING, final

from fastuuid import UUID

from destack.language import ClientType
from destack.pb2 import RuntimeClient
from destack.utils.oracle import Oracle

from .service import ServiceHandle
from .spec import RuntimeSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    from destack.runtime import RuntimeService

    from .client import ClientHandle
    from .simulation import Simulation


@final
class RuntimeHandle(ServiceHandle[RuntimeSpec, "RuntimeService", RuntimeClient]):
    """A (Runtime) Machine in a Destack"""

    def __init__(
        self, id: str, spec: RuntimeSpec, oracle: Oracle, simulation: "Simulation"
    ) -> None:
        super().__init__(id, spec, oracle, simulation)
        self.clients_by_name: dict[str, ClientHandle] = {}

    def __str__(self):
        return self.spec.name

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    async def start(self) -> "RuntimeService":
        machine = self.simulation.get_machine(self.spec.machine)
        supervisor = await self.simulation.supervisor.connect(self.spec.name)
        self._service = RuntimeService(
            id=self.id,
            supervisor=supervisor,
            network=self.simulation.network.network,
            oracle=self.simulation.oracle,
            space_id=self.simulation.get_space_id(machine.spec.destack),
            client_type=ClientType.MACHINE,
            client_id=UUID(machine.client_data.id),
            client_access_token=machine.access_token,
            machine_id=UUID(machine.machine_data.id),
            max_processs=self.spec.max_processs,
            mode=RuntimeProcessMode.LOCAL,
            on_error=self.simulation.on_error,
        )
        await self._service.start()
        return self._service

    async def get_client(self, channel: SimulatedChannel) -> RuntimeClient:
        return RuntimeClient(channel=channel.channel)
