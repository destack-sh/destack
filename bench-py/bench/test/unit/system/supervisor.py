import pytest
from bench.language import (
    Client,
    ClientType,
    DatabaseInfo,
    Session,
    StaticCellRegistry,
    StaticDatabaseRegistry,
)
from bench.proto import (
    LoginUserRequest,
    LogoutUserRequest,
    NullNetwork,
    RpcMetadata,
    SignupUserRequest,
    SupervisorClient,
    pack_rpc_headers,
)
from bench.test.fixtures import raises_grpc_error
from bench.test.simulation.core import SimulatedChannel
from bench.utils.oracle import REAL_ORACLE
from grpclib import Status as GRPCStatus

from bench import pb2

#
# Simulated but unit-test-like supervisor-only tests
#


@pytest.fixture
async def supervisor_service(global_database: DatabaseInfo, main_database: DatabaseInfo):
    from bench.system import SupervisorService

    supervisor_service = SupervisorService(
        id="supervisor",
        global_database=global_database,
        network=NullNetwork(),
        oracle=REAL_ORACLE,
        cell_registry=StaticCellRegistry(()),
        database_registry=StaticDatabaseRegistry(()),
    )
    await supervisor_service.start()
    yield supervisor_service
    supervisor_service.stop()
    await supervisor_service.wait_stopped()


@pytest.fixture
async def supervisor(supervisor_service):
    async with SimulatedChannel(services=(supervisor_service,), oracle=REAL_ORACLE) as channel:
        yield SupervisorClient(channel=channel)


async def test_user_registration(supervisor: SupervisorClient):
    """Create a User, login and logout. Try some wrong passwords and tokens. Read back data to confirm."""

    session = Session()

    user_slug = "florian"
    user_name = "Florian Cäsar"
    user_email = "florian@symbolx.com"
    client_name = "pytest"
    client_device_name = "pytest"

    client_in = Client(
        type=ClientType.WEB,
        name=client_name,
        device_name=client_device_name,
        _session=session,
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
    signup_rep = await supervisor.signup_user(signup_req)
    assert signup_rep.user.slug == user_slug

    # login, invalid password -> fail
    login_req = LoginUserRequest(slug=user_slug, password="bad", client=client_in)
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        _ = await supervisor.login_user(login_req)

    # login, wrong password -> fail
    login_req = LoginUserRequest(slug=user_slug, password="321Password!!!", client=client_in)
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        _ = await supervisor.login_user(login_req)

    # login, correct password -> success
    login_req = LoginUserRequest(slug=user_slug, password="Password123!", client=client_in)
    login_rep = await supervisor.login_user(login_req)
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
        _ = await supervisor.logout_user(LogoutUserRequest(), metadata=bad_access_headers)

    # logout, valid token -> success
    _ = await supervisor.logout_user(LogoutUserRequest(), metadata=access_headers)
