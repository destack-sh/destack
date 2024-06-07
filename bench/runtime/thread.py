import asyncio
from contextlib import asynccontextmanager
from typing import TYPE_CHECKING
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import Bench, Package, User
from bench.language.bench import Client, Machine
from bench.language.code_ import Code, run_code_exec
from bench.language.connection import StoreEngine
from bench.language.const import BlockType, RunKind, RunStatus, _active_run
from bench.language.run import Run, RunError
from bench.language.session import Session, unsuspend_session
from bench.language.validation import on_invalid_raise
from bench.language.value import check_value
from bench.proto import wiring
from bench.proto.wire import GraphScope, HostStub, RunData, SupervisorStub
from bench.runtime.connection import ConnectedBench, ConnectedPackage, QueryConnector
from bench.runtime.core import BENCH_QUERY, PACKAGE_QUERY
from bench.utils.dt import utcnow
from bench.utils.func import CriticalLock
from bench.utils.task import TaskManager

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class RuntimeThread:
    """
    A thread for actually executing untrusted Runs in a Runtime in a specific Session.
    """

    def __init__(
        self,
        *,
        id: int,
        bench_id: UUID,
        supervisor: SupervisorStub,
        host: HostStub,
        client: Client,
        machine: Machine | None,
        connector: QueryConnector,
        engines: tuple[StoreEngine, ...],
        queue: asyncio.Queue[RunData],
    ):
        self.id = id

        # bench stuff
        self._supervisor = supervisor
        self._host = host
        self._bench_id = bench_id
        self._bench: ConnectedBench | None = None
        self._main_package: ConnectedPackage | None = None

        # context
        self._client = client
        self._machine = machine
        self._connector = connector
        self._engines = engines

        # processing
        self._session: Session | None = None
        self._tx_lock: asyncio.Lock = CriticalLock(
            name=f"{self.__class__.__name__}_{self._bench_id or ''}_{self.id}"
        )
        self._queue = queue
        self._tasks = TaskManager(owner=self, logger=logger)

    def __str__(self):
        bench = self._bench._node if self._bench else None
        return f"{self.id} on {repr(bench) or self._bench_id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def bench(self) -> Bench:
        assert self._bench is not None, f"no bench for {self!r}"
        return self._bench.node

    @property
    def main_package(self) -> Package:
        assert self._main_package is not None, f"no main package for {self!r}"
        return self._main_package.node

    @property
    def epoch(self) -> int:
        assert self._bench is not None, f"no bench for {self!r}"
        assert self._main_package is not None, f"no main package for {self!r}"
        return max(self._bench.epoch, self._main_package.epoch)

    @asynccontextmanager
    async def session(self, *, readonly: bool = False, autocommit: bool = False):
        """Gets exclusive query and edit access to the main session."""
        assert self._session is not None, f"no session for {self!r}"
        async with self._tx_lock, unsuspend_session(
            self._session, readonly=readonly, autocommit=autocommit
        ):
            yield self._session

    @tracer.start_as_current_span("thread.start")
    async def start(self):
        # setup thread
        self._session = Session(
            client=self._client,
            machine=self._machine,
            server=self._machine.parent if self._machine else None,
            user=self._client.parent if isinstance(self._client.parent, User) else None,
            _is_readonly=False,
            _default_scope=GraphScope(bench_id=str(self._bench_id)),
            _engines=self._engines,
            _supervisor=self._supervisor,
            _host=self._host,
            _subject=self._client.parent,
            _origin=self._client.to_origin(),
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
            self._session.parent = self._main_package.node

        # finally, start processing runs
        self._tasks.start_queue(
            self._queue, self._process_run, f"{self.bench.slug}_run{self.id}", skip_errors=True
        )
        logger.info("thread.start", process=self, bench=self._bench, span="current")

    async def _process_run(self, run_data: RunData):
        assert self._main_package is not None, f"no main package for {self!r}"
        package = self._main_package.node
        assert (
            run_data.parent_ptr and UUID(run_data.parent_ptr.id) == package.id
        ), f"{run_data!r} not in {package!r}"
        run = wiring.unpack_object(run_data, parent=package, session=self._session, expect=Run)

        async with self.session(readonly=False, autocommit=True):
            run.status = RunStatus.RUNNING
            run.started_at = utcnow()
            run.started_epoch = self.epoch
            run_token = _active_run.set(run)
            logger.info("run.start", thread=self, run=run)
            try:
                if run.kind == RunKind.BLOCK:
                    block = run.block
                    assert block is not None, f"no block for {run!r}"
                    check_value(run.inputs, block.as_type, on_invalid_raise)
                    if block.type == BlockType.CODE:
                        code = block.code or Code.empty()
                        run_code_exec(code.to_string(), {"self": block})
                    elif block.type == BlockType.TEXT:
                        ...  # nocheckin
                    else:
                        raise NotImplementedError(f"unsupported block type {block.type}")
                else:
                    raise NotImplementedError(f"unsupported run kind {run.kind}")
                run.status = RunStatus.COMPLETED
                logger.info("run.complete", thread=self, run=run)
            except Exception as e:
                run.fail(RunError.from_exception(e))
                logger.error("run.fail", thread=self, run=run, error=e, exc_info=e)
            finally:
                run.terminated_at = utcnow()
                run.terminated_epoch = self.epoch
                run.duration = (run.terminated_at - run.started_at).total_seconds()
                _active_run.reset(run_token)

    def close(self):
        self._tasks.close()

    async def wait_closed(self):
        await self._tasks.wait_closed()
