import asyncio
from typing import cast, override
from urllib.parse import urlparse
from uuid import UUID, uuid4

import cachetools
import structlog
from grpclib.client import Channel
from opentelemetry import trace

from bench.language import NodeReference
from bench.language.bench import Machine
from bench.language.connection import RemoteEngine
from bench.language.const import (
    BENCH_NODE_TYPES,
    IN_PACKAGE_NODE_TYPES,
    PUBLIC_NODE_TYPES,
    ClientType,
    NodeType,
)
from bench.language.node import EMPTY_SCOPE
from bench.proto import wire
from bench.proto.services import ServiceBase
from bench.proto.wire import (
    GraphScopeData,
    HostClient,
    QueueRunRequest,
    QueueRunResponse,
    ResolveHostsRequest,
    ResolveHostsRequestBenchKey,
    RpcMetadata,
    RunData,
    RuntimeBase,
    ServiceKind,
    SupervisorClient,
)
from bench.proto.wiring import pack_rpc_headers
from bench.runtime.thread import RuntimeThread
from bench.utils.oracle import REAL_ORACLE, Oracle
from bench.utils.tenacity import RETRY_GRPC_FOREVER, retry

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
        self._supervisor = SupervisorClient(Channel(self._supervisor_host, self._supervisor_port))

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
        self._start_queue: asyncio.Queue[RunData] = asyncio.Queue()
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
        # NOTE :Incomplete: start threads as actual threads/processes (if available i.e. not WASM)
        assert self._max_threads > 0, f"no threads for {self!r}"
        for i in range(self._max_threads):
            thread = RuntimeThread(
                id=i,
                bench_id=self._bench_id,
                supervisor=self._supervisor,
                host=self._host,
                client_id=self._client_id,
                server_id=self._server_id,
                machine_id=self._machine_id,
                engines=self._engines,
                process_queue=self._start_queue,
                oracle=self.oracle,
            )
            self._threads.append(thread)
            await thread.start()

        logger.info("runtime.start", runtime=self)

    def close(self):
        super().close()

    @override
    async def queue_run(self, request: QueueRunRequest) -> QueueRunResponse:
        # just add to start queue
        self._start_queue.put_nowait(request.run)
        logger.trace("runtime.queue_run", run=request.run, span="current")
        return QueueRunResponse()

    @cachetools.cached({})
    @retry(RETRY_GRPC_FOREVER, REAL_ORACLE)
    @tracer.start_as_current_span("runtime.resolve_host")
    async def _get_host_client(self, bench_id: UUID) -> HostClient:
        request = ResolveHostsRequest(benches=[ResolveHostsRequestBenchKey(id=str(bench_id))])
        response = await self._supervisor.resolve_hosts(request, metadata=self._rpc_headers)
        host_info = response.hosts[0]
        self.logger.info("runtime.resolve_host", bench_id=bench_id, host_info=host_info)
        host_channel = Channel(host_info.domain, host_info.grpc_port)
        return HostClient(host_channel)
