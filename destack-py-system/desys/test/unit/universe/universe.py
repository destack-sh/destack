import pytest
from grpclib import Status as GRPCStatus

from destack import pb2
from destack.language import Client, ClientType, DatabaseInfo, Session
from destack.proto import (
    LoginUserRequest,
    LogoutUserRequest,
    NullNetwork,
    RpcMetadata,
    SignupUserRequest,
    UniverseClient,
    pack_rpc_headers,
)
from destack.test.fixtures import raises_grpc_error
from destack.utils.oracle import REAL_ORACLE
from desys.sharding import DATABASE_PROVIDER, GALAXY_PROVIDER
from desys.test.simulation.core import SimulatedChannel


@pytest.fixture
async def universe_service(
    global_postgres_database: DatabaseInfo, spatial_postgres_database: DatabaseInfo
):
    from desys.universe import UniverseService

    universe_service = UniverseService(
        id="universe",
        global_database=global_postgres_database,
        network=NullNetwork(),
        oracle=REAL_ORACLE,
        galaxy_provider=GALAXY_PROVIDER,
        database_provider=DATABASE_PROVIDER,
    )
    await universe_service.start()
    yield universe_service
    universe_service.stop()
    await universe_service.wait_stopped()


@pytest.fixture
async def universe(universe_service):
    async with SimulatedChannel(services=(universe_service,), oracle=REAL_ORACLE) as channel:
        yield UniverseClient(channel=channel)


async def test_user_registration(destack: UniverseClient):
    """Create a User, login and logout. Try some wrong passwords and tokens. Read back data to confirm."""

    user_slug = "florian"
    user_name = "Florian Cäsar"
    user_email = "florian@symbolx.com"
    client_name = "pytest"
    client_device_name = "pytest"

    client_in = Client(
        type=ClientType.WEB,
        name=client_name,
        device_name=client_device_name,
        _session=Session(),
    ).to_proto()

    # signup -> success
    signup_req = SignupUserRequest(
        slug=user_slug,
        name=user_name,
        email=user_email,
        client=client_in,
        password="Password123!",
        region=pb2.Region.REGION_ZURICH,
    )
    signup_rep = await destack.signup_user(signup_req)
    assert signup_rep.user.slug == user_slug

    # login, wrong password -> fail
    login_req = LoginUserRequest(slug=user_slug, password="321Password!!!", client=client_in)
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        _ = await destack.login_user(login_req)

    # login, correct password -> success
    login_req = LoginUserRequest(slug=user_slug, password="Password123!", client=client_in)
    login_rep = await destack.login_user(login_req)
    assert login_rep.access_token
    access_metadata = RpcMetadata(
        client_id=login_rep.client.id, client_access_token=login_rep.access_token
    )
    access_headers = pack_rpc_headers(access_metadata)

    # logout, invalid token -> fail
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        bad_access_metadata = access_metadata.__deepcopy__()
        bad_access_metadata.client_access_token = "bad"
        bad_access_headers = pack_rpc_headers(bad_access_metadata)
        _ = await destack.logout_user(LogoutUserRequest(), metadata=bad_access_headers)

    # logout, valid token -> success
    _ = await destack.logout_user(LogoutUserRequest(), metadata=access_headers)
