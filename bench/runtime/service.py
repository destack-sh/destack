import asyncio
from dataclasses import dataclass
from typing import cast, override
from urllib.parse import urlparse
from uuid import UUID, uuid4

import cachetools
import grpclib
import grpclib.metadata
import structlog
from grpclib.client import Channel
from opentelemetry import trace

from bench.language import NodeReference
from bench.language.bench import Machine
from bench.language.const import (
    BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    PUBLIC_NODE_TYPES,
    ClientType,
    NodeType,
)
from bench.language.node import EMPTY_SCOPE
from bench.proto import wire
from bench.proto.networking import localize_url
from bench.proto.services import ServiceBase
from bench.proto.wire import (
    GraphScopeData,
    HostClient,
    KillRunRequest,
    KillRunResponse,
    PauseRunRequest,
    PauseRunResponse,
    ProcessRunRequest,
    ProcessRunResponse,
    ResolveHostsRequest,
    ResolveHostsRequestBenchKey,
    RpcMetadata,
    RunData,
    RuntimeBase,
    ServiceKind,
    SupervisorClient,
)
from bench.proto.wiring import pack_rpc_headers
from bench.runtime.remote import RemoteEngine
from bench.runtime.thread import RuntimeThread
from bench.utils.oracle import Oracle
from bench.utils.telemetry import set_baggage
from bench.utils.tenacity import RETRY_GRPC_FOREVER

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class ActiveRun:
    run: RunData
    thread: RuntimeThread | None
    task: asyncio.Task | None


