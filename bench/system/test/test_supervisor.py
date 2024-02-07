from contextlib import contextmanager
from dataclasses import replace, dataclass
import random
import string
from typing import TYPE_CHECKING
from uuid import UUID

import grpclib
import pytest
from grpclib.testing import ChannelFor

from bench.language import Client, ReadOptions, User
from bench.language.const import NodeType, ABOVE_SOURCE_NODE_TYPES, PUBLIC_NODE_TYPES, EDIT_TYPES
from bench.language.expression import A
from bench.proto import wire, wiring
from bench.proto.wire import (
    SupervisorStub,
    LoginUserRequest,
    LogoutUserRequest,
    ReadNodesRequest,
    RpcMetadata,
    SignupUserRequest,
    EditData,
    ClientOrigin,
    CommitEditsRequest,
    SearchNodesRequest,
    AggregationOp,
    AggregateNodesRequest,
)
from bench.system.supervisor import Supervisor
from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.language.test.fabricator import Fabricator


@pytest.fixture(scope="module")
async def supervisor(event_loop) -> SupervisorStub:
    service = Supervisor()
    await service.start_quick()
    try:
        async with ChannelFor([service]) as channel:
            stub = SupervisorStub(channel)
            yield stub
    finally:
        service.close()
        await service.wait_closed()


