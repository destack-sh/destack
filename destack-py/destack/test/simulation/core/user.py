from typing import TYPE_CHECKING, final

from more_itertools import first

from destack.language import NodeReference
from destack.proto import SupervisorClient, UserData
from destack.utils.oracle import Oracle

from .spec import UserSpec

if TYPE_CHECKING:
    from .client import ClientHandle
    from .simulation import Simulation


@final
class UserHandle:
    """A User"""

    def __init__(
        self, id: str, name: str, spec: UserSpec, oracle: Oracle, simulation: "Simulation"
    ):
        self.id = id
        self.name = name
        self.spec = spec
        self.oracle = oracle
        self.simulation = simulation
        self.clients_by_name: dict[str, ClientHandle] = {}
        self._user_data: UserData | None = None
        self._user_ptr: NodeReference | None = None

    def __str__(self):
        return self.name

    def __repr__(self):
        return f"<{self.__class__.__name__} {self.name}>"

    @property
    def some_client(self) -> "ClientHandle":
        client = first(iter(self.clients_by_name.values()), None)
        assert client is not None, f"{self!r} has no clients"
        return client

    @property
    def user_data(self) -> UserData:
        assert self._user_data is not None, f"{self!r} not ready"
        return self._user_data

    @property
    def user_ptr(self) -> NodeReference:
        assert self._user_ptr is not None, f"{self!r} not ready"
        return self._user_ptr

    async def prepare(self, supervisor_client: SupervisorClient):
        """Creates the User"""
        raise NotImplementedError
