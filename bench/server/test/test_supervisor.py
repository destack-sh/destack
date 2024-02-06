from contextlib import contextmanager
from dataclasses import replace
from uuid import UUID

import grpclib
import pytest
from grpclib.testing import ChannelFor

from bench.language import Client, ReadOptions, User
from bench.language.const import NodeType
from bench.proto import wire
from bench.proto.wire import (
    GlobalSupervisorStub,
    LoginUserRequest,
    LogoutUserRequest,
    ReadNodesRequest,
    RpcMetadata,
    SignupUserRequest,
)
from bench.server.supervisor import GlobalSupervisor
from bench.utils.dt import utcnow_with_tz


@pytest.fixture(scope="module")
async def supervisor(event_loop) -> GlobalSupervisorStub:
    service = GlobalSupervisor()
    await service.start_quick()
    try:
        async with ChannelFor([service]) as channel:
            stub = GlobalSupervisorStub(channel)
            yield stub
    finally:
        service.close()
        await service.wait_closed()


@contextmanager
def raises_grpc_error(status: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    assert exc_info.value.status == status, f"expected {status}, got {exc_info!r}"


async def test_user_auth_flow(supervisor: GlobalSupervisorStub):
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


async def test_cross_user_protection(supervisor: GlobalSupervisorStub):
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
            name=f"{user.name}'s device",
            device_name="pytest",
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


async def test_global_crud_protection(supervisor: GlobalSupervisorStub):
    """Create, read, update and search global nodes directly. Shouldn't be possible for most nodes."""

    pass
    # read user with owned data, unauthorized -> success but empty (except public data)
    # read_user_req = ReadNodesRequest(
    #     roots=[user.to_ref()._to_data()],
    #     options=ReadOptions(descendant_types=[NodeType.CLIENT])._to_data(),
    # )
    # read_user_rep = await supervisor.read_nodes(read_user_req)
    # assert not len(read_user_rep.nodes) == 1
    # assert not read_user_rep.nodes[0].email
    # create User -> fail
    # upsert User -> fail

    # update User.name, authorized -> success
    # user.name = "Testificate"
    # edit = wire.EditData(
    #     type=wire.EditType.UPDATE,
    #     node_type=wire.NodeType.USER,
    #     node=wiring.wrap_some_node(user._to_data()),
    #     properties=[User.name.id],
    # )
    # edit_req = CommitEditsRequest(edits=[edit])
    # await supervisor.commit_edits(edit_req, access_metadata.to_headers())
    #
    # # update User.password_hash, authorized -> fail (system property)
    # user.password_hash = b"bad"
    # edit = wire.EditData(
    #     type=wire.EditType.UPDATE,
    #     node_type=wire.NodeType.USER,
    #     node=wiring.wrap_some_node(user._to_data()),
    #     properties=[User.password_hash.id],
    # )
    # edit_req = CommitEditsRequest(edits=[edit])
    # with raises_grpc_error(grpclib.Status.PERMISSION_DENIED):
    #     _ = await supervisor.commit_edits(edit_req, access_metadata.to_headers())
