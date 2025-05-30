from typing import TYPE_CHECKING, final, override

from bench.language import Database
from bench.proto import HostClient
from bench.utils.oracle import Oracle

from .service import ServiceHandle
from .spec import HostSpec
from .transport import SimulatedChannel

if TYPE_CHECKING:
    from .simulation import Simulation


@final
class HostHandle(ServiceHandle[HostSpec, "HostService", HostClient]):
    """A Host for a Bench"""

    def __init__(self, id: str, spec: HostSpec, oracle: Oracle, simulation: "Simulation"):
        super().__init__(id, spec, oracle, simulation)

    def __str__(self) -> str:
        return f"{self.spec.bench}"

    def __repr__(self) -> str:
        return f"<HostHandle {self!s}>"

    @override
    async def start(self) -> "HostService":
        self._service = HostService(
            id=self.id,
            bench_id=self.simulation.get_bench_id(self.spec.bench),
            global_database=self.simulation.global_database,
            main_database=self.simulation.main_database,
            network=self.simulation.network.network,
            oracle=self.oracle,
            on_error=self.simulation.on_error,
        )
        await self._service.start()
        return self._service

    @override
    async def get_client(self, channel: SimulatedChannel) -> HostClient:
        return HostClient(channel=channel.channel)

    @override
    async def close(self):
        await super().close()
        # manually decommission databases (bootstrapping problem since the Host session uses the database)
        async with pg_connection(
            self.simulation.global_database, owner=self, autocommit=True
        ) as conn:
            for database in self.service.main_package.get_children(Database):
                if database.external_name:
                    await conn.execute(sqlstr(f'DROP DATABASE "{database.external_name}"'))
