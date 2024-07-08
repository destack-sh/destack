from dataclasses import dataclass
from uuid import UUID

import pytest

from bench.language import NodeReference, Store
from bench.proto import wire
from bench.proto.wire import (
    ClientDataIn,
    CreateBenchRequest,
    HostClient,
    NodeReferenceData,
    RpcMetadata,
    SignupUserRequest,
    SupervisorClient,
)
from bench.proto.wiring import pack_rpc_headers
from bench.sql.engine import GLOBAL_SCHEMA
from bench.system.host import Host
from bench.system.supervisor import Supervisor
from bench.test.fixtures import create_test_db, make_system_store
from bench.test.simulation.transport import SimulatedChannel
from bench.utils.oracle import REAL_ORACLE

# NOTE: The runtime tests are basically smaller, more focused simulation tests,
#  so some of the setup logic is similar but I didn't want to introduce cross-dependencies.


@pytest.fixture()
async def global_store(request: pytest.FixtureRequest):
    global_store = make_system_store(f"test-{request.node.name}")
    await create_test_db(global_store, GLOBAL_SCHEMA)
    return global_store


@pytest.fixture()
async def supervisor_service(global_store: Store):
    service = Supervisor(global_store, REAL_ORACLE)
    await service.start()
    yield service
    service.close()
    await service.wait_closed()


@pytest.fixture()
async def supervisor_client(supervisor_service: Supervisor):
    async with SimulatedChannel(services=(supervisor_service,), oracle=REAL_ORACLE) as channel:
        supervisor_client = SupervisorClient(channel)
        yield supervisor_client


@dataclass
class ClientHandle:
    user_ptr: NodeReferenceData
    user_data: wire.UserData
    client_data: wire.ClientData
    rpc_metadata: RpcMetadata
    rpc_headers: dict[str, str]


@pytest.fixture()
async def client(supervisor_client: SupervisorClient, request: pytest.FixtureRequest):
    # make per-test client to create bench
    name = request.node.name
    client_in = ClientDataIn(type=wire.ClientType.BENCH_SERVER, name=name, device_name="test")
    signup_req = SignupUserRequest(
        slug=name,
        name=name,
        email=f"{name}@test.com",
        password=name,
        client=client_in,
    )
    signup_rep = await supervisor_client.signup_user(signup_req)
    user_ptr = NodeReference.from_node_data(signup_rep.user)
    client_data = signup_rep.client
    rpc_metadata = RpcMetadata(
        client_type=client_data.type,
        client_id=client_data.id,
        client_nonce=client_data.id,
        client_access_token=signup_rep.access_token,
    )
    rpc_headers = pack_rpc_headers(rpc_metadata)
    return ClientHandle(
        user_ptr=user_ptr,
        user_data=signup_rep.user,
        client_data=client_data,
        rpc_metadata=rpc_metadata,
        rpc_headers=rpc_headers,
    )


@pytest.fixture()
async def host_service(
    global_store: Store, client: ClientHandle, supervisor_client: SupervisorClient
):
    # create bench in supervisor
    create_bench_req = CreateBenchRequest(
        owner=client.user_ptr,
        is_main=True,
        slug=client.user_data.name,
        region=wire.Region.EUROPE_CENTRAL,
    )
    create_bench_rep = await supervisor_client.create_bench(
        create_bench_req, metadata=client.rpc_headers
    )
    bench_id = UUID(create_bench_rep.bench.id)

    # start host service
    service = Host(bench_id=bench_id, global_store=global_store, oracle=REAL_ORACLE)
    await service.start()
    yield service
    service.close()
    await service.wait_closed()


@pytest.fixture()
async def host_client(host_service: Host):
    async with SimulatedChannel(services=(host_service,), oracle=REAL_ORACLE) as channel:
        yield HostClient(channel=channel)
