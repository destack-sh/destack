from dataclasses import dataclass

import pytest
from grpclib.testing import ChannelFor

from bench.language import Bench, ReadOptions, User
from bench.language.const import NodeType, UserStatus
from bench.language.graph import NodeDataGraph
from bench.proto import wire, wiring
from bench.proto.wire import (
    CreateBenchRequest,
    GetNodesRequest,
    GraphScope,
    HostStub,
    SupervisorStub,
)
from bench.system.host import HostMultiplexer
from bench.system.test.conftest import UserHandle


@dataclass
class BenchHandle:
    bench: Bench
    owner: User
    owner_handle: UserHandle
    supervisor: SupervisorStub
    host: HostStub

    @property
    def scope(self):
        return GraphScope(bench_id=str(self.bench.id))

    @property
    def headers(self):
        return self.owner_handle.headers


@pytest.fixture(scope="function")
async def some_bench(supervisor: SupervisorStub, some_user: UserHandle) -> BenchHandle:
    create_bench_req = CreateBenchRequest(
        owner=some_user.user.to_ref()._to_data(),
        slug=some_user.user.slug,
        is_main=True,
        region=wire.Region.EUROPE_CENTRAL,
    )
    create_bench_rep = await supervisor.create_bench(create_bench_req, metadata=some_user.headers)
    bench: Bench = wiring.unpack_node(create_bench_rep.bench)

    service = HostMultiplexer()
    await service.start()
    try:
        async with ChannelFor([service]) as channel:
            host_stub = HostStub(channel)
            handle = BenchHandle(
                bench=bench,
                owner=some_user.user,
                owner_handle=some_user,
                supervisor=supervisor,
                host=host_stub,
            )
            yield handle
    finally:
        service.close()
        await service.wait_closed()


async def test_user_activate(some_bench: BenchHandle):
    """Activate a User by creating their main Bench and provisioning it."""

    # get user to check they're activated with a main Bench
    read_user_req = GetNodesRequest(
        roots=[some_bench.owner.to_ref()._to_data()],
        options=ReadOptions(select_all_properties=True)._to_data(),
    )
    read_user_rep = await some_bench.supervisor.get_nodes(
        read_user_req, metadata=some_bench.headers
    )
    user = read_user_rep.nodes[0].user
    assert user.main_bench_ptr is not None, f"{user!r} has no main Bench"
    assert user.status == UserStatus.ACTIVATED

    # read back bench (should be allowed & have default resources)
    read_bench_req = GetNodesRequest(
        roots=[user.main_bench_ptr],
        scope=some_bench.scope,
        options=ReadOptions(
            select_all_properties=True,
            descendant_types=[
                NodeType.ENVIRONMENT,
                NodeType.BRANCH,
                NodeType.SERVER,
                NodeType.STORE,
            ],
        )._to_data(),
    )
    read_bench_rep = await some_bench.host.get_nodes(read_bench_req, metadata=some_bench.headers)
    node_graph = NodeDataGraph([wiring.unwrap_some_node(n) for n in read_bench_rep.nodes])
    bench: Bench = wiring.unpack_nodes_graph(node_graph)[0]
    assert bench.owner_id == some_bench.owner.id
    assert bench.main_environment.store
    assert bench.main_environment.search
    assert not bench.main_environment.store.main_credential  # can't read kernel
