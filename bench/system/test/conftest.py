import random
import secrets
import string
from dataclasses import dataclass
from typing import TYPE_CHECKING, Mapping

import pytest
from grpclib.testing import ChannelFor

from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.language.user import Client, User
    from bench.proto.wire import ClientOrigin, NodeReferenceData, RpcMetadata, SupervisorStub


@dataclass(slots=True)
class UserHandle:
    user: "User"
    client: "Client"
    subject: "NodeReferenceData"
    origin: "ClientOrigin"
    metadata: "RpcMetadata"

    @property
    def headers(self) -> Mapping[str, str]:
        return self.metadata.to_headers()


async def make_new_user_handle(
    supervisor: "SupervisorStub",
    user: "User",
    *,
    password: str | None = None,
    client_name: str = "Macbook Pro",
) -> UserHandle:
    """Signs up a new user and returns a handle for the user and a client."""

    from bench.language.user import Client
    from bench.proto import wire
    from bench.proto.wire import ClientOrigin, NodeReferenceData, RpcMetadata, SignupUserRequest

    if password is None:
        password = secrets.token_hex(8)
    client = Client(
        parent=user,
        name=f"{user.name}'s {client_name}",
        device_name="pytest",
        last_seen_at=utcnow_with_tz(),
    )
    signup_req = SignupUserRequest(
        id=str(user.id),
        slug=user.slug,
        name=user.name,
        email=user.email,
        password=password,
        client=client._to_data(),
    )
    signup_rep = await supervisor.signup_user(signup_req)
    origin = ClientOrigin(id=str(client.id), nonce=str(random.randint(0, 2**32)))
    subject = NodeReferenceData(
        metatype=wire.StructType.NODE_REFERENCE, type=wire.NodeType.USER, id=str(user.id)
    )
    metadata = RpcMetadata(client_id=str(client.id), client_access_token=signup_rep.access_token)
    return UserHandle(user=user, client=client, origin=origin, subject=subject, metadata=metadata)


async def make_existing_user_handle(
    supervisor: "SupervisorStub", user: "User", *, password: str, client_name: str
) -> UserHandle:
    """Logs in an existing user and returns a handle for the user and a client."""

    from bench.language.user import Client
    from bench.proto import wire
    from bench.proto.wire import ClientOrigin, LoginUserRequest, NodeReferenceData, RpcMetadata

    client = Client(
        parent=user,
        name=f"{user.name}'s {client_name}",
        device_name="pytest",
        last_seen_at=utcnow_with_tz(),
    )
    login_req = LoginUserRequest(
        id=str(user.id),
        slug=user.slug,
        email=user.email,
        password=password,
        client=client._to_data(),
    )
    login_rep = await supervisor.login_user(login_req)
    origin = ClientOrigin(id=str(client.id), nonce=str(random.randint(0, 2**32)))
    subject = NodeReferenceData(
        metatype=wire.StructType.NODE_REFERENCE, type=wire.NodeType.USER, id=str(user.id)
    )
    metadata = RpcMetadata(client_id=str(client.id), client_access_token=login_rep.access_token)
    return UserHandle(user=user, client=client, origin=origin, subject=subject, metadata=metadata)


async def make_random_user_handle(supervisor: "SupervisorStub") -> UserHandle:
    from bench.language import User
    from bench.language.const import UserStatus

    random_slug = "".join(random.choices(string.ascii_letters, k=10)).lower()
    random_email = f"{random_slug}@whatever.com"
    user = User(slug=random_slug, name=random_slug, email=random_email, status=UserStatus.INVITED)
    return await make_new_user_handle(
        supervisor, user, password=secrets.token_hex(8), client_name=secrets.token_hex(8)
    )


@pytest.fixture(scope="function")
async def some_user(supervisor: "SupervisorStub") -> UserHandle:
    return await make_random_user_handle(supervisor)


@pytest.fixture(scope="function")
async def supervisor() -> "SupervisorStub":
    from bench.system.supervisor import Supervisor, SupervisorStub

    service = Supervisor()
    await service.start_quick()
    try:
        async with ChannelFor([service]) as channel:
            stub = SupervisorStub(channel)
            yield stub
    finally:
        service.close()
        await service.wait_closed()