@contextmanager
def raises_grpc_error(status: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    assert exc_info.value.status == status, f"expected {status}, got {exc_info!r}"


async def test_user_auth_flow(supervisor: SupervisorStub):
    """Create a User, login and logout. Read back user data at various points."""

    user = User(slug="test", name="Test", email="test@symbolx.com")
    client = Client(parent=user, name="test", device_name="pytest", last_seen_at=utcnow_with_tz())

    # signup -> success
    signup_req = SignupUserRequest(
        id=str(user.id),
        slug=user.slug,
        name=user.name,
        email=user.email,
        client=client._to_data(),
        password="Password123!",
    )
    signup_rep = await supervisor.signup_user(signup_req)
    assert signup_rep.user.slug == str(user.slug)
    assert signup_rep.user.id == str(user.id)

    # login, invalid password -> fail
    login_req = LoginUserRequest(slug=user.slug, password="bad", client=client._to_data())
    with raises_grpc_error(grpclib.Status.INVALID_ARGUMENT):
        _ = await supervisor.login_user(login_req)

    # login, wrong password -> fail
    login_req = LoginUserRequest(
        slug=user.slug, password="321Password!!!", client=client._to_data()
    )
    with raises_grpc_error(grpclib.Status.INVALID_ARGUMENT):
        _ = await supervisor.login_user(login_req)

    # login, correct password -> success
    login_req = LoginUserRequest(slug=user.slug, password="Password123!", client=client._to_data())
    login_rep = await supervisor.login_user(login_req)
    assert login_rep.access_token

    # read user with sensitive data, authorized -> success
    read_user_req = ReadNodesRequest(
        roots=[user.to_ref()._to_data()],
        options=ReadOptions(
            include_properties=[User.email], descendant_types=[NodeType.CLIENT]
        )._to_data(),
    )
    access_metadata = RpcMetadata(
        client_id=str(client.id),
        client_kind=wire.ClientKind.USER,
        client_access_token=login_rep.access_token,
    )
    read_user_rep = await supervisor.read_nodes(
        read_user_req, metadata=access_metadata.to_headers()
    )
    assert len(read_user_rep.nodes) == 2
    assert read_user_rep.nodes[0].user.email == user.email

    # logout, invalid token -> fail
    with raises_grpc_error(grpclib.Status.UNAUTHENTICATED):
        bad_access_metadata = replace(access_metadata, client_access_token="bad")
        _ = await supervisor.logout_user(
            LogoutUserRequest(), metadata=bad_access_metadata.to_headers()
        )

    # logout, valid token -> success
    _ = await supervisor.logout_user(LogoutUserRequest(), metadata=access_metadata.to_headers())

    # read user, logged out, expired token -> fail
    with raises_grpc_error(grpclib.Status.UNAUTHENTICATED):
        _ = await supervisor.read_nodes(read_user_req, metadata=access_metadata.to_headers())


async def test_cross_user_protection(supervisor: SupervisorStub):
    """Users can only make some actions on their own behalf."""

    user_a = User(slug="alice", name="Alice", email="alice@bench.app")
    user_b = User(slug="bob", name="Bob", email="bob@bench.app")
    user_c = User(slug="carol", name="Carol", email="carol@bench.app")
    all_users = (user_a, user_b, user_c)

    # signup users
    metadata_by_user: dict[UUID, RpcMetadata] = {}
    for user in all_users:
        client = Client(
            parent=user,
            name=f"{user.name}'s MacBook Pro",
            device_name="macbook",
            last_seen_at=utcnow_with_tz(),
        )
        signup_req = SignupUserRequest(
            id=str(user.id),
            slug=user.slug,
            name=user.name,
            email=user.email,
            password=f"Password{user.slug}123!",
            client=client._to_data(),
        )
        signup_rep = await supervisor.signup_user(signup_req)
        assert signup_rep.user.slug == user.slug
        metadata_by_user[user.id] = RpcMetadata(
            client_id=str(client.id),
            client_kind=wire.ClientKind.USER,
            client_access_token=signup_rep.access_token,
        )

    # cross-test user access/actions
    for actor in all_users:
        for target in all_users:
            is_self = actor == target

            # request our own and everyone else's data
            sensitive_properties = (User.email, User.password_salt, User.password_hash)
            read_user_req = ReadNodesRequest(
                roots=[target.to_ref()._to_data()],
                options=ReadOptions(include_properties=sensitive_properties)._to_data(),
            )
            read_user_rep = await supervisor.read_nodes(
                read_user_req, metadata=metadata_by_user[actor.id].to_headers()
            )
            read_target = read_user_rep.nodes[0].user
            assert read_target.slug == target.slug
            if is_self:  # we should be able to read our own sensitive data
                assert read_target.email == target.email
                assert read_target.password_salt
                assert read_target.password_hash
            else:  # but not others'
                assert not read_target.email
                assert not read_target.password_salt
                assert not read_target.password_hash

            # create new client
            new_client = Client(
                parent=actor,
                name=f"{actor.name}'s Toaster",
                device_name="toaster",
                last_seen_at=utcnow_with_tz(),
            )


@dataclass
class UserHandle:
    user: User
    client: Client
    origin: ClientOrigin
    metadata: RpcMetadata


async def make_user_handle(supervisor: SupervisorStub, user: User) -> UserHandle:
    client = Client(
        parent=user, name=user.name + "'s iPad", device_name="pytest", last_seen_at=utcnow_with_tz()
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
    metadata = RpcMetadata(
        client_id=str(client.id),
        client_kind=wire.ClientKind.USER,
        client_access_token=signup_rep.access_token,
    )
    return UserHandle(user=user, client=client, origin=origin, metadata=metadata)


async def make_random_user_handle(supervisor: SupervisorStub) -> UserHandle:
    random_slug = "".join(random.choices(string.ascii_letters, k=10))
    random_email = f"{random_slug}@whatever.com"
    user = User(slug=random_slug, name=random_slug, email=random_email)
    return await make_user_handle(supervisor, user)


@pytest.fixture(scope="module")
async def some_user(supervisor: SupervisorStub) -> UserHandle:
    return await make_random_user_handle(supervisor)


@pytest.mark.parametrize("node_type", PUBLIC_NODE_TYPES, ids=lambda t: t.name)
async def test_public_node_read(
    node_type: NodeType, some_user: UserHandle, supervisor: SupervisorStub
):
    """Public nodes should be readable, but not directly editable in any way."""

    packed_node_type = wiring.pack_enum(NodeType, node_type)

    # search (and count)
    search_req = SearchNodesRequest(node_type=packed_node_type, count=True)
    search_rep = await supervisor.search_nodes(search_req, metadata=some_user.metadata.to_headers())
    # can we assert anything here?

    # search with filter (and count)
    search_req = SearchNodesRequest(node_type=packed_node_type)
    search_rep = await supervisor.search_nodes(search_req, metadata=some_user.metadata.to_headers())
    # ...?

    # aggregate: exists
    aggregate_req = AggregateNodesRequest(
        node_type=packed_node_type, aggregation=A(op=AggregationOp.EXISTS)._to_data()
    )
    aggregate_rep = await supervisor.aggregate_nodes(
        aggregate_req, metadata=some_user.metadata.to_headers()
    )
    assert isinstance(aggregate_rep.aggregation.exists, bool)

    # aggregate: count
    aggregate_req = AggregateNodesRequest(
        node_type=packed_node_type, aggregation=A(op=AggregationOp.COUNT)._to_data()
    )
    aggregate_rep = await supervisor.aggregate_nodes(
        aggregate_req, metadata=some_user.metadata.to_headers()
    )
    assert isinstance(aggregate_rep.aggregation.count, int)


@pytest.mark.parametrize("node_type", (nt for nt in ABOVE_SOURCE_NODE_TYPES), ids=lambda t: t.name)
async def test_global_node_edit(
    node_type: NodeType,
    some_user: UserHandle,
    supervisor: SupervisorStub,
    fabricator: "Fabricator",
):
    """'Global' nodes should not be directly editable by regular users."""

    node = fabricator.fabricate(node_type)
    node_data = wiring.pack_node(node)
    for edit_type in EDIT_TYPES:
        edit = EditData(
            type=edit_type,
            node_type=node_data.node_type,
            node=wiring.wrap_some_node(node_data),
            origin=some_user.origin,
        )
        commit_req = CommitEditsRequest(edits=[edit])
        with raises_grpc_error(grpclib.Status.PERMISSION_DENIED):
            _ = await supervisor.commit_edits(commit_req, metadata=some_user.metadata.to_headers())
