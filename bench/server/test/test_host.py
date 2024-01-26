from grpclib.testing import ChannelFor
import pytest

from bench.proto.wire import PackageHostStub
from bench.server.host import PackageHost


@pytest.fixture(scope="module")
async def host(event_loop) -> PackageHostStub:
    service = PackageHost()
    await service.start_quick()
    try:
        async with ChannelFor([service]) as channel:
            stub = PackageHostStub(channel)
            yield stub
    finally:
        service.close()
        await service.wait_closed()
