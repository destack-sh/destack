import asyncio
from contextlib import asynccontextmanager
from typing import TYPE_CHECKING
from uuid import UUID

import structlog

from bench.language import Bench, Package
from bench.language.bench import Client, Machine
from bench.language.connection import StoreEngine
from bench.language.session import Session
from bench.proto import wiring
from bench.proto.wire import GraphScope, HostStub, RunData, SupervisorStub
from bench.runtime.connection import ConnectedBench, ConnectedPackage, QueryConnector
from bench.runtime.core import BENCH_QUERY, PACKAGE_QUERY
from bench.utils.dt import monotime
from bench.utils.func import CriticalLock
from bench.utils.task import TaskManager

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)


class RuntimeThread:
    """
    A thread for actually executing untrusted Runs in a Runtime in a specific Session.
    """

    def __init__(
        self,
        *,
        id: UUID,
        bench_id: UUID,
        supervisor: SupervisorStub,
        host: HostStub,
        connector: QueryConnector,
        engines: tuple[StoreEngine, ...],
        client: Client,
        machine: Machine | None,
        queue: asyncio.Queue[RunData],
    ):
        self.id = id

        # context
        self._client = client
        self._machine = machine
        self._connector = connector
        self._engines = engines

        # bench stuff
        self._supervisor = supervisor
        self._host = host
        self._bench_id = bench_id
        self._bench: ConnectedBench | None = None
        self._main_package: ConnectedPackage | None = None

        # processing
        self._session: Session | None = None
        self._tx_lock: asyncio.Lock = CriticalLock(
            name=f"{self.__class__.__name__}_{self._bench_id or ''}_{self.id}"
        )
        self._queue = queue
        self._tasks = TaskManager(owner=self, logger=logger)

    def __str__(self):
        return f"{self.id} in {self._client!r} on {self._bench!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

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

        # setup thread
        self._session = Session(
            _is_readonly=False,
            _default_scope=GraphScope(bench_id=str(self._bench_id)),
            _engines=self._engines,
            _supervisor=self._supervisor,
            _host=self._host,
        )
        await self._session.open(in_context=False)

        # connect
        async with self.session(readonly=True):
            self._bench = await self._connector.connect(
                BENCH_QUERY.where(id=self._bench_id), self._tx_lock, self._session
            )
            main_environment = self._bench.node.main_environment
            assert main_environment is not None, f"{self._bench!r} has no main environment"
            main_branch = self._bench.node.main_branch
            assert main_branch is not None, f"{self._bench!r} has no main branch"
            assert main_branch.main_package_id is not None, f"{main_branch!r} has no main package"
            self._main_package = await self._connector.connect(
                PACKAGE_QUERY.where(id=main_branch.main_package_id), self._tx_lock, self._session
            )

        # finally, start processing runs
        self._tasks.start_queue(
            self._queue, self._process_run, f"{self.bench.slug}_run{self.id}", skip_errors=True
        )
        logger.info(
            "thread.start",
            process=self,
            bench=self._bench,
            client=self._client,
            duration=monotime() - start,
        )

    async def _process_run(self, run_data: RunData):
        assert (
            run_data.parent_ptr and run_data.parent_ptr.id == self.main_package.id
        ), f"{run_data!r} not in {self.main_package!r}"
        run = wiring.unpack_node(run_data, parent=self.main_package, session=self._session)

        raise NotImplementedError(f"nocheckin: _process_run {run!r}")

    def close(self):
        self._tasks.close()

    async def wait_closed(self):
        await self._tasks.wait_closed()
