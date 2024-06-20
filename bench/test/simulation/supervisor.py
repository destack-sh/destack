from dataclasses import replace
from typing import cast

from grpclib import Status as GRPCStatus

from bench.language import Client, ReadOptions, User
from bench.language.const import (
    ClientType,
    NodeType,
)
from bench.language.property import Property
from bench.language.user import UserStatus
from bench.proto.wire import (
    ClientDataIn,
    GetNodesRequest,
    LoginUserRequest,
    LogoutUserRequest,
    RpcMetadata,
    SignupUserRequest,
    SupervisorClient,
)
from bench.test.fixtures import raises_grpc_error


async def test_user_registration(supervisor: SupervisorClient):
    """Create a User, login and logout. Read back data to confirm."""

    user = User(slug="test", name="Test", email="test@symbolx.com", status=UserStatus.INVITED)
    assert user.slug is not None and user.email is not None
    client = Client(
        parent=user,
        type=ClientType.BENCH_WEB,
        name="test",
        device_name="pytest",
        seen_at=get_oracle().utc(),
    )

    # signup -> success
    signup_req = SignupUserRequest(
        id=str(user.id),
        slug=user.slug,
        name=user.name,
        email=user.email,
        client=cast(ClientDataIn, client._to_data()),
        password="Password123!",
    )
    signup_rep = await supervisor.signup_user(signup_req)
    assert signup_rep.user.slug == str(user.slug)
    assert signup_rep.user.id == str(user.id)

    # login, invalid password -> fail
    login_req = LoginUserRequest(
        slug=user.slug, password="bad", client=cast(ClientDataIn, client._to_data())
    )
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        _ = await supervisor.login_user(login_req)

    # login, wrong password -> fail
    login_req = LoginUserRequest(
        slug=user.slug, password="321Password!!!", client=cast(ClientDataIn, client._to_data())
    )
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        _ = await supervisor.login_user(login_req)

    # login, correct password -> success
    login_req = LoginUserRequest(
        slug=user.slug, password="Password123!", client=cast(ClientDataIn, client._to_data())
    )
    login_rep = await supervisor.login_user(login_req)
    assert login_rep.access_token

    # read user with sensitive data, authorized -> success
    options = ReadOptions(
        include_properties=[cast(Property, User.email)],
        descendant_types=[NodeType.CLIENT, NodeType.HANDLE],
    )._to_data()
    read_user_req = GetNodesRequest(roots=[user._to_ref_data()], options=options)
    access_metadata = RpcMetadata(
        client_id=str(client.id), client_access_token=login_rep.access_token
    )
    access_headers = access_metadata.to_headers()  # type: ignore
    read_user_rep = await supervisor.get_nodes(read_user_req, metadata=access_headers)
    assert len(read_user_rep.nodes) == 3
    assert read_user_rep.nodes[0].user.email == user.email
    assert read_user_rep.nodes[0].user.main_handle_ptr
    assert read_user_rep.nodes[0].user.main_handle_ptr.id == read_user_rep.nodes[2].handle.id
    assert read_user_rep.nodes[1].client.device_name == client.device_name
    assert read_user_rep.nodes[2].handle.slug == user.slug

    # logout, invalid token -> fail
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        bad_access_metadata = replace(access_metadata, client_access_token="bad")
        bad_access_headers = bad_access_metadata.to_headers()  # type: ignore
        _ = await supervisor.logout_user(LogoutUserRequest(), metadata=bad_access_headers)

    # logout, valid token -> success
    _ = await supervisor.logout_user(LogoutUserRequest(), metadata=access_headers)

    # read user, logged out, expired token -> fail
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        _ = await supervisor.get_nodes(read_user_req, metadata=access_headers)
