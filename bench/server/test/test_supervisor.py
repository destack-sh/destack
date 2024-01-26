import grpclib
from grpclib.testing import ChannelFor
import pytest

from bench.language import User, Client
from bench.proto.wire import (
    GlobalSupervisorStub,
    SignupUserRequest,
    LoginUserRequest,
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


async def test_user_signup_flow(supervisor: GlobalSupervisorStub, event_loop):
    user = User(slug="test", email="test@symbolx.com")
    client = Client(parent=user, name="test", device_name="pytest", last_seen_at=utcnow_with_tz())

    # signup
    signup_req = SignupUserRequest(
        user=user._to_wire(), client=client._to_wire(), password="Password123!"
    )
    signup_rep = await supervisor.signup_user(signup_req)
    assert signup_rep.user.id == str(user.id)

    # login - wrong password
    login_req = LoginUserRequest(slug=user.slug, password="wrong", client=client._to_wire())
    with pytest.raises(grpclib.GRPCError):
        _ = await supervisor.login_user(login_req)

    # login - correct password
    login_req = LoginUserRequest(slug=user.slug, password="Password123!", client=client._to_wire())
    login_rep = await supervisor.login_user(login_req)
    assert login_rep.access_token
