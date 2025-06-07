from typing import TYPE_CHECKING, final, override

from fastuuid import UUID

from bench.language import Region
from bench.proto import SupervisorClient

from .service import ServiceHandle
from .spec import SupervisorSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    from bench.system import CreateBenchOptions, HostInfo, SupervisorService

    from .simulation import Simulation


class SimulatedHostMap:
    def __init__(self, simulation: "Simulation"):
        self.simulation = simulation

    def get(self, space_id: UUID, region: Region) -> "HostInfo":
        bench = self.simulation.get_bench(space_id)
        host = self.simulation.get_host(bench.name)
        domain = f"{host.id}"  # :SimulatedNetwork
        return HostInfo(host_domain=domain, grpc_port=0, grpc_web_port=0, ssl=False)


@final
class SupervisorHandle(ServiceHandle[SupervisorSpec, "SupervisorService", SupervisorClient]):
    """A global Supervisor"""

    service_cls = "SupervisorService"
    client_cls = SupervisorClient

    def __repr__(self) -> str:
        return "<SupervisorHandle>"

    @override
    async def start(self) -> "SupervisorService":
        self._service = SupervisorService(
            id=self.id,
            global_database=self.simulation.global_database,
            database_provider=self.simulation.database_provider,
            network=self.simulation.network.network,
            oracle=self.oracle,
            cell_provider=SimulatedHostMap(self.simulation),
            on_error=self.simulation.on_error,
            # we create Machines manually (via MachineHandle) in simulation
            create_bench_options=CreateBenchOptions(create_machine_scaler=False),
        )
        await self._service.start()
        return self._service

    @override
    async def get_client(self, channel: SimulatedChannel) -> SupervisorClient:
        return SupervisorClient(channel=channel.channel)
