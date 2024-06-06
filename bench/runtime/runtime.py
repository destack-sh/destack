import asyncio
from contextlib import asynccontextmanager
from typing import cast, override
from urllib.parse import urlparse
from uuid import UUID

import structlog
from grpclib.client import Channel
from opentelemetry import trace

from bench.language import Bench, Package
from bench.language.bench import Client, Machine, Server
from bench.language.connection import RemoteEngine
from bench.language.const import (
    BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    PUBLIC_NODE_TYPES,
    ClientType,
    RunStatus,
)
from bench.language.run import Run
from bench.language.session import Session, unsuspend_session
from bench.proto import wire, wiring
from bench.proto.services import ServiceBase
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
    RUNTIME_CONCURRENCY,
)
from bench.runtime.thread import RuntimeThread
from bench.utils.func import CriticalLock
from bench.utils.tenacity import RETRY_GRPC_FOREVER

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Runtime(ServiceBase, RuntimeBase):
    """
    A Runtime processes selected Runs in a Bench/Package in Sessions on a Client.
    """

    kind = ServiceKind.INTERNAL  # :ServiceKind

    def __init__(
        self,
        *,
        supervisor_url: str,
        bench_id: UUID,
        client_type: ClientType,
        client_id: UUID,
        client_access_token: str,
        machine_id: UUID | None,
    ):
        super().__init__(logger=logger, tracer=tracer)

        # parse out supervisor host and port
        _supervisor_url = urlparse(supervisor_url)
        self._supervisor_host = _supervisor_url.hostname
        self._supervisor_port = _supervisor_url.port
        if self._supervisor_host is None or self._supervisor_port is None:
            raise ValueError(f"invalid supervisor URL: {supervisor_url}")
        self._supervisor = SupervisorStub(Channel(self._supervisor_host, self._supervisor_port))

        # context
        if client_type == ClientType.BENCH_SERVER and machine_id is None:
            raise ValueError(f"missing machine_id for {client_type} {client_id}")
        self._client_type = client_type
        self._client_id = client_id
        self._client_access_token = client_access_token
        self._rpc_metadata = RpcMetadata(
            client_type=cast(wire.ClientType, client_type),
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
    def server(self) -> Server:
        assert isinstance(self.client.parent, Server), f"no server for {self!r}"
        return self.client.parent

    @property
    def main_package(self) -> Package:
        assert self._main_package is not None, f"no main package for {self!r}"
        return self._main_package.node

    @asynccontextmanager
    async def session(self, *, readonly: bool = False, autocommit: bool = False):
        """Gets exclusive query and edit access to the main session."""
        assert self._session is not None, f"no session for {self!r}"
        async with self._tx_lock, unsuspend_session(
            self._session, readonly=readonly, autocommit=autocommit
        ):
            yield self._session

    @tracer.start_as_current_span("runtime.start")
    async def start(self):
        # setup host
        self._host = await get_host_client(self._bench_id, self._supervisor)
        bench_scope = GraphScope(bench_id=str(self._bench_id))
        self._connector = RemoteConnector(self._host, bench_scope, self._rpc_metadata)
        self._engines = (
            # global engine
            RemoteEngine(
                scope=GraphScope(),
                node_types=PUBLIC_NODE_TYPES,
                remote=self._supervisor,
                retry=RETRY_GRPC_FOREVER,
                rpc_metadata=self._rpc_metadata,
            ),
            # bench engine
            RemoteEngine(
                scope=bench_scope,
                node_types=BENCH_NODE_TYPES | IN_PACKAGE_NODE_TYPES,
                remote=self._host,
                retry=RETRY_GRPC_FOREVER,
                rpc_metadata=self._rpc_metadata,
            ),
        )
        self._session = Session(
            _is_readonly=False,
            _default_scope=bench_scope,
            _engines=self._engines,
            _supervisor=self._supervisor,
            _host=self._host,
        )
        await self._session.open(in_context=False)

        # connect
        # NOTE :Performance: share query connections between runtime/threads?
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
            self._session.machine = self._machine
            self._session.client = self._client
            if isinstance(self._client.parent, Server):
                self._session.server = self._client.parent
            else:
                self._session.user = self._client.parent
            self._session._subject = self._client.parent
            self._session._origin = self._client.to_origin()

            # connect main package
            self._main_package = await self._connector.connect(
                PACKAGE_QUERY.where(id=main_branch.main_package_id), self._tx_lock, self._session
            )
            self._packages[main_branch.main_package_id] = self._main_package
            self._session.parent = self._main_package.node

        # start threads
        for i in range(RUNTIME_CONCURRENCY):
            thread = RuntimeThread(
                id=i,
                bench_id=self._bench_id,
                supervisor=self._supervisor,
                host=self._host,
                client=self._client,
                machine=self._machine,
                connector=self._connector,
                engines=self._engines,
                queue=self._run_queue,
            )
            self._threads.append(thread)
            await thread.start()

        logger.info(
            "runtime.start",
            runtime=self,
            bench=self._bench.node,
            client=self._client,
            span="current",
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
        assert self._client is not None, f"{self!r} not ready"

        # mark run as queued in this runtime
        run = wiring.unpack_object(
            request.run, parent=self.main_package, session=self._session, expect=Run
        )
        async with self.session(autocommit=True):
            run.status = RunStatus.QUEUED
            if isinstance(self._client.parent, Server):
                run.server = self._client.parent
            run.client = self._client
            run.machine = self._machine

        # just add to main queue
        self._run_queue.put_nowait(run._to_data())
        logger.trace("runtime.queue_run", run=run, span="current")
        return QueueRunResponse()


async def get_host_client(bench_id: UUID, supervisor: SupervisorStub) -> HostStub:  # noqa: RUF029
    # NOTE :Scalability: lookup bench host (via supervisor?) :SingleHostService
    return HostStub(supervisor.channel)