class RuntimeService(ServiceBase, RuntimeBase):
    kind = ServiceKind.INTERNAL  # :ServiceKind

    def __init__(
        self,
        *,
        supervisor_url: str,
        bench_id: UUID,
        client_type: ClientType,
        client_id: UUID,
        client_access_token: str,
        server_id: UUID | None,
        machine_id: UUID | None,
        max_threads: int,
        oracle: Oracle,
    ):
        super().__init__(logger=logger, tracer=tracer, oracle=oracle)
        self._nonce = uuid4()

        # parse out supervisor host and port
        _supervisor_url = urlparse(supervisor_url)
        self._supervisor_host = _supervisor_url.hostname
        self._supervisor_port = _supervisor_url.port
        if self._supervisor_host is None or self._supervisor_port is None:
            raise ValueError(f"invalid supervisor URL: {supervisor_url}")
        _supervisor_channel = Channel(
            host=self._supervisor_host,
            port=self._supervisor_port,
            ssl=_supervisor_url.scheme == "https",
        )
        self._supervisor = SupervisorClient(_supervisor_channel)

        # context
        if client_type == ClientType.BENCH_MACHINE and machine_id is None:
            raise ValueError(f"missing machine_id for {client_type} {client_id}")
        self._client_type = client_type
        self._client_id = client_id
        self._client_access_token = client_access_token
        self._rpc_metadata = RpcMetadata(
            client_type=cast(wire.ClientType, client_type),
            client_id=str(self._client_id),
            client_access_token=self._client_access_token,
        )
        self._rpc_headers = pack_rpc_headers(self._rpc_metadata)
        self._server_id = server_id
        self._machine_id = machine_id
        self._machine: Machine | None = None
        self._engines: tuple[RemoteEngine, ...] = ()
        self._host: HostClient | None = None
        self._bench_id = bench_id
        self._bench_ptr = NodeReference(
            type=NodeType.BENCH, id=bench_id, ck=bench_id, bench_id=bench_id
        )

        # processing
        self._active_runs: list[ActiveRun] = []
        self._available_threads: asyncio.Queue[RuntimeThread] = asyncio.Queue()
        self._max_threads = max_threads
        self._threads: list[RuntimeThread] = []

    def __str__(self):
        return f"{self._client_id} on {self._bench_id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def host(self) -> HostClient:
        assert self._host is not None, f"no host for {self!r}"
        return self._host

    async def start(self):
        # setup host
        self._host = await self._get_host_client(self._bench_id)
        bench_scope = GraphScopeData(
            metatype=wire.ObjectType.GRAPH_SCOPE, bench_id=str(self._bench_id)
        )
        self._engines = (
            # global engine
            RemoteEngine(
                scope=EMPTY_SCOPE._to_data(),
                node_types=PUBLIC_NODE_TYPES,
                remote=self._supervisor,
                write_retry=RETRY_GRPC_FOREVER,
                rpc_metadata=self._rpc_metadata,
            ),
            # bench engine
            RemoteEngine(
                scope=bench_scope,
                node_types=BENCH_NODE_TYPES | IN_PACKAGE_NODE_TYPES,
                remote=self._host,
                write_retry=RETRY_GRPC_FOREVER,
                rpc_metadata=self._rpc_metadata,
            ),
        )

        # start threads
        assert self._max_threads > 0, f"no threads for {self!r}"
        for i in range(self._max_threads):
            # nocheckin: launch RuntimeThread in own process (if available i.e. not WASM)
            thread = RuntimeThread(
                id=i,
                bench_id=self._bench_id,
                supervisor=self._supervisor,
                host=self._host,
                client_id=self._client_id,
                server_id=self._server_id,
                machine_id=self._machine_id,
                engines=self._engines,
                oracle=self.oracle,
            )
            self._threads.append(thread)
            self._available_threads.put_nowait(thread)
            await thread.start()

        logger.info("runtime.start", runtime=self)

    def close(self):
        super().close()

    @tracer.start_as_current_span("runtime.process_run")
    async def _do_process_run(self, run: ActiveRun):
        set_baggage(bench_id=self._bench_id, client_id=self._client_id, run_id=run.run.id)
        self._active_runs.append(run)
        try:
            # acquire thread
            try:
                run.thread = await self._available_threads.get()
            except asyncio.CancelledError as e:
                # cancelled before we got a thread
                logger.trace("runtime.process_run.cancel", error=e)
                return

            # run in thread
            try:
                request = ProcessRunRequest(run=run.run, is_blocking=True)
                _ = await run.thread.process_run(request)
            finally:
                self._available_threads.put_nowait(run.thread)
        finally:
            self._active_runs.remove(run)

    @override
    async def process_run(self, request: ProcessRunRequest) -> ProcessRunResponse:
        run = ActiveRun(run=request.run, thread=None, task=None)
        run.task = asyncio.create_task(self._do_process_run(run))
        if request.is_blocking:
            await run.task
        return ProcessRunResponse()

    @override
    async def pause_run(self, request: PauseRunRequest) -> PauseRunResponse:
        # find active run
        for run in self._active_runs:
            if run.run.id == request.run.id:
                if run.thread is None:
                    # not yet running, cancel directly
                    assert run.task is not None, f"no task for {run!r}"
                    run.task.cancel()
                else:
                    # try to pause in thread
                    request = PauseRunRequest(run=run.run)
                    _ = await run.thread.pause_run(request)
                return PauseRunResponse(is_processed=True)
        else:
            return PauseRunResponse(is_processed=False)

    @override
    async def kill_run(self, request: KillRunRequest) -> KillRunResponse:
        # find active run
        for run in self._active_runs:
            if run.run.id == request.run.id:
                if run.thread is None:
                    # not yet running, cancel directly
                    assert run.task is not None, f"no task for {run!r}"
                    run.task.cancel()
                else:
                    # try to kill in thread
                    request = KillRunRequest(run=run.run)
                    _ = await run.thread.kill_run(request)
                return KillRunResponse(is_processed=True)
        else:
            return KillRunResponse(is_processed=False)

    @cachetools.cached({})
    async def _get_host_client(self, bench_id: UUID) -> HostClient:
        request = ResolveHostsRequest(benches=[ResolveHostsRequestBenchKey(id=str(bench_id))])
        retry = RETRY_GRPC_FOREVER.new(self.oracle)
        while retry.should_retry:
            retry.on_attempt()
            try:
                response = await self._supervisor.resolve_hosts(
                    request,
                    metadata=self._rpc_headers,
                    deadline=grpclib.metadata.Deadline.from_timeout(5),
                )
                host_info = response.hosts[0]
                host_info.domain = localize_url(host_info.domain)
                self.logger.info(
                    "runtime.resolve_host",
                    supervisor=self._supervisor,
                    bench_id=bench_id,
                    host_domain=host_info.domain,
                    host_port=host_info.grpc_port,
                )
                host_channel = Channel(
                    host=host_info.domain, port=host_info.grpc_port, ssl=host_info.ssl
                )
                return HostClient(host_channel)
            except Exception as e:
                interval = retry.get_wait_interval()
                self.logger.error(
                    "runtime.resolve_host.error", bench_id=bench_id, exc_info=e, interval=interval
                )
                if not retry.on_error(e):
                    raise
                await self.oracle.sleep(interval)
        else:
            raise retry.to_error(operation=request)
