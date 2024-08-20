from dataclasses import replace
from typing import cast
from uuid import uuid4

import pytest
from grpclib import Status as GRPCStatus

from bench.language import NodeReference, ReadOptions, User
from bench.language.const import (
    NodeType,
)
from bench.language.node import GraphScope
from bench.language.property import Property
from bench.proto import wire
from bench.proto.wire import (
    ClientDataIn,
    GetNodesRequest,
    LoginUserRequest,
    LogoutUserRequest,
    RpcMetadata,
    SignupUserRequest,
    SupervisorClient,
    UserData,
)
from bench.proto.wiring import pack_rpc_headers
from bench.system.supervisor.service import SupervisorService
from bench.system.utils.sharding import HostMap
from bench.test.fixtures import raises_grpc_error
from bench.test.simulation.transport import SimulatedChannel
from bench.utils.oracle import REAL_ORACLE

#
# Simulated but unit-test-like supervisor-only tests
#


@pytest.fixture()
async def supervisor_service(global_store):
    supervisor_service = SupervisorService(global_store, REAL_ORACLE, HostMap({}))
    await supervisor_service.start()
    yield supervisor_service
    supervisor_service.close()
    await supervisor_service.wait_closed()


@pytest.fixture()
async def supervisor(supervisor_service):
    async with SimulatedChannel(services=(supervisor_service,), oracle=REAL_ORACLE) as channel:
        yield SupervisorClient(channel=channel)


async def test_user_registration(supervisor: SupervisorClient):
    """Create a User, login and logout. Try some wrong passwords and tokens. Read back data to confirm."""

    user_slug = "test"
    user_name = "Test"
    user_email = "test@symbolx.com"
    client_name = "pytest"
    client_device_name = "pytest"

    user_in = UserData(slug=user_slug, name=user_name, email=user_email)
    client_in = ClientDataIn(
        id=str(uuid4()),
        type=wire.ClientType.BENCH_WEB,
        name=client_name,
        device_name=client_device_name,
    )

    # signup -> success
    signup_req = SignupUserRequest(
        slug=cast(str, user_in.slug),
        name=user_in.name,
        email=cast(str, user_in.email),
        client=cast(ClientDataIn, client_in),
        password="Password123!",
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

    # read user with sensitive data, authorized -> success
    options = ReadOptions(
        include_properties=[cast(Property, User.email)],
        descendant_types=[NodeType.CLIENT, NodeType.HANDLE],
    )._to_data()
    read_user_req = GetNodesRequest(
        scope=GraphScope()._to_data(),
        roots=[NodeReference._ref_data_from_node_data(signup_rep.user)],
        options=options,
    )
    access_metadata = RpcMetadata(
        client_id=login_rep.client.id, client_access_token=login_rep.access_token
    )
    access_headers = pack_rpc_headers(access_metadata)
    read_user_rep = await supervisor.get_nodes(read_user_req, metadata=access_headers)
    assert len(read_user_rep.nodes) == 3
    assert read_user_rep.nodes[0].user.email == user_email
    assert read_user_rep.nodes[0].user.main_handle_ptr
    assert read_user_rep.nodes[0].user.main_handle_ptr.id == read_user_rep.nodes[2].handle.id
    assert read_user_rep.nodes[1].client.device_name == client_device_name
    assert read_user_rep.nodes[2].handle.slug == user_slug

    # logout, invalid token -> fail
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        bad_access_metadata = replace(access_metadata, client_access_token="bad")
        bad_access_headers = pack_rpc_headers(bad_access_metadata)
        _ = await supervisor.logout_user(LogoutUserRequest(), metadata=bad_access_headers)

    # logout, valid token -> success
    _ = await supervisor.logout_user(LogoutUserRequest(), metadata=access_headers)

    # read user, logged out, expired token -> fail
    with raises_grpc_error(GRPCStatus.UNAUTHENTICATED):
        _ = await supervisor.get_nodes(read_user_req, metadata=access_headers)
