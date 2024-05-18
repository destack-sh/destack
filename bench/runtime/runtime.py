import asyncio
from contextlib import asynccontextmanager
from urllib.parse import urlparse
from uuid import UUID

import structlog
from grpclib.client import Channel

from bench.language import Bench, NodeType, Package, Session
from bench.language.access import Subject
from bench.language.connection import RemoteEngine
from bench.language.const import BENCH_NODE_TYPES, IN_PACKAGE_NODE_TYPES, PUBLIC_NODE_TYPES
from bench.language.resource import Client
from bench.proto.services import MonitoredServiceBase
from bench.proto.wire import (
    BenchData,
    GraphScope,
    HostStub,
    PackageData,
    RpcMetadata,
    RuntimeBase,
    StartRunRequest,
    StartRunResponse,
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
BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).select_all()
PACKAGE_QUERY = (
    Package.descendants(*LOADED_PACKAGE_NODE_TYPES)
    .ancestors(Bench)
    .select_all()
    .exclude(Bench.encryption_key)
)

ConnectedBench = ConnectedQuery[Bench, BenchData]
ConnectedPackage = ConnectedQuery[Package, PackageData]

REMOTE_CONNECTION_RETRY = RetryOptions(max_attempts=-1)


class Runtime(RuntimeBase, MonitoredServiceBase):
    """
    A Runtime processes selected Runs in a Bench/Package in Sessions on a Client.
    """

    def __init__(
        self,
        *,
        supervisor_url: str,
        bench_id: UUID,
        client_id: UUID,
        client_access_token: str,
    ):
        super().__init__()

        # parse out supervisor host and port
        _supervisor_url = urlparse(supervisor_url)
        self._supervisor_host = _supervisor_url.hostname
        self._supervisor_port = _supervisor_url.port
        if self._supervisor_host is None or self._supervisor_port is None:
            raise ValueError(f"invalid supervisor URL: {supervisor_url}")
        self._supervisor = SupervisorStub(Channel(self._supervisor_host, self._supervisor_port))
        self._engines: tuple[RemoteEngine, ...] = ()

        # context
        self._client_id = client_id
        self._client_access_token = client_access_token
        self._rpc_metadata = RpcMetadata(
            client_id=str(self._client_id),
            client_access_token=self._client_access_token,
        )
        self._client: Client | None = None
        self._bench_id = bench_id

        # bench stuff
        self._host: HostStub | None = None
        self._bench: ConnectedBench | None = None
        self._main_package: ConnectedPackage | None = None
        self._packages: dict[UUID, ConnectedPackage] = {}

    def __str__(self):
        bench_str = repr(self._bench._node) if self._bench and self._bench._node else self._bench_id
        client_str = repr(self._client) if self._client else self._client_id
        return f"{client_str} on {bench_str}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def host(self) -> HostStub:
        assert self._host is not None, f"no host for {self!r}"
        return self._host

    async def connect_package(self, package_id: UUID) -> ConnectedPackage:
        """'Connect's a Package to get it live."""
        assert self._host is not None, f"no host for {self!r}"
        scope = GraphScope(bench_id=str(self._bench_id), package_id=str(package_id))
        package = await ConnectedQuery(
            query=PACKAGE_QUERY.where(id=package_id),
            remote=self._host,
            scope=scope,
            rpc_metadata=self._rpc_metadata,
        ).connect()
        self._packages[package_id] = package
        return package

    def disconnect_package(self, package_id: UUID):
        package = self._packages.pop(package_id)
        package.close()

    async def start(self):
        # set up host
        self._host = await get_host_client(self._bench_id, self._supervisor)
        bench_scope = GraphScope(bench_id=str(self._bench_id))
        self._engines = (
            # global engine
            RemoteEngine(
                default_scope=GraphScope(),
                node_types=PUBLIC_NODE_TYPES,
                remote=self._supervisor,
                retry=REMOTE_CONNECTION_RETRY,
                rpc_metadata=self._rpc_metadata,
            ),
            # bench engine
            RemoteEngine(
                default_scope=bench_scope,
                node_types=BENCH_NODE_TYPES,
                remote=self._host,
                retry=REMOTE_CONNECTION_RETRY,
                rpc_metadata=self._rpc_metadata,
            ),
            # in-package engine
            RemoteEngine(
                default_scope=bench_scope,
                node_types=IN_PACKAGE_NODE_TYPES,
                remote=self._host,
                retry=REMOTE_CONNECTION_RETRY,
                rpc_metadata=self._rpc_metadata,
            ),
        )

        # connect
        start = asyncio.get_event_loop().time()
        async with local_session(self._engines, self._supervisor, self._host) as session:
            # get details on this client (and check that it's valid)
            self._client = await Client.get(id=self._client_id)
            session.untrack(self._client)

            # connect bench & main packages
            self._bench = await ConnectedQuery(
                query=BENCH_QUERY.where(id=self._bench_id),
                remote=self._host,
                scope=GraphScope(bench_id=str(self._bench_id)),
                rpc_metadata=self._rpc_metadata,
            ).connect()
            main_branch = self._bench.node.main_branch
            assert main_branch is not None, f"{self._bench!r} has no main branch"
            assert main_branch.main_package_id is not None, f"{main_branch!r} has no main package"
            self._main_package = await self.connect_package(main_branch.main_package_id)
            session.untrack_many(self._bench.node, self._main_package.node)
        logger.info(
            "runtime.connected",
            bench=self._bench.node,
            client=self._client,
            duration=asyncio.get_event_loop().time() - start,
        )

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

    async def start_run(self, subject: Subject, request: StartRunRequest) -> StartRunResponse:
        raise NotImplementedError("nocheckin: start_run")


@asynccontextmanager
async def local_session(
    engines: tuple[RemoteEngine, ...], supervisor: SupervisorStub, host: HostStub
):
    async with Session(_supervisor=supervisor, _host=host, _engines=engines) as session:
        yield session


async def get_host_client(bench_id: UUID, supervisor: SupervisorStub) -> HostStub:
    # TODO :Scalability: lookup bench host (via supervisor?) :SingleHostService
    return HostStub(supervisor.channel)
