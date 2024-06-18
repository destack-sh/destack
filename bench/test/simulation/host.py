from contextlib import asynccontextmanager
from dataclasses import dataclass, replace
from typing import cast
from uuid import UUID

import pytest
import structlog
from grpclib.testing import ChannelFor

from bench.language import Bench, ReadOptions, Store, User
from bench.language.bench import Branch, Package, ResourceStatus
from bench.language.code import Code
from bench.language.connection import RemoteEngine
from bench.language.const import (
    BENCH_NODE_TYPES,
    IN_BENCH_NODE_TYPES,
    NODE_TYPES,
    PUBLIC_NODE_TYPES,
    ROOT_RESOURCE_NODE_TYPES,
    NodeType,
    RunKind,
    RunStatus,
    UserStatus,
)
from bench.language.expression import NodeReference
from bench.language.graph import NodeDataGraph, NodeSuperGraph
from bench.language.run import Run
from bench.language.session import Session
from bench.proto import wire, wiring
from bench.proto.wire import (
    CreateBenchRequest,
    GetNodesRequest,
    GraphScope,
    HostStub,
    SupervisorBase,
    SupervisorStub,
)
from bench.sql.client import close_pg_connection_pool
from bench.system.core import MockHost, global_session
from bench.system.provisioner import get_provisioners_for
from bench.test.simulation.conftest import UserHandle, make_random_user_handle
from bench.utils.tenacity import RETRY_NEVER

logger = structlog.get_logger(__name__)


@dataclass(slots=True)
class BenchHandle:
    bench: Bench
    branch: Branch
    package: Package
    owner: User
    owner_handle: UserHandle
    # the actual stubs (per function scope) :PytestAsyncWeirdness
    _supervisor: SupervisorStub | None
    _host: HostStub | None

    @property
    def scope(self):
        return GraphScope(bench_id=str(self.bench.id))

    @property
    def headers(self):
        return self.owner_handle.headers

    @property
    def supervisor(self) -> SupervisorStub:
        assert self._supervisor is not None, f"no supervisor for {self!r}"
        return self._supervisor

    @property
    def host(self) -> HostStub:
        assert self._host is not None, f"no host for {self!r}"
        return self._host

    def session(self) -> Session:
        assert self._supervisor is not None, f"no supervisor for {self!r}"
        assert self._host is not None, f"no host for {self!r}"
        engines = (
            # global engine
            RemoteEngine(
                scope=GraphScope(),
                node_types=PUBLIC_NODE_TYPES,
                remote=self._supervisor,
                rpc_metadata=self.owner_handle.metadata,
                retry=RETRY_NEVER,
            ),
            # bench engine
            RemoteEngine(
                scope=self.scope,
                node_types=IN_BENCH_NODE_TYPES,
                remote=self._host,
                rpc_metadata=self.owner_handle.metadata,
                retry=RETRY_NEVER,
            ),
        )
        return Session(
            parent=self.package,
            client=self.owner_handle.client,
            user=self.owner_handle.user,
            _engines=engines,
            _supervisor=self._supervisor,
            _host=self._host,
            _origin=self.owner_handle.origin,
            _subject=self.owner_handle.user,
        )


