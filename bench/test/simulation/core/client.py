from typing import TYPE_CHECKING, final

from bench import pb2
from bench.language import ClientType
from bench.proto import (
    ClientData,
    ClientDataIn,
    ClientOriginData,
    LoginUserRequest,
    RpcMetadata,
    SupervisorClient,
    pack_enum,
    pack_rpc_headers,
)

from .spec import ClientSpec

if TYPE_CHECKING:
    from .computer import ComputerHandle
    from .simulation import Simulation
    from .user import UserHandle


@final
class ClientHandle:
    """A Client to a Bench"""

    def __init__(
        self,
        id: str,
        spec: ClientSpec,
        parent: "UserHandle | ComputerHandle",
        simulation: "Simulation",
    ):
        self.id = id
        self.spec = spec
        self.parent = parent
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
            metatype=pb2.ObjectType.OBJECT_TYPE_CLIENT_ORIGIN,
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
        from .computer import ComputerHandle
        from .user import UserHandle

        if isinstance(self.parent, UserHandle):
            client_in = ClientDataIn(
                type=pack_enum(ClientType, self.spec.type),
                name=self.spec.name,
                device_name=self.spec.name,
            )
            login_req = LoginUserRequest(
                slug=self.spec.parent[1], password=self.spec.parent[1], client=client_in
            )
            login_rep = await supervisor_client.login_user(login_req)
            self._client_data = login_rep.client
            self._access_token = login_rep.access_token
        elif isinstance(self.parent, ComputerHandle):
            self._client_data = self.parent.client_data
            self._access_token = self.parent.access_token
        else:
            raise ValueError(f"invalid client parent: {self.parent!r} in {self!r}")
        self._rpc_metadata = RpcMetadata(
            client_type=self._client_data.type,
            client_id=self._client_data.id,
            client_nonce=self._client_data.id,
            client_access_token=self._access_token,
        )
        self._rpc_headers = pack_rpc_headers(self._rpc_metadata)
