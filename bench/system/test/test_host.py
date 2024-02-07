from grpclib.testing import ChannelFor
import pytest

from bench.proto.wire import BenchHostStub
from bench.system.host import BenchHost


@pytest.fixture(scope="module")
async def host(event_loop) -> BenchHostStub:
    service = BenchHost()
    await service.start_quick()
    try:
        async with ChannelFor([service]) as channel:
            stub = BenchHostStub(channel)
            yield stub
    finally:
        service.close()
        await service.wait_closed()