@asynccontextmanager
async def make_some_bench(supervisor: SupervisorStub, host: HostStub):
    some_user = await make_random_user_handle(supervisor)

    # make bench in supervisor
    create_bench_req = CreateBenchRequest(
        owner=some_user.user._to_ref_data(),
        slug=cast(str, some_user.user.slug),
        is_main=True,
        region=wire.Region.EUROPE_CENTRAL,
    )
    create_bench_rep = await supervisor.create_bench(create_bench_req, metadata=some_user.headers)
    bench_scope = GraphScope(bench_id=create_bench_rep.bench.id)
    bench_id = UUID(create_bench_rep.bench.id)

    # activate bench in host
    remote_engine = RemoteEngine(
        scope=bench_scope,
        node_types=BENCH_NODE_TYPES,
        remote=host,
        rpc_metadata=some_user.metadata,
        retry=RETRY_NEVER,
    )
    async with Session(_default_scope=bench_scope, _engines=(remote_engine,)) as session:
        bench = await Bench.descendants(
            NodeType.BRANCH, NodeType.PACKAGE, *ROOT_RESOURCE_NODE_TYPES
        ).get(id=bench_id)
        assert bench.main_branch is not None, f"{bench!r} has no main branch"
        main_package = await Package.ancestors(Bench).get(id=bench.main_branch.main_package_id)

    handle = BenchHandle(
        bench=bench,
        branch=bench.main_branch,
        package=main_package,
        owner=some_user.user,
        owner_handle=some_user,
        _supervisor=None,
        _host=None,
    )
    yield handle

    # decommission
    async with global_session() as session:
        provisioners = get_provisioners_for(MockHost(session), bench)
        bench = await Bench.descendants(*ROOT_RESOURCE_NODE_TYPES).get(id=bench_id)
        for resource in bench.resources:
            # ensure store postgres connections are closed
            if isinstance(resource, Store):
                await close_pg_connection_pool(resource)
            # then decommission
            for provisioner in provisioners:
                if resource.metatype in provisioner.provision_types:
                    await provisioner.decommission(resource)
                    break
            else:
                raise RuntimeError(f"no provisioner for {resource!r} in {provisioners!r}")
        logger.debug("test_host.decommissioned", bench=bench, resources=list(bench.resources))
        await session.commit()


@pytest.fixture(scope="module")
async def some_bench_setup(supervisor_service: SupervisorBase, host_service):
    async with ChannelFor([supervisor_service, host_service]) as channel:
        supervisor = SupervisorStub(channel)
        host = HostStub(channel)
        async with make_some_bench(supervisor, host) as handle:
            yield handle


@pytest.fixture()
def some_bench(supervisor, host, some_bench_setup: BenchHandle):
    handle = replace(some_bench_setup, _supervisor=supervisor, _host=host)
    return handle


async def test_activate_user(some_bench: BenchHandle):
    """Ensure that the BenchHandle fixtures successfully activates a User."""

    # get user to check they're activated with a main Bench
    read_user_req = GetNodesRequest(
        roots=[some_bench.owner._to_ref_data()],
        options=ReadOptions(select_all_properties=True)._to_data(),
    )
    read_user_rep = await some_bench.supervisor.get_nodes(
        read_user_req, metadata=some_bench.headers
    )
    user = read_user_rep.nodes[0].user
    assert user.main_bench_ptr is not None, f"{user!r} has no main Bench"
    assert user.status == UserStatus.ACTIVATED

    # read back bench (should be allowed & have default resources setup in healthy state)
    read_bench_options = ReadOptions(
        select_all_properties=True,
        descendant_types=[NodeType.ENVIRONMENT, NodeType.BRANCH, NodeType.SERVER, NodeType.STORE],
    )
    read_bench_req = GetNodesRequest(
        roots=[user.main_bench_ptr], scope=some_bench.scope, options=read_bench_options._to_data()
    )
    read_bench_rep = await some_bench.host.get_nodes(read_bench_req, metadata=some_bench.headers)
    data_graph = NodeDataGraph(
        scope=GraphScope(),
        node_types=NODE_TYPES.tuple,
        nodes=[wiring.unwrap_some_node(n) for n in read_bench_rep.nodes],
    )
    supergraph = NodeSuperGraph(
        root_ptr=wiring.unpack_object(user.main_bench_ptr, supergraph=None, expect=NodeReference)
    )
    roots, _ = wiring.unpack_node_roots(data_graph, supergraph)
    bench: Bench = cast(Bench, roots[0])
    assert UUID(user.main_bench_ptr.id) == bench.id
    assert bench.owner_id == some_bench.owner.id
    env = bench.main_environment
    assert env, f"{bench!r} has no main environment"
    assert env.store and env.store.status == ResourceStatus.HEALTHY
    assert env.server and env.server.status == ResourceStatus.HEALTHY


async def test_create_run(some_bench: BenchHandle):
    # TODO :Test: test runs properly
    async with some_bench.session() as session:
        run = Run(
            parent=some_bench.package,
            kind=RunKind.LAMBDA,
            status=RunStatus.SCHEDULED,
            code=Code.from_string("print('hello')"),
        )
        session._create(run)
        await session.commit()

        run = await Run.get(id=run.id)
        run = await Run.include_ancestors().get(id=run.id)
        ...
