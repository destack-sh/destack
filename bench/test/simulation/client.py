from typing import TYPE_CHECKING, final

from bench.language import ClientType, NodeReference
from bench.proto import wire
from bench.proto.wire import (
    ClientData,
    ClientDataIn,
    ClientOriginData,
    LoginUserRequest,
    RpcMetadata,
    SignupUserRequest,
    SupervisorClient,
    UserData,
)
from bench.proto.wiring import pack_enum, pack_rpc_headers, unpack_object
from bench.test.simulation.spec import ClientSpec

if TYPE_CHECKING:
    from bench.test.simulation.simulation import Simulation


@final
class UserHandle:
    """A User"""

    def __init__(self, name: str, simulation: "Simulation"):
        self.name = name
        self.simulation = simulation
        self._clients_by_name: dict[str, ClientHandle] = {}
        self._user_data: UserData | None = None
        self._user_ptr: NodeReference | None = None

    def __str__(self):
        return self.name

    def __repr__(self):
        return f"<{self.__class__.__name__} {self.name}>"

    @property
    def some_client(self) -> "ClientHandle":
        return next(iter(self._clients_by_name.values()))

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
            type=wire.ClientType.BENCH_SERVER, name=f"{self.name}-signup", device_name="test"
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
        self._user_ptr = unpack_object(
            NodeReference._ref_data_from_node_data(self._user_data),
            expect=NodeReference,
            supergraph=None,
        )


@final
class ClientHandle:
    """A Client to a Bench"""

    def __init__(self, spec: ClientSpec, user: UserHandle, simulation: "Simulation"):
        self.spec = spec
        self.user = user
        self.simulation = simulation
        self._client_data: ClientData | None = None
        self._access_token: str | None = None
        self._rpc_metadata: RpcMetadata | None = None
        self._rpc_headers: dict[str, str] | None = None

    def __str__(self):
        return self.spec.name

    def __repr__(self):
        return f"<{self.__class__.__name__} {self.spec.name}>"

    @property
    def client_data(self) -> ClientData:
        assert self._client_data is not None, f"{self!r} not ready"
        return self._client_data

    def to_origin(self, *, nonce: str | None) -> ClientOriginData:
        return ClientOriginData(
            metatype=wire.ObjectType.CLIENT_ORIGIN,
            type=self.client_data.type,
            id=self.client_data.id,
            nonce=nonce or self.client_data.id,
        )

    @property
    def rpc_metadata(self) -> RpcMetadata:
        assert self._rpc_metadata is not None, f"{self!r} not ready"
        return self._rpc_metadata

    @property
    def rpc_headers(self):
        assert self._rpc_headers is not None, f"{self!r} not ready"
        return self._rpc_headers

    async def prepare(self, supervisor_client: SupervisorClient):
        """Logs in this Client as the User"""
        client_in = ClientDataIn(
            type=pack_enum(ClientType, self.spec.type), name=self.spec.name, device_name="test"
        )
        login_req = LoginUserRequest(
            slug=self.spec.username,
            password=self.spec.username,
            client=client_in,
        )
        login_rep = await supervisor_client.login_user(login_req)
        self._client_data = login_rep.client
        self._access_token = login_rep.access_token
        self._rpc_metadata = RpcMetadata(
            client_type=self._client_data.type,
            client_id=self._client_data.id,
            client_nonce=self._client_data.id,
            client_access_token=self._access_token,
        )
        self._rpc_headers = pack_rpc_headers(self._rpc_metadata)
