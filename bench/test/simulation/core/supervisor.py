from typing import TYPE_CHECKING, final, override
from uuid import UUID

from bench.language import Region
from bench.proto import SupervisorClient
from bench.system import CreateBenchOptions, HostInfo, HostMap, SupervisorService

from .service import ServiceHandle
from .spec import SupervisorSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    from .simulation import Simulation


class SimulatedHostMap(HostMap):
    def __init__(self, simulation: "Simulation"):
        self.simulation = simulation

    def get(self, bench_id: UUID, region: Region) -> HostInfo:
        bench = self.simulation.get_bench(bench_id)
        host = self.simulation.get_host(bench.name)
        domain = f"{host.id}"  # :SimulatedNetwork
        return HostInfo(host_domain=domain, grpc_port=0, grpc_web_port=0, ssl=False)


@final
class SupervisorHandle(ServiceHandle[SupervisorSpec, SupervisorService, SupervisorClient]):
    """A global Supervisor"""

    service_cls = SupervisorService
    client_cls = SupervisorClient

    def __repr__(self) -> str:
        return "<SupervisorHandle>"

    @override
    async def start(self) -> SupervisorService:
        self._service = SupervisorService(
            id=self.id,
            global_store=self.simulation.global_store,
            store_map=self.simulation.store_map,
            network=self.simulation.network.network,
            oracle=self.oracle,
            host_map=SimulatedHostMap(self.simulation),
            on_error=self.simulation.on_error,
            # we create Computers manually (via ComputerHandle) in simulation
            create_bench_options=CreateBenchOptions(create_computer_scaler=False),
        )
        await self._service.start()
        return self._service

    @override
    async def get_client(self, channel: SimulatedChannel) -> SupervisorClient:
        return SupervisorClient(channel=channel.channel)
