from grpclib.testing import ChannelFor
import pytest

from bench.proto.wire import BenchHostStub
from bench.system.host import BenchHost


@pytest.fixture(scope="function")
async def host() -> BenchHostStub:
    service = BenchHost()
    await service.start_quick()
    try:
        async with ChannelFor([service]) as channel:
            stub = BenchHostStub(channel)
            yield stub
    finally:
        service.close()
        await service.wait_closed()
