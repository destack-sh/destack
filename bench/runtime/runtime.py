import asyncio
from contextlib import asynccontextmanager
from typing import override
from urllib.parse import urlparse
from uuid import UUID

import structlog
from grpclib.client import Channel

from bench.language import Bench, Package
from bench.language.bench import Client, Machine
from bench.language.connection import RemoteEngine
from bench.language.const import BENCH_NODE_TYPES, IN_PACKAGE_NODE_TYPES, PUBLIC_NODE_TYPES
from bench.language.session import Session
from bench.proto.services import BenchServiceBase
from bench.proto.wire import (
    GraphScope,
    HostStub,
    QueueRunRequest,
    QueueRunResponse,
    RpcMetadata,
    RunData,
    RuntimeBase,
    ServiceKind,
    SupervisorStub,
)
from bench.runtime.connection import (
    ConnectedBench,
    ConnectedPackage,
    QueryConnector,
    RemoteConnector,
)
from bench.runtime.core import (
    BENCH_QUERY,
    PACKAGE_QUERY,
    REMOTE_CONNECTION_RETRY,
    RUNTIME_PARALLELISM,
)
from bench.runtime.thread import RuntimeThread
from bench.utils.dt import monotime
from bench.utils.func import CriticalLock
from bench.utils.uuidt import UUIDT

logger = structlog.get_logger(__name__)


class Runtime(RuntimeBase, BenchServiceBase):
    """
    A Runtime processes selected Runs in a Bench/Package in Sessions on a Client.
    """

    kind = ServiceKind.INTERNAL  # :ServiceKind

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

        # context
        self._client_id = client_id
        self._client_access_token = client_access_token
        self._rpc_metadata = RpcMetadata(
            client_id=str(self._client_id),
            client_access_token=self._client_access_token,
        )
        self._client: Client | None = None
        self._machine_id = machine_id
        self._machine: Machine | None = None
        self._connector: QueryConnector | None = None
        self._engines: tuple[RemoteEngine, ...] = ()

        # bench stuff
        self._host: HostStub | None = None
        self._bench_id = bench_id
        self._bench: ConnectedBench | None = None
        self._main_package: ConnectedPackage | None = None
        self._packages: dict[UUID, ConnectedPackage] = {}
        self._packages_lock = asyncio.Lock()

        # processing
        self._session: Session | None = None
        self._tx_lock: asyncio.Lock = CriticalLock(
            name=f"{self.__class__.__name__}_{bench_id or ''}"
        )
        self._run_queue: asyncio.Queue[RunData] = asyncio.Queue()
        self._threads: list[RuntimeThread] = []

    def __str__(self):
        bench_str = (
            repr(self._bench.node) if self._bench and self._bench.has_result else self._bench_id
        )
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
    def main_package(self) -> Package:
        assert self._main_package is not None, f"no main package for {self!r}"
        return self._main_package.node

    async def get_package(self, package_id: UUID) -> ConnectedPackage:
        """Connects a Package."""
        assert self._session is not None, f"no session for {self!r}"
        assert self._connector is not None, f"no connector for {self!r}"
        package = self._packages.get(package_id)
        if package is None:
            async with self._packages_lock:
                package = await self._connector.connect(
                    query=PACKAGE_QUERY.where(id=package_id),
                    tx_lock=self._tx_lock,
                    session=self._session,
                )
                self._packages[package_id] = package
        return package

    @asynccontextmanager
    async def session(self, *, readonly: bool = False, autocommit: bool = False):
        """Gets exclusive query and edit access to the main session."""
        async with self._tx_lock:
            assert self._session is not None, f"session not ready in {self!r}"
            was_readonly = self._session._is_readonly
            self._session._is_readonly = readonly
            self._session.unsuspend()
            yield self._session
            if autocommit:
                await self._session.commit()
            elif self._session.tx.edits:
                raise RuntimeError(f"uncommitted edits in {self!r}: {self._session.tx.edits!r}")
            self._session.suspend()  # suspend by default
            self._session._is_readonly = was_readonly

    async def start(self):
        start = monotime()

        # setup host
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
                node_types=BENCH_NODE_TYPES | IN_PACKAGE_NODE_TYPES,
                remote=self._host,
                retry=REMOTE_CONNECTION_RETRY,
                rpc_metadata=self._rpc_metadata,
            ),
        )
        self._connector = RemoteConnector(self._host, bench_scope, self._rpc_metadata)
        self._session = Session(
            _is_readonly=False,
            _default_scope=bench_scope,
            _engines=self._engines,
            _supervisor=self._supervisor,
            _host=self._host,
        )
        await self._session.open(in_context=False)

        # connect
        async with self.session(readonly=True):
            # connect bench
            self._bench = await self._connector.connect(
                BENCH_QUERY.where(id=self._bench_id), self._tx_lock, self._session
            )
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
            self._main_package = await self._connector.connect(
                PACKAGE_QUERY.where(id=main_branch.main_package_id), self._tx_lock, self._session
            )
            self._packages[main_branch.main_package_id] = self._main_package
            self._session.parent = self._main_package.node

        # start threads
        for _ in range(RUNTIME_PARALLELISM):
            thread = RuntimeThread(
                id=UUIDT(),
                bench_id=self._bench_id,
                supervisor=self._supervisor,
                host=self._host,
                # TODO :Performance: share query connections between runtime/threads
                connector=self._connector,
                engines=self._engines,
                client=self._client,
                machine=self._machine,
                queue=self._run_queue,
            )
            self._threads.append(thread)
            await thread.start()

        logger.info(
            "runtime.start",
            runtime=self,
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
    async def queue_run(self, request: QueueRunRequest) -> QueueRunResponse:
        # just add to main queue
        self._run_queue.put_nowait(request.run)
        logger.trace("runtime.queue_run", run=request.run)
        return QueueRunResponse()


async def get_host_client(bench_id: UUID, supervisor: SupervisorStub) -> HostStub:  # noqa: RUF029
    # NOTE :Scalability: lookup bench host (via supervisor?) :SingleHostService
    return HostStub(supervisor.channel)
