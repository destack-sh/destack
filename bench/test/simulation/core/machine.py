from typing import TYPE_CHECKING, final

from more_itertools import first

from bench.language import (
    Bench,
    Client,
    ClientType,
    Machine,
    MachineType,
    NodeType,
    Package,
    ResourceStatus,
)
from bench.pb2 import ClientData, MachineData
from bench.utils.func import generate_access_token
from bench.utils.oracle import Oracle

from .spec import MachineSpec

if TYPE_CHECKING:
    from .client import ClientHandle
    from .simulation import Simulation


@final
class MachineHandle:
    """A (Runtime) Machine in a Bench"""

    def __init__(
        self, id: str, spec: MachineSpec, oracle: Oracle, simulation: "Simulation"
    ) -> None:
        self.id = id
        self.spec = spec
        self.oracle = oracle
        self.simulation = simulation
        self.clients_by_name: dict[str, ClientHandle] = {}
        self._client_data: ClientData | None = None
        self._machine_data: MachineData | None = None
        self._access_token: str | None = None

    def __str__(self):
        return self.spec.name

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    @property
    def machine_data(self) -> MachineData:
        assert self._machine_data is not None, f"{self!r} not ready"
        return self._machine_data

    @property
    def client_data(self) -> ClientData:
        assert self._client_data is not None, f"{self!r} not ready"
        return self._client_data

    @property
    def access_token(self) -> str:
        assert self._access_token is not None, f"{self!r} not ready"
        return self._access_token

    async def prepare(self):
        """Create the Machine and Client."""
        from bench.system import ACCESS_TOKEN_LENGTH

        from .session import make_pg_session

        # root session
        session = make_pg_session(self.simulation)
        async with session:
            # create machine and client
            bench = await Bench.select_all().include_descendants(Package).get(slug=self.spec.bench)
            bench._graph.add_types(NodeType.MACHINE, NodeType.CLIENT)
            runtime = first(
                (
                    runtime
                    for runtime in self.simulation.runtimes_by_name.values()
                    if runtime.spec.machine == self.spec.name
                ),
                None,
            )
            # add connection url if we have a runtime :SimulatedNetwork
            grpc_url = f"simulation://{runtime.id}:0" if runtime else None
            package = bench.package
            assert package is not None, f"no main package for {bench!r}"
            machine = Machine(
                parent=package,
                name=self.spec.name,
                type=MachineType.RUNTIME,
                status=ResourceStatus.AVAILABLE,
                grpc_url=grpc_url,
            )
            session.create(machine)
            client = Client(
                parent=bench,
                type=ClientType.MACHINE,
                name=self.spec.name,
                access_token=generate_access_token(ACCESS_TOKEN_LENGTH),
                machine=machine,
                seen_at=self.simulation.oracle.utc(),
            )
            session.create(client)
            machine.client = client
            await session.commit()
        self._access_token = client.access_token
        self._client_data = client._to_proto()
        self._client_data.ClearField("parent_ptr")  # type: ignore
        self._machine_data = machine._to_proto()
        self._machine_data.ClearField("parent_ptr")  # type: ignore
