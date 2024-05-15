from contextlib import asynccontextmanager
from urllib.parse import urlparse
from uuid import UUID

import structlog
from grpclib.client import Channel

from bench.language import Bench, NodeType, Package, Session
from bench.language.connection import RemoteEngine
from bench.language.const import BENCH_NODE_TYPES
from bench.proto.services import MonitoredServiceBase
from bench.proto.wire import (
    BenchData,
    GraphScope,
    HostStub,
    PackageData,
    RuntimeBase,
    SupervisorStub,
)
from bench.runtime.connection import ConnectedQuery
from bench.utils.tenacity import RetryOptions

logger = structlog.get_logger(__name__)

LOADED_BENCH_NODE_TYPES: tuple[NodeType, ...] = (
    NodeType.BENCH,
    NodeType.ENVIRONMENT,
    NodeType.BRANCH,
    NodeType.PACKAGE,
)
LOADED_PACKAGE_NODE_TYPES: tuple[NodeType, ...] = (
    NodeType.DEPENDENCY,
    NodeType.UPGRADE,
    NodeType.SPACE,
    NodeType.LINK,
    NodeType.NOTICE,
    NodeType.BLOCK,
    NodeType.TRIGGER,
    NodeType.FIELD,
    NodeType.QUERY,
    NodeType.STEP,
    NodeType.VIEW,
)
BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).include_all()
PACKAGE_QUERY = Package.descendants(*LOADED_PACKAGE_NODE_TYPES).ancestors(Bench).include_all()

ConnectedBench = ConnectedQuery[Bench, BenchData]
ConnectedPackage = ConnectedQuery[Package, PackageData]

REMOTE_CONNECTION_RETRY = RetryOptions(max_attempts=-1)


class Runtime(RuntimeBase, MonitoredServiceBase):
    """
    A Runtime processes selected Runs for a Bench/Package in Sessions.
    A Runtime process is started for each active Package in a Bench.
    """

    def __init__(
        self,
        *,
        supervisor_url: str,
        bench_id: UUID,
        client_id: UUID | None = None,
        user_id: UUID | None = None,
        server_id: UUID | None = None,
    ):
        super().__init__()
        # context
        self._client_id = client_id
        self._server_id = server_id
        self._user_id = user_id
        self._bench_id = bench_id

        # parse out supervisor host and port
        _supervisor_url = urlparse(supervisor_url)
        self._supervisor_host = _supervisor_url.hostname
        self._supervisor_port = _supervisor_url.port
        if self._supervisor_host is None or self._supervisor_port is None:
            raise ValueError(f"invalid supervisor URL: {supervisor_url}")
        self._supervisor = SupervisorStub(Channel(self._supervisor_host, self._supervisor_port))

        # bench stuff
        self._host: HostStub | None = None
        self._bench: ConnectedBench | None = None
        self._main_package: ConnectedPackage | None = None
        self._packages: dict[UUID, ConnectedPackage] = {}

    def __str__(self):
        return f"{self._bench_id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def host(self) -> HostStub:
        assert self._host is not None, f"no host for {self!r}"
        return self._host

    async def connect_package(self, package_id: UUID) -> ConnectedPackage:
        assert self._host is not None, f"no host for {self!r}"
        scope = GraphScope(bench_id=str(self._bench_id), package_id=str(package_id))
        package = await ConnectedQuery(
            query=PACKAGE_QUERY.where(id=package_id),
            remote=self._host,
            scope=scope,
        ).connect()
        self._packages[package_id] = package
        return package

    async def start(self):
        self._host = await get_host_client(self._bench_id, self._supervisor)
        async with local_session(self._supervisor, self._bench_id, self._host):
            # connect bench & main packages
            self._bench = await ConnectedQuery(
                query=BENCH_QUERY.where(id=self._bench_id),
                remote=self._host,
                scope=GraphScope(bench_id=str(self._bench_id)),
            ).connect()
            main_branch = self._bench.node.main_branch
            assert main_branch is not None, f"{self._bench!r} has no main branch"
            assert main_branch.package_id is not None, f"{main_branch!r} has no main package"
            self._main_package = await self.connect_package(main_branch.package_id)

    def close(self):
        if self._bench is not None:
            self._bench.close()
        for package in self._packages.values():
            package.close()

    async def wait_closed(self):
        if self._bench is not None:
            await self._bench.wait_closed()
        for package in self._packages.values():
            await package.wait_closed()
        self._bench = None
        self._main_package = None
        self._packages.clear()


@asynccontextmanager
async def local_session(supervisor: SupervisorStub, bench_id: UUID, host: HostStub):
    bench_scope = GraphScope(bench_id=str(bench_id))
    engines = (
        RemoteEngine(
            default_scope=bench_scope,
            node_types=BENCH_NODE_TYPES,
            remote=host,
            retry=REMOTE_CONNECTION_RETRY,
        ),
        RemoteEngine(
            default_scope=bench_scope,
            node_types=LOADED_PACKAGE_NODE_TYPES,
            remote=host,
            retry=REMOTE_CONNECTION_RETRY,
        ),
    )
    async with Session(_supervisor=supervisor, _host=host, _engines=engines) as session:
        yield session


async def get_host_client(bench_id: UUID, supervisor: SupervisorStub) -> HostStub:
    # TODO :Scalability: lookup bench host (via supervisor?) :SingleHostService
    return HostStub(supervisor.channel)
