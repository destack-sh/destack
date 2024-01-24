from datetime import datetime

from grpclib.testing import ChannelFor
import pytest

from bench.language import User, Client
from bench.proto.wire import GlobalSupervisorStub, SignupUserRequest
from bench.server.supervisor import GlobalSupervisor


@pytest.fixture(scope="module")
async def supervisor() -> GlobalSupervisorStub:
    service = GlobalSupervisor()
    async with ChannelFor([service]) as channel:
        stub = GlobalSupervisorStub(channel)
        yield stub


async def test_user_signup_flow(supervisor: GlobalSupervisorStub):
    user = User(slug="test", email="test@symbolx.com")
    client = Client(parent=user, name="test", device_name="pytest", last_seen_at=datetime.utcnow())
    signup_rep = await supervisor.signup_user(
        SignupUserRequest(user=user._to_wire(), client=client._to_wire(), password="password1")
    )
