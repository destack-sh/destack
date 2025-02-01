from typing import TYPE_CHECKING, final

from more_itertools import first

from bench import pb2
from bench.language import NodeReference
from bench.proto import (
    ClientDataIn,
    SignupUserRequest,
    SupervisorClient,
    UserData,
    unpack_builtin_object,
)
from bench.utils.oracle import Oracle

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
        client_in = ClientDataIn(
            type=pb2.ClientType.CLIENT_TYPE_MACHINE,
            name=f"{self.name}-signup",
            device_name="test",
        )
        signup_req = SignupUserRequest(
            slug=self.name,
            name=self.name,
            email=f"{self.name}@test.com",
            password=self.name,
            client=client_in,
        )
        signup_rep = await supervisor_client.signup_user(signup_req)
        self._user_data = signup_rep.user
        self._user_ptr = unpack_builtin_object(
            NodeReference._ref_data_from_node_data(self._user_data),
            expect=NodeReference,
            supergraph=None,
        )
