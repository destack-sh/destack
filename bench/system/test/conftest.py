import random
import secrets
import string
from dataclasses import dataclass
from typing import TYPE_CHECKING, Mapping, cast

import pytest
from grpclib.testing import ChannelFor

from bench.proto.wire import (
    ClientDataIn,
    ClientOrigin,
    HostBase,
    HostStub,
    NodeReferenceData,
    RpcMetadata,
    SupervisorBase,
    SupervisorStub,
)
from bench.utils.oracle import get_oracle

if TYPE_CHECKING:
    from bench.language.user import Client, User
    from bench.proto.monkey import _PatchedRpcMetadata


@pytest.fixture(scope="session")
async def supervisor_service():
    from bench.system.supervisor import Supervisor

    service = Supervisor()
    await service.start()
    try:
        yield service
    finally:
        service.close()
        await service.wait_closed()


@pytest.fixture(scope="session")
async def host_service():
    from bench.system.host import HostRouter

    service = HostRouter()
    await service.start()
    try:
        yield service
    finally:
        service.close()
        await service.wait_closed()


# NOTE: we cannot keep gRPC service stubs across function boundaries because pytest-async
#   creates a new loop for each test function, and gRPC services are tied to the loop.
#   :PytestAsyncWeirdness


@pytest.fixture()  # :PytestAsyncWeirdness
async def supervisor(supervisor_service: SupervisorBase):
    async with ChannelFor([supervisor_service]) as channel:
        stub = SupervisorStub(channel)
        yield stub


@pytest.fixture()  # :PytestAsyncWeirdness
async def host(host_service: HostBase):
    async with ChannelFor([host_service]) as channel:
        stub = HostStub(channel)
        yield stub


@dataclass(slots=True)
class UserHandle:
    user: "User"
    client: "Client"
    subject: "NodeReferenceData"
    origin: "ClientOrigin"
    metadata: "RpcMetadata"
    headers: Mapping[str, str]


async def make_new_user_handle(
    supervisor: "SupervisorStub",
    user: "User",
    *,
    password: str | None = None,
    client_name: str = "Macbook Pro",
) -> UserHandle:
    """Signs up a new user and returns a handle for the user and a client."""

    from bench.language import Client, ClientType
    from bench.proto import wire
    from bench.proto.wire import ClientOrigin, NodeReferenceData, RpcMetadata, SignupUserRequest

    if password is None:
        password = secrets.token_hex(8)
    client = Client(
        parent=user,
        type=ClientType.BENCH_WEB,
        name=f"{user.name}'s {client_name}",
        device_name="pytest",
        seen_at=get_oracle().utc(),
    )
    signup_req = SignupUserRequest(
        id=str(user.id),
        slug=cast(str, user.slug),
        name=user.name,
        email=user.email,
        password=password,
        client=cast(ClientDataIn, client._to_data()),
    )
    signup_rep = await supervisor.signup_user(signup_req)
    origin = ClientOrigin(
        type=wire.ClientType(client.type), id=str(client.id), nonce=str(random.randint(0, 2**32))
    )
    subject = NodeReferenceData(
        metatype=wire.ObjectType.NODE_REFERENCE, type=wire.NodeType.USER, id=str(user.id)
    )
    metadata = RpcMetadata(
        client_type=wire.ClientType(client.type),
        client_id=str(client.id),
        client_access_token=signup_rep.access_token,
    )
    handle = UserHandle(
        user=user,
        client=client,
        origin=origin,
        subject=subject,
        metadata=metadata,
        headers=metadata.to_headers(),  # type: ignore
    )
    return handle


async def make_existing_user_handle(
    supervisor: "SupervisorStub", user: "User", *, password: str, client_name: str
) -> UserHandle:
    """Logs in an existing user and returns a handle for the user and a client."""

    from bench.language import Client, ClientType
    from bench.proto import wire
    from bench.proto.wire import ClientOrigin, LoginUserRequest, NodeReferenceData, RpcMetadata

    client = Client(
        parent=user,
        type=ClientType.BENCH_WEB,
        name=f"{user.name}'s {client_name}",
        device_name="pytest",
        seen_at=get_oracle().utc(),
    )
    login_req = LoginUserRequest(
        id=str(user.id),
        slug=cast(str, user.slug),
        email=user.email,
        password=password,
        client=cast(ClientDataIn, client._to_data()),
    )
    login_rep = await supervisor.login_user(login_req)
    origin = ClientOrigin(id=str(client.id), nonce=str(random.randint(0, 2**32)))
    subject = NodeReferenceData(
        metatype=wire.ObjectType.NODE_REFERENCE, type=wire.NodeType.USER, id=str(user.id)
    )
    metadata = RpcMetadata(client_id=str(client.id), client_access_token=login_rep.access_token)
    handle = UserHandle(
        user=user,
        client=client,
        origin=origin,
        subject=subject,
        metadata=metadata,
        headers=cast("_PatchedRpcMetadata", metadata).to_headers(),
    )
    return handle


async def make_random_user_handle(supervisor: "SupervisorStub") -> UserHandle:
    from bench.language import User
    from bench.language.const import UserStatus

    random_slug = "".join(random.choices(string.ascii_letters, k=10)).lower()
    random_email = f"{random_slug}@symbolx.com"
    user = User(slug=random_slug, name=random_slug, email=random_email, status=UserStatus.INVITED)
    return await make_new_user_handle(
        supervisor, user, password=secrets.token_hex(8), client_name=secrets.token_hex(8)
    )


@pytest.fixture()
async def some_user(supervisor: "SupervisorStub") -> UserHandle:
    return await make_random_user_handle(supervisor)
