import abc
import asyncio
import enum
from contextlib import asynccontextmanager
from typing import Any, override
from uuid import UUID

import cachetools
import structlog
from opentelemetry import trace

from bench import pb2
from bench.language import (
    BENCH_NODE_TYPES,
    EMPTY_SCOPE_DATA,
    NONCE,
    PUBLIC_NODE_TYPES,
    SOURCE_NODE_TYPES,
    STATIC_RESOURCE_NODE_TYPES,
    Bench,
    Client,
    ClientType,
    GraphScope,
    Machine,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    Package,
    RemoteEngine,
    Session,
    User,
)
from bench.pb2 import GraphScopeData, HostClient, ResolveHostsRequest, SupervisorClient
from bench.proto import Network, ServiceBase, get_rpc_metadata, localize_url, pack_rpc_headers
from bench.utils.oracle import Oracle
from bench.utils.sync import CriticalLock
from bench.utils.tenacity import RETRY_GRPC_FOREVER

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

BENCH_QUERY = Bench.include_descendants(
    NodeType.HANDLE, NodeType.PACKAGE, *STATIC_RESOURCE_NODE_TYPES
).select_all()
PACKAGE_QUERY = (
    Package.include_descendants(*SOURCE_NODE_TYPES)
    .include_ancestors(Bench)
    .select_all()
    .deselect(Bench.encryption_key)
)


class RuntimeThreadMode(enum.StrEnum):
    LOCAL = "local"
    PROCESS = "process"


class RuntimeServiceBase(ServiceBase, abc.ABC):
    """
    Common base for RuntimeService/RuntimeThread.
    """

    def __init__(
        self,
        *,
        id: str,
        logger: Any,
        tracer: trace.Tracer,
        network: Network,
        oracle: Oracle,
        bench_id: UUID,
        supervisor: SupervisorClient,
        client_type: ClientType,
        client_id: UUID,
        client_access_token: str,
        machine_id: UUID | None,
        mode: "RuntimeThreadMode",
    ):
        super().__init__(id=id, logger=logger, tracer=tracer, network=network, oracle=oracle)
        self._mode = mode

        # services
        self._supervisor = supervisor
        self._host: HostClient | None = None
        if client_type == ClientType.MACHINE and machine_id is None:
            raise ValueError(f"missing machine_id for {client_type} {client_id}")
        self._client_type = client_type
        self._client_id = client_id
        self._client_access_token = client_access_token
        self._rpc_metadata = get_rpc_metadata(
            client_type=client_type,
            client_id=client_id,
            client_access_token=client_access_token,
        )
        self._rpc_headers = pack_rpc_headers(self._rpc_metadata)

        # bench
        self._bench_id = bench_id
        self._bench_ptr = NodeReference(
            node_type=NodeType.BENCH, id=bench_id, ck=bench_id, bench_id=bench_id
        )
        self._supergraph = NodeSuperGraph(name="Runtime", root_ptr=self._bench_ptr)
        self._bench: Bench | None = None
        self._main_package: Package | None = None
        self._session: Session | None = None
        self._tx_lock: asyncio.Lock = CriticalLock(
            name=f"{self.__class__.__name__}_{self._bench_id or ''}"
        )
        self._machine_id = machine_id
        self._machine: Machine | None = None
        self._client_id = client_id
        self._client: Client | None = None
        self._engines: tuple[RemoteEngine, ...] = ()

    @property
    def bench(self) -> Bench:
        assert self._bench is not None, f"no bench for {self!r}"
        return self._bench

    @property
    def main_package(self) -> Package:
        assert self._main_package is not None, f"no main package for {self!r}"
        return self._main_package

    @asynccontextmanager
    async def session(self, *, readonly: bool = False):
        """Gets exclusive query and edit access to the main session."""
        assert self._session is not None, f"no session for {self!r}"
        async with self._tx_lock, self._session.active(readonly=readonly):
            yield self._session

    @override
    async def start(self):
        # setup host
        self._host = await self.resolve_host_client(self._bench_id)
        bench_scope = GraphScopeData(
            metatype=pb2.ObjectType.OBJECT_TYPE_GRAPH_SCOPE, bench_id=str(self._bench_id)
        )
        self._engines = (
            # global engine
            RemoteEngine(
                name="global",
                scope=EMPTY_SCOPE_DATA,
                node_types=PUBLIC_NODE_TYPES,
                remote=self._supervisor,
                write_retry=RETRY_GRPC_FOREVER,
                rpc_metadata=self._rpc_metadata,
            ),
            # bench engine
            RemoteEngine(
                name="bench",
                scope=bench_scope,
                node_types=BENCH_NODE_TYPES,
                remote=self._host,
                write_retry=RETRY_GRPC_FOREVER,
                rpc_metadata=self._rpc_metadata,
            ),
        )

        # setup session
        self._session = Session(
            _is_readonly=False,
            _default_scope=GraphScope(bench_id=self._bench_id)._to_data(),
            _engines=self._engines,
            _supervisor=self._supervisor,
            _host=self._host,
            _supergraph=self._supergraph,
            _oracle=self._oracle,
        )
        await self._session.open(_set_in_context=False)

        # connect to host
        async with self.session(readonly=True):
            # get bench
            self._bench = await BENCH_QUERY.get(self._bench_ptr, live=True)
            self._session.parent = self._bench
            self._client = await Client.get(id=self._client_id)
            assert self._client, f"{self._bench!r} has no client {self._client_id}"
            if self._machine_id:
                self._machine = await Machine.get(id=self._machine_id)

            # get package
            self._main_package = await PACKAGE_QUERY.get(self._bench.main_package_ptr, live=True)

        # update session context
        self._session.client = self._client
        self._session.machine = self._machine
        if isinstance(self._client.parent, User):
            self._session.user = self._client.parent
            self._session._subject = self._client.parent
        elif isinstance(self._client.parent, Bench):
            self._session._subject = self._client.machine
        else:
            raise ValueError(f"unknown client parent {self._client.parent!r} in {self!r}")
        self._session._origin = (
            self._client.to_origin(nonce=NONCE)._to_data() if self._client else None
        )

    @cachetools.cached({})
    @tracer.start_as_current_span("runtime.resolve_host_client")
    async def resolve_host_client(self, bench_id: UUID) -> HostClient:
        request = ResolveHostsRequest(benches=[ResolveHostsRequest.BenchKey(id=str(bench_id))])
        retry = RETRY_GRPC_FOREVER.new(self.oracle)
        while retry.should_retry:
            retry.on_attempt()
            try:
                response = await self._supervisor.resolve_hosts(
                    request, metadata=self._rpc_headers, timeout=(5)
                )
                host_info = response.hosts[0]
                assert host_info.domain, f"no domain for {host_info!r}"
                domain = localize_url(host_info.domain)
                self.logger.info(
                    "runtime.resolve_host",
                    supervisor=self._supervisor,
                    bench_id=bench_id,
                    host_domain=host_info.domain,
                    host_port=host_info.grpc_port,
                )
                protocol = "https" if host_info.ssl else "http"
                connection_uri = f"{protocol}://{domain}:{host_info.grpc_port}"
                host_channel = self.network.get_channel(connection_uri, source_id=self.id)
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
