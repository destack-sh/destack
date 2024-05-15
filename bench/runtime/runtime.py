from contextlib import asynccontextmanager
from urllib.parse import urlparse
from uuid import UUID

import structlog
from grpclib.client import Channel

from bench.language import Bench, NodeType, Package, Session
from bench.language.connection import RemoteEngine
from bench.language.const import BENCH_NODE_TYPES
from bench.proto.services import MonitoredServiceBase
from bench.proto.wire import GraphScope, HostStub, RuntimeBase, SupervisorStub

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
        self._bench: Bench | None = None
        self._main_package: Package | None = None
        self._packages: dict[UUID, Package] = {}

    def __str__(self):
        return f"{self._bench_id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def host(self) -> HostStub:
        assert self._host is not None, f"no host for {self!r}"
        return self._host

    async def start(self):
        self._host = await get_host_client(self._bench_id, self._supervisor)
        async with local_session(self._supervisor, self._bench_id, self._host):
            # load bench & main packages
            self._bench = await BENCH_QUERY.get(id=self._bench_id)
            assert self._bench.main_branch is not None, f"{self._bench!r} has no main branch"
            self._main_package = await PACKAGE_QUERY.get(id=self._bench.main_branch.package_id)
            self._packages[self._main_package.id] = self._main_package

        # nocheckin: watch for edits in runtime, reconnect & re-watch on error (like in bench-web)

    def close(self):
        pass

    async def wait_closed(self):
        pass


@asynccontextmanager
async def local_session(supervisor: SupervisorStub, bench_id: UUID, host: HostStub):
    bench_scope = GraphScope(bench_id=str(bench_id))
    # nocheckin: auto-retry engines on error? (or only remote?)
    engines = (
        RemoteEngine(default_scope=bench_scope, node_types=BENCH_NODE_TYPES, remote=host),
        RemoteEngine(default_scope=bench_scope, node_types=LOADED_PACKAGE_NODE_TYPES, remote=host),
    )
    async with Session(_supervisor=supervisor, _host=host, _engines=engines) as session:
        yield session


async def get_host_client(bench_id: UUID, supervisor: SupervisorStub) -> HostStub:
    # TODO :Scalability: lookup bench host (via supervisor?) :SingleHostService
    return HostStub(supervisor.channel)
