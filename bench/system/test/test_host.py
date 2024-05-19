from dataclasses import dataclass
from typing import cast

import pytest
from grpclib.testing import ChannelFor

from bench.conftest import detached_session
from bench.language import Bench, ReadOptions, User
from bench.language.bench import Branch, Package
from bench.language.code_ import Code
from bench.language.connection import RemoteEngine
from bench.language.const import (
    BENCH_NODE_TYPES,
    RESOURCE_NODE_TYPES,
    NodeType,
    RunKind,
    RunStatus,
    UserStatus,
)
from bench.language.graph import NodeDataGraph
from bench.language.run import Run
from bench.language.session import Session
from bench.proto import wire, wiring
from bench.proto.wire import (
    CreateBenchRequest,
    GetNodesRequest,
    GraphScope,
    HostStub,
    SupervisorStub,
)
from bench.system.host import HostMultiplexer
from bench.system.resource import decommission_all_resources
from bench.system.test.conftest import UserHandle


@dataclass(slots=True)
class BenchHandle:
    bench: Bench
    branch: Branch
    package: Package
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

    def make_session(self) -> Session:
        return Session(
            parent=self.package,
            _supervisor=self.supervisor,
            _host=self.host,
        )


# nocheckin: share activated bench fixture across module (hangs...)
@pytest.fixture(scope="function")
async def some_bench(supervisor: SupervisorStub, some_user: UserHandle):
    # make bench in supervisor
    create_bench_req = CreateBenchRequest(
        owner=some_user.user.to_ref()._to_data(),
        slug=cast(str, some_user.user.slug),
        is_main=True,
        region=wire.Region.EUROPE_CENTRAL,
    )
    create_bench_rep = await supervisor.create_bench(create_bench_req, metadata=some_user.headers)
    bench_scope = GraphScope(bench_id=create_bench_rep.bench.id)

    # activate bench in host
    host = HostMultiplexer()
    await host.start()
    try:
        async with ChannelFor([host]) as channel:
            host_stub = HostStub(channel)
            remote_engine = RemoteEngine(
                default_scope=bench_scope,
                node_types=BENCH_NODE_TYPES,
                remote=host_stub,
                rpc_metadata=some_user.metadata,
            )

            # get main package
            async with Session(_engines=(remote_engine,)) as session:
                bench = await Bench.descendants(
                    NodeType.BRANCH, NodeType.PACKAGE, *RESOURCE_NODE_TYPES
                ).get(id=bench_scope.bench_id)
                assert bench.main_branch is not None, f"{bench!r} has no main branch"
                main_package = await Package.ancestors(Bench).get(
                    id=bench.main_branch.main_package_id
                )

            handle = BenchHandle(
                bench=bench,
                branch=bench.main_branch,
                package=main_package,
                owner=some_user.user,
                owner_handle=some_user,
                supervisor=supervisor,
                host=host_stub,
            )
            yield handle
    finally:
        host.close()
        await host.wait_closed()

        # decommission
        async with detached_session() as session:
            bench = await Bench.descendants(*RESOURCE_NODE_TYPES).get(id=bench_scope.bench_id)
            await decommission_all_resources(bench, session, commit_per=True)
            await session.commit()


async def test_user_activation(some_bench: BenchHandle):
    """Ensure that the BenchHandle fixtures successfully activates a User."""

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
    read_bench_options = ReadOptions(
        select_all_properties=True,
        descendant_types=[NodeType.ENVIRONMENT, NodeType.BRANCH, NodeType.SERVER, NodeType.STORE],
    )
    read_bench_req = GetNodesRequest(
        roots=[user.main_bench_ptr], scope=some_bench.scope, options=read_bench_options._to_data()
    )
    read_bench_rep = await some_bench.host.get_nodes(read_bench_req, metadata=some_bench.headers)
    data_graph = NodeDataGraph([wiring.unwrap_some_node(n) for n in read_bench_rep.nodes])
    roots, _ = wiring.unpack_node_roots(data_graph)
    bench: Bench = cast(Bench, roots[0])
    assert bench.owner_id == some_bench.owner.id
    assert bench.main_environment
    assert bench.main_environment.store
    assert not bench.main_environment.store.connection_uri  # can't read kernel


async def test_create_run(some_bench: BenchHandle):
    run = Run(
        kind=RunKind.LAMBDA,
        status=RunStatus.SCHEDULED,
        code=Code.from_string("print('hello')"),
    )
    # nocheckin ...