import asyncio
from contextlib import asynccontextmanager
from typing import cast, override
from urllib.parse import urlparse
from uuid import UUID

import structlog
from grpclib.client import Channel

from bench.language import Bench, NodeType, Package, Session
from bench.language.access import Subject
from bench.language.bench import Client, Machine
from bench.language.connection import RemoteEngine
from bench.language.const import BENCH_NODE_TYPES, IN_PACKAGE_NODE_TYPES, PUBLIC_NODE_TYPES
from bench.language.run import Run
from bench.proto import wiring
from bench.proto.services import BenchServiceBase
from bench.proto.wire import (
    BenchData,
    GraphScope,
    HostStub,
    PackageData,
    QueueRunRequest,
    QueueRunResponse,
    RpcMetadata,
    RuntimeBase,
    SupervisorStub,
)
from bench.runtime.connection import ConnectedQuery
from bench.runtime.process import RuntimeProcess
from bench.utils.dt import monotime
from bench.utils.tenacity import RetryOptions

logger = structlog.get_logger(__name__)

LOADED_BENCH_NODE_TYPES: tuple[NodeType, ...] = (
    NodeType.BENCH,
    NodeType.ENVIRONMENT,
    NodeType.BRANCH,
    NodeType.MACHINE,
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
RUNTIME_PARALLELISM = 1


class Runtime(RuntimeBase, BenchServiceBase):
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
        machine_id: UUID | None,
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
        self._machine_id = machine_id
        self._machine: Machine | None = None

        # bench stuff
        self._host: HostStub | None = None
        self._bench: ConnectedBench | None = None
        self._main_package: ConnectedPackage | None = None
        self._packages: dict[UUID, ConnectedPackage] = {}
        self._packages_lock = asyncio.Lock()

        # processes
        self._run_queue: asyncio.Queue[Run] = asyncio.Queue()
        self._processes: list[RuntimeProcess] = []

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

    @property
    def bench(self) -> Bench:
        assert self._bench is not None, f"no bench for {self!r}"
        return self._bench.node

    @property
    def client(self) -> Client:
        assert self._client is not None, f"no client for {self!r}"
        return self._client

    @property
    def machine(self) -> Machine:
        assert self._machine is not None, f"no machine for {self!r}"
        return self._machine

    @property
    def main_package(self) -> Package:
        assert self._main_package is not None, f"no main package for {self!r}"
        return self._main_package.node

    async def get_package(self, package_id: UUID) -> ConnectedPackage:
        """Connect's a Package."""
        assert self._host is not None, f"no host for {self!r}"
        package = self._packages.get(package_id)
        if package is None:
            async with self._packages_lock:
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
                scope=GraphScope(),
                node_types=PUBLIC_NODE_TYPES,
                remote=self._supervisor,
                retry=REMOTE_CONNECTION_RETRY,
                rpc_metadata=self._rpc_metadata,
            ),
            # bench engine
            RemoteEngine(
                scope=bench_scope,
                node_types=BENCH_NODE_TYPES,
                remote=self._host,
                retry=REMOTE_CONNECTION_RETRY,
                rpc_metadata=self._rpc_metadata,
            ),
            # in-package engine
            RemoteEngine(
                scope=bench_scope,
                node_types=IN_PACKAGE_NODE_TYPES,
                remote=self._host,
                retry=REMOTE_CONNECTION_RETRY,
                rpc_metadata=self._rpc_metadata,
            ),
        )

        # connect
        start = monotime()
        async with local_session(self._engines, self._supervisor, self._host) as session:
            # connect bench
            self._bench = await ConnectedQuery(
                query=BENCH_QUERY.where(id=self._bench_id),
                remote=self._host,
                scope=GraphScope(bench_id=str(self._bench_id)),
                rpc_metadata=self._rpc_metadata,
            ).connect()
            main_environment = self._bench.node.main_environment
            assert main_environment is not None, f"{self._bench!r} has no main environment"
            main_branch = self._bench.node.main_branch
            assert main_branch is not None, f"{self._bench!r} has no main branch"
            assert main_branch.main_package_id is not None, f"{main_branch!r} has no main package"

            # get client in Bench (and check that it's valid & belongs there)
            self._client = main_environment.server.clients.get(self._client_id)
            assert self._client is not None, f"{main_environment!r} has no client {self._client_id}"
            # and machine (if specified)
            if self._machine_id is not None:
                self._machine = main_environment.server.machines.get(self._machine_id)
                assert (
                    self._machine is not None
                ), f"{main_environment.server!r} has no machine {self._machine_id}"

            # connect main package
            self._main_package = await self.get_package(main_branch.main_package_id)

            session.untrack_many(self._bench.node, self._client, self._main_package.node)

        # start processes
        for i in range(RUNTIME_PARALLELISM):
            process = RuntimeProcess(
                i, self, self._bench.node, self._main_package.node, self._run_queue
            )
            self._processes.append(process)
            await process.start()

        logger.info(
            "runtime.start",
            bench=self._bench.node,
            client=self._client,
            duration=monotime() - start,
        )

    def close(self):
        super().close()
        if self._bench is not None:
            self._bench.close()
        for package in self._packages.values():
            package.close()

    async def wait_closed(self):
        await super().wait_closed()
        if self._bench is not None:
            await self._bench.wait_closed()
        for package in self._packages.values():
            await package.wait_closed()
        self._bench = None
        self._main_package = None
        self._packages.clear()

    @override
    async def queue_run(self, subject: Subject, request: QueueRunRequest) -> QueueRunResponse:
        assert (
            request.run.parent_ptr and request.run.parent_ptr.id == self.main_package.id
        ), f"{request.run!r} not in {self.main_package!r}"
        run = cast(Run, wiring.unpack_node(request.run, parent=self.main_package))
        self._run_queue.put_nowait(run)
        logger.trace("runtime.queue_run", run=run, subject=subject)
        return QueueRunResponse()


@asynccontextmanager
async def local_session(
    engines: tuple[RemoteEngine, ...], supervisor: SupervisorStub, host: HostStub
):
    async with Session(_supervisor=supervisor, _host=host, _engines=engines) as session:
        yield session


async def get_host_client(bench_id: UUID, supervisor: SupervisorStub) -> HostStub:
    # NOTE :Scalability: lookup bench host (via supervisor?) :SingleHostService
    return HostStub(supervisor.channel)
