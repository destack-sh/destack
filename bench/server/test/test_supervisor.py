from contextlib import contextmanager
from dataclasses import replace

import grpclib
import pytest
from grpclib.testing import ChannelFor

from bench.language import Client, User
from bench.proto import wire
from bench.proto.wire import (
    GlobalSupervisorStub,
    LoginUserRequest,
    SignupUserRequest,
    ReadNodesRequest,
    RpcMetadata,
    LogoutUserRequest,
    ReadOptionsData,
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


async def test_user_signup_flow(supervisor: GlobalSupervisorStub):
    """Tests user account creation, login & logout."""

    user = User(slug="test", email="test@symbolx.com")
    client = Client(parent=user, name="test", device_name="pytest", last_seen_at=utcnow_with_tz())

    # signup -> success
    signup_req = SignupUserRequest(
        user=user._to_data(), client=client._to_data(), password="Password123!"
    )
    signup_rep = await supervisor.signup_user(signup_req)
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

    # read user without sensitive data, no token -> success
    read_user_req = ReadNodesRequest(
        roots=[user.as_reference._to_data()],
        options=ReadOptionsData(descendant_types=[wire.NodeType.CLIENT]),
    )
    with raises_grpc_error(grpclib.Status.UNAUTHENTICATED):
        _ = await supervisor.read_nodes(read_user_req)

    # read user with sensitive data, no token -> fail
    read_user_req = ReadNodesRequest(
        roots=[user.as_reference._to_data()],
        options=ReadOptionsData(descendant_types=[wire.NodeType.CLIENT], include_sensitive=True),
    )
    with raises_grpc_error(grpclib.Status.PERMISSION_DENIED):
        _ = await supervisor.read_nodes(read_user_req)

    # read user, valid token -> success
    access_metadata = RpcMetadata(
        client_id=str(client.id),
        client_kind=wire.ClientKind.USER,
        client_token=login_rep.access_token,
    )
    await supervisor.read_nodes(read_user_req, access_metadata.to_headers())

    # logout, invalid token -> fail
    with raises_grpc_error(grpclib.Status.UNAUTHENTICATED):
        bad_access_metadata = replace(access_metadata, client_token="bad")
        _ = await supervisor.logout_user(LogoutUserRequest(), bad_access_metadata.to_headers())

    # logout, valid token -> success
    await supervisor.logout_user(LogoutUserRequest(), access_metadata.to_headers())

    # read user, logged out, "valid" (but expired) token -> fail
    with raises_grpc_error(grpclib.Status.UNAUTHENTICATED):
        _ = await supervisor.read_nodes(read_user_req, access_metadata.to_headers())
