from typing import TYPE_CHECKING, final

from bench.language import Bench, Client, ClientType, Machine, ResourceStatus
from bench.language.core.const import NodeType
from bench.pb2.lang_pb2 import ClientData, MachineData
from bench.system import ACCESS_TOKEN_LENGTH
from bench.utils.func import generate_access_token

from .spec import MachineSpec

if TYPE_CHECKING:
    from .client import ClientHandle
    from .simulation import Simulation


@final
class MachineHandle:
    """A Machine in a Bench"""

    def __init__(self, spec: MachineSpec, simulation: "Simulation") -> None:
        self.spec = spec
        self.simulation = simulation
        self.clients_by_name: dict[str, ClientHandle] = {}
        self._machine_data: MachineData | None = None
        self._client_data: ClientData | None = None
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
        from .session import make_pg_session

        # root session
        session = make_pg_session(self.simulation)
        async with session:
            # create machine and client
            bench = await Bench.select_all().get(slug=self.spec.bench)
            bench._graph.add_types(NodeType.MACHINE, NodeType.CLIENT)
            machine = Machine(parent=bench, name=self.spec.name, status=ResourceStatus.UP)
            session._create(machine)
            client = Client(
                parent=bench,
                type=ClientType.MACHINE,
                name=self.spec.name,
                access_token=generate_access_token(ACCESS_TOKEN_LENGTH),
                machine=machine,
                seen_at=self.simulation.oracle.utc(),
            )
            session._create(client)
            machine.client = client
            await session.commit()
        self._access_token = client.access_token
        self._client_data = client._to_data()
        self._client_data.ClearField("parent_ptr")
        self._machine_data = machine._to_data()
        self._machine_data.ClearField("parent_ptr")
