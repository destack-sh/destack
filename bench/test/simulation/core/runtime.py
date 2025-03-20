from typing import TYPE_CHECKING, final
from uuid import UUID

from bench.language import ClientType
from bench.pb2 import RuntimeClient
from bench.runtime import RuntimeService
from bench.runtime.base import RuntimeProcessMode
from bench.utils.oracle import Oracle

from .service import ServiceHandle
from .spec import RuntimeSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    from .client import ClientHandle
    from .simulation import Simulation


@final
class RuntimeHandle(ServiceHandle[RuntimeSpec, RuntimeService, RuntimeClient]):
    """A (Runtime) Computer in a Bench"""

    def __init__(
        self, id: str, spec: RuntimeSpec, oracle: Oracle, simulation: "Simulation"
    ) -> None:
        super().__init__(id, spec, oracle, simulation)
        self.clients_by_name: dict[str, ClientHandle] = {}

    def __str__(self):
        return self.spec.name

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    async def start(self) -> RuntimeService:
        computer = self.simulation.get_computer(self.spec.computer)
        supervisor = await self.simulation.supervisor.connect(self.spec.name)
        self._service = RuntimeService(
            id=self.id,
            supervisor=supervisor,
            network=self.simulation.network.network,
            oracle=self.simulation.oracle,
            bench_id=self.simulation.get_bench_id(computer.spec.bench),
            client_type=ClientType.COMPUTER,
            client_id=UUID(computer.client_data.id),
            client_access_token=computer.access_token,
            computer_id=UUID(computer.computer_data.id),
            max_processs=self.spec.max_processs,
            mode=RuntimeProcessMode.LOCAL,
            on_error=self.simulation.on_error,
        )
        await self._service.start()
        return self._service

    async def get_client(self, channel: SimulatedChannel) -> RuntimeClient:
        return RuntimeClient(channel=channel.channel)
