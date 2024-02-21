from grpclib.testing import ChannelFor
import pytest

from bench.proto.wire import HostStub
from bench.system.host import Host


@pytest.fixture(scope="function")
async def host() -> HostStub:
    service = Host()
    await service.start_quick()
    try:
        async with ChannelFor([service]) as channel:
            stub = HostStub(channel)
            yield stub
    finally:
        service.close()
        await service.wait_closed()
