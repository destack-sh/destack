from typing import TYPE_CHECKING, final
from uuid import UUID

from bench.language import ClientType
from bench.pb2 import RuntimeClient
from bench.runtime import RuntimeService
from bench.runtime.base import RuntimeThreadMode
from bench.utils.oracle import Oracle

from .service import ServiceHandle
from .spec import RuntimeSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    from .client import ClientHandle
    from .simulation import Simulation


@final
class RuntimeHandle(ServiceHandle[RuntimeSpec, RuntimeService, RuntimeClient]):
    """A (Runtime) Machine in a Bench"""

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
        machine = self.simulation.get_machine(self.spec.machine)
        supervisor = await self.simulation.supervisor.connect(self.spec.name)
        self._service = RuntimeService(
            id=self.id,
            supervisor=supervisor,
            network=self.simulation.network.network,
            oracle=self.simulation.oracle,
            bench_id=self.simulation.get_bench_id(machine.spec.bench),
            client_type=ClientType.MACHINE,
            client_id=UUID(machine.client_data.id),
            client_access_token=machine.access_token,
            machine_id=UUID(machine.machine_data.id),
            max_threads=self.spec.max_threads,
            max_concurrency_per_thread=self.spec.max_concurrency_per_thread,
            mode=RuntimeThreadMode.LOCAL,
        )
        await self._service.start()
        return self._service

    async def get_client(self, channel: SimulatedChannel) -> RuntimeClient:
        return RuntimeClient(channel=channel.channel)
