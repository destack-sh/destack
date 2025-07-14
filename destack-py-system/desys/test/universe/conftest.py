import pytest_asyncio

from destack.grpc import (
    NullNetwork,
    UniverseClient,
)
from destack.language import WORLD_ORACLE, DatabaseInfo
from desys.sharding import DATABASE_PROVIDER, GALAXY_PROVIDER
from desys.test.simulation import SimulatedChannel


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def universe_service(
    omni_postgres_database: DatabaseInfo,
):
    from desys.universe import UniverseService

    universe_service = UniverseService(
        id="universe",
        global_database=omni_postgres_database,
        network=NullNetwork(),
        oracle=WORLD_ORACLE,
        galaxy_provider=GALAXY_PROVIDER,
        database_provider=DATABASE_PROVIDER,
    )
    await universe_service.start()
    yield universe_service
    universe_service.stop()
    await universe_service.wait_stopped()


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def universe(universe_service):
    async with SimulatedChannel(services=(universe_service,), oracle=WORLD_ORACLE) as channel:
        yield UniverseClient(channel=channel)
