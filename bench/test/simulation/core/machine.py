from typing import TYPE_CHECKING, final

from bench.pb2.lang_pb2 import ClientData, MachineData

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
        raise NotImplementedError("nocheckin")
