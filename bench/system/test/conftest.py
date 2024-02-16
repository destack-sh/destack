from dataclasses import dataclass
import random
import string
from typing import Mapping, TYPE_CHECKING

import pytest

from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.language.user import User, Client
    from bench.proto.wire import ClientOrigin, RpcMetadata, SupervisorStub, NodeReferenceData


@dataclass
class UserHandle:
    user: "User"
    client: "Client"
    subject: "NodeReferenceData"
    origin: "ClientOrigin"
    metadata: "RpcMetadata"

    @property
    def headers(self) -> Mapping[str, str]:
        return self.metadata.to_headers()


async def make_user_handle(supervisor: "SupervisorStub", user: "User") -> UserHandle:
    from bench.language.user import Client
    from bench.proto import wire
    from bench.proto.wire import ClientOrigin, RpcMetadata, SignupUserRequest, NodeReferenceData

    client = Client(
        parent=user,
        name=user.name + "'s MacBook Pro",
        device_name="pytest",
        last_seen_at=utcnow_with_tz(),
    )
    signup_req = SignupUserRequest(
        id=str(user.id),
        slug=user.slug,
        name=user.name,
        email=user.email,
        password="Password123!",
        client=client._to_data(),
    )
    signup_rep = await supervisor.signup_user(signup_req)
    origin = ClientOrigin(
        id=str(client.id), kind=wire.ClientKind.USER, nonce=str(random.randint(0, 2**32))
    )
    subject = NodeReferenceData(
        metatype=wire.StructType.NODE_REFERENCE, type=wire.NodeType.USER, id=str(user.id)
    )
    metadata = RpcMetadata(
        client_id=str(client.id),
        client_kind=wire.ClientKind.USER,
        client_access_token=signup_rep.access_token,
    )
    return UserHandle(user=user, client=client, origin=origin, subject=subject, metadata=metadata)


async def make_random_user_handle(supervisor: "SupervisorStub") -> UserHandle:
    from bench.language import User
    from bench.language.const import UserStatus

    random_slug = "".join(random.choices(string.ascii_letters, k=10))
    random_email = f"{random_slug}@whatever.com"
    user = User(slug=random_slug, name=random_slug, email=random_email, status=UserStatus.INVITED)
    return await make_user_handle(supervisor, user)


@pytest.fixture(scope="function")
async def some_user(supervisor: "SupervisorStub") -> UserHandle:
    return await make_random_user_handle(supervisor)
