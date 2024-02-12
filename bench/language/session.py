import asyncio
import dataclasses
from dataclasses import dataclass
import sys
import threading
from collections import defaultdict, deque
from concurrent.futures import ThreadPoolExecutor
from contextvars import ContextVar
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Collection,
    Optional,
    Union,
)
from uuid import UUID, uuid4

import psycopg
import structlog

from bench.language.const import (
    EditType,
    NodeTrackingLevel,
    NodeType,
    RunStatus,
    StructType,
    TriggerType,
    _active_session,
)
from bench.language.field import TypeInfo
from bench.language.node import (
    UNSET,
    Node,
    Package,
    ScopeNode,
    Struct,
    get_node_id,
    node,
    p_internal,
    p_parent,
    p_runtime,
    struct,
    p_system,
    Bench,
    Environment,
    Branch,
    Property,
)
from bench.language.run import Run, RunError
from bench.search.client import get_os_errors, os_client
from bench.proto.wire import BenchHostStub, SupervisorStub, EditData
from bench.sql.client import get_pg_connection_pool
from bench.sql.core import PrimitiveType
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import _auto_async_to_sync
from bench.utils.env import IS_DEBUG

if TYPE_CHECKING:
    from bench.language import Block, Server, Trigger

logger = structlog.get_logger(__name__)


@struct(StructType.LOG_ENTRY, index_in_search=True)
class LogEntry(Struct):
    """An entry. In a log."""

    id: UUID = p_internal(2, default_factory=uuid4)
    package: Package = p_internal(5, require=True, array=False, references=NodeType.PACKAGE)
    bench: Package = p_internal(6, require=True, array=False, references=NodeType.BENCH)
    created_at: datetime = p_internal(32, default_factory=utcnow_with_tz)
    stream: str = p_internal(33)
    session: "Session" = p_internal(34, require=False, array=True, references=NodeType.SESSION)
    level: Optional[str] = p_internal(35, default=None)
    logger: Optional[str] = p_internal(36, default=None)
    block: Optional["Block"] = p_internal(37, require=False, array=False, references=NodeType.BLOCK)
    run: Optional["Run"] = p_internal(38, require=False, array=False, references=NodeType.RUN)
    message: Optional[str] = p_internal(39, default=None)

    def __content_str__(self):
        return f"'{self.message}' ({self.created_at})"


@struct(StructType.CONTEXT)
class Context(Struct):
    """A semi-magical value that accumulates context down the graph (starting with system context)."""

    # system
    bench: Optional[Bench] = p_internal(30, require=False, array=False, references=NodeType.BENCH)
    environment: Optional[Environment] = p_internal(
        31, require=False, array=False, references=NodeType.ENVIRONMENT
    )
    branch: Optional[Branch] = p_internal(
        32, require=False, array=False, references=NodeType.BRANCH
    )
    package: Optional[Package] = p_internal(
        33, require=False, array=False, references=NodeType.PACKAGE
    )
    module: Optional["Block"] = p_internal(
        34, require=False, array=False, references=NodeType.BLOCK
    )
    page: Optional["Block"] = p_internal(35, require=False, array=False, references=NodeType.BLOCK)
    # log: ...

    # custom
    # value: ...


dcfield = dataclasses.field


@dataclass(slots=True)
class Transaction:
    """
    A transaction in the Bench state graph.
    Edits in a transaction are atomic (in our primary Postgres/Relational stores).
    TODO @Cleanup: Transaction should be a Struct (along with Edit)
      (but we don't have a simple way of representing Edit.node/Edit.properties yet)
    """

    id: UUID = dcfield(default_factory=uuid4)
    is_runtime: bool = dcfield(default=False)

    global_pg_connection: Optional[psycopg.AsyncConnection] = dcfield(default=None)
    local_pg_connection: Optional[psycopg.AsyncConnection] = dcfield(default=None)
    global_host: Optional["SupervisorStub"] = dcfield(default=None)
    bench_host: Optional["BenchHostStub"] = dcfield(default=None)

    edits: list[EditData] = dcfield(default_factory=list)
    pending_edits: list[EditData] = dcfield(default_factory=list)
    _pending_updates_idx: dict[Node, int] = dcfield(default_factory=dict)

    # for syncing databases (should probably generalize into' untracked edits')
    _changed_record_ids_by_base_id: dict[UUID, set[UUID]] = dcfield(
        default_factory=lambda: defaultdict(set)
    )
    _schema_changed: bool = dcfield(default=False)

    @property
    def has_edits(self) -> bool:
        return len(self.edits) > 0 or len(self._changed_record_ids_by_base_id) > 0

    @staticmethod
    def from_existing(edits: Collection[EditData]):
        tx = Transaction()
        tx.edits.extend(edits)
        tx.pending_edits.extend(edits)
        for edit in edits:  # replay edits
            tx._on_edit(edit)
        return tx

    #
    # Edits
    #

    def _make_edit(self, type: EditType, n: Node):
        """Creates an edit and adds it to the pending edits."""
        from bench.proto import wiring

        # TODO @Performance: pack only edited node properties
        node_data = n._to_data()
        if n._updated_properties:
            properties = n._updated_properties.search(True)
        else:
            properties = None
        edit = EditData(
            type=wiring.pack_enum(EditType, type),
            node_type=node_data.metatype,
            node=wiring.wrap_some_node(node_data),
            properties=properties,
        )
        self.edits.append(edit)
        self.pending_edits.append(edit)

    def _on_edit(self, edit: EditData):
        if edit.node_type == NodeType.FIELD:
            self._schema_changed = True

    def create(self, n: Node):
        self._make_edit(EditType.CREATE, n)

    def upsert(self, n: Node):
        self._make_edit(EditType.UPSERT, n)

    def update(self, n: Node, properties: tuple[Property, ...]):
        from bench.proto import wiring

        current_update_idx = self._pending_updates_idx.get(n)
        if current_update_idx is not None:
            # update edit in place to avoid re-packing everything for successive updates
            #  (this is almost always correct in user code as package nodes ref with ck, not ids,
            #   in our own code, we just flush if we need a create first)
            update = self.pending_edits[current_update_idx]
            update.properties = n._updated_properties.search(True)
            node_data = wiring.unwrap_some_node(update.node)
            for prop in properties:
                if prop.reference_wired_ptr:
                    prop = prop.reference_wired_ptr
                value = getattr(struct, prop.name)
                value = wiring._pack_struct_prop(prop, value, ignore_array=False)
                setattr(node_data, prop.name, value)
        else:
            self._make_edit(EditType.UPDATE, n)

    def move(self, n: Node):
        self._make_edit(EditType.MOVE, n)

    def soft_delete(self, n: Node):
        self._make_edit(EditType.SOFT_DELETE, n)

    def restore(self, n: Node):
        self._make_edit(EditType.RESTORE, n)

    def archive(self, n: Node):
        self._make_edit(EditType.ARCHIVE, n)

    def unarchive(self, n: Node):
        self._make_edit(EditType.UNARCHIVE, n)

    def delete(self, n: Node):
        self._make_edit(EditType.DELETE, n)

    def _records_changed(self, database: "Block", record_ids: Collection[UUID]):
        self._changed_record_ids_by_base_id[database.id].update(record_ids)

    #
    # Transaction management
    #

    async def open(self):
        pass

    async def flush(self):
        """Flushes any pending edits to the primary stores (without committing)."""
        raise NotImplementedError

    async def commit(self):
        """Commits the transaction (flushing any pending edits). Syncs to secondary stores."""
        raise NotImplementedError

    async def rollback(self):
        """Rolls back uncommitted edits in primary stores."""
        raise NotImplementedError

    async def close(self):
        """Closes the transaction and associated cursors, rolling back uncommitted edits."""
        if self.global_pg_cursor:
            await self.global_pg_cursor.connection.rollback()
        for pg_name, pg_cursor in self.local_pg_cursors.items():
            await pg_cursor.connection.rollback()
            pg_pool = await get_pg_connection_pool(pg_name)
            await pg_pool.putconn(pg_cursor.connection)


_executor: ThreadPoolExecutor | None = ThreadPoolExecutor(max_workers=1)


@node(NodeType.SESSION, index_in_search=True, local=True)
class Session(ScopeNode):
    """
    A managed session for interacting with Bench nodes and (if on a Server) running them.
    """

    parent: Package = p_parent(4, NodeType.PACKAGE, is_system=True)
    server: Optional["Server"] = p_system(
        31, require=False, array=False, references=NodeType.SERVER
    )
    opened_at: Optional[datetime] = p_system(32, default=None)
    closed_at: Optional[datetime] = p_system(33, default=None)
    is_runtime: bool = p_system(34, default=False)

    # transaction
    _tx: Transaction | None = p_runtime(default=None)
    _global_pg_connection: Optional[psycopg.AsyncConnection] = p_runtime(default=None)
    _local_pg_connection: Optional[psycopg.AsyncConnection] = p_runtime(default=None)
    _supervisor: Optional["SupervisorStub"] = p_runtime(default=None)
    _host: Optional["BenchHostStub"] = p_runtime(default=None)
    _dangling_nodes_by_ck: dict[UUID, Node] = p_runtime(default_factory=dict)

    # runtime
    _stacktrace: list[Run] | None = p_runtime(default_factory=list)
    _runs_by_id: dict[UUID, Run] | None = p_runtime(default=None)
    _pending_runs_by_id: dict[UUID, Run] | None = p_runtime(default=None)
    _active_nodes_by_ck: dict[UUID, Node] | None = p_runtime(default=None)
    _runtime_tracing_lock: threading.Lock = p_runtime(default_factory=threading.Lock)

    # logs
    _cached_logs: deque[LogEntry] | None = p_runtime(default=None)
    _pending_logs: list[LogEntry] | None = p_runtime(default=None)
    _flush_session_loop: asyncio.Task | None = p_runtime(default=None)
    _stdout_collector: Optional["LogCollector"] = p_runtime(default=None)
    _stderr_collector: Optional["LogCollector"] = p_runtime(default=None)

    def __content_str__(self):
        if self.closed_at:
            status = "closed"
        elif self.opened_at:
            status = "open"
        else:
            status = "pending"
        return (
            f"{status}, "
            f"{self._tx or '<no tx>'}, "
            f"{len(self._runs_by_id) if self._runs_by_id is not None else 0} runs"
        )

    def _init_inner(self) -> None:
        self._session = self

    @property
    def pending_nodes(self) -> tuple[Node, ...]:
        return tuple(n for n in self._dangling_nodes_by_ck.values() if not n.parent)

    @property
    def has_edits(self) -> bool:
        """Whether this session has any non-session edits."""
        return self._tx is not None and self._tx.has_edits

    @property
    def is_open(self) -> bool:
        return self.opened_at is not None and self.closed_at is None

    @property
    def is_closed(self) -> bool:
        return self.closed_at is not None

    @property
    def host(self) -> "BenchHostStub":
        """The remote host."""
        assert self._host is not None, f"host not available in {self!r}"
        return self._host

    async def open(self, session_flush_interval: float = 0.1):
        """Opens the session for regular business."""

        if self.opened_at is not None:
            raise RuntimeError(f"session already open: {self!r}")
        if _active_session.get() is not None:
            raise RuntimeError(f"another session is active: {_active_session.get()!r}")
        _active_session.set(self)
        logger.debug("session.open")

        # prepare runtime
        if self.is_runtime:
            # log collection
            self._pending_logs = []
            self._cached_logs = deque(maxlen=LOG_CACHE_SIZE)
            self._stdout_collector = LogCollector(self._track_log, "stdout", self)
            self._stderr_collector = LogCollector(self._track_log, "stderr", self)
            self._stdout_collector.start()
            self._stderr_collector.start()

            # auto-flush session/runs/etc.
            async def _flush_session_loop():
                while True:
                    await asyncio.sleep(session_flush_interval)
                    await self.flush_session()
                    await self._flush_logs()

            self._flush_session_loop = asyncio.create_task(_flush_session_loop())
            self._runs_by_id = {}
            self._pending_runs_by_id = {}
            self._active_nodes_by_ck = {}
            self._stacktrace = []

        # open transaction
        self._tx = Transaction(is_runtiem=self.is_runtime)

        self.opened_at = utcnow_with_tz()
        logger.debug("session.open.done")

    @_auto_async_to_sync
    async def flush(self):
        assert self.is_open, f"cannot flush {self!r} when closed"
        await self._tx.flush()
        raise NotImplementedError

    @_auto_async_to_sync
    async def commit(self) -> Collection[EditData]:
        assert self.is_open, f"cannot commit {self!r} when closed"
        await self._tx.commit()
        return self._tx.edits

    @_auto_async_to_sync
    async def rollback(self):
        assert self.is_open, f"cannot rollback {self!r} when closed"
        await self._tx.rollback()

    @_auto_async_to_sync
    async def close(self):
        """Closes the session, rolling back uncommitted edits. Prevents further runs/edits."""
        if self.closed_at is not None:
            raise RuntimeError(f"session already closed {self}")
        logger.debug("session.close")

        # close transaction
        await self._tx.close()

        # close session
        self.closed_at = utcnow_with_tz()
        _active_session.set(None)

        # close runtime
        if self.is_runtime:
            self._stdout_collector.stop()
            self._stderr_collector.stop()
            self._flush_session_loop.cancel()

        if self.pending_nodes:
            logger.warn("session.close.dangling", dangling=self.pending_nodes)

    #
    # Tracking
    #

    def track(self, node: Node):
        """Start tracking the node in this session."""
        if node._session != self:
            node._track_self(self)

    def track_many(self, *nodes: Node):
        """Start tracking the nodes in this session."""
        for n in nodes:
            if n._session != self:
                n._track_self(self)

    def untrack(self, node: Node):
        """Stop tracking the node in this session."""
        node._untrack_self(self)

    def untrack_many(self, *nodes: Node):
        """Stop tracking the nodes in this session."""
        for n in nodes:
            n._untrack_self(self)

    #
    # Transaction
    #

    def create(self, *nodes: Node):
        """Creates a new node. Errors if the node already exists."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.create(n)

    def upsert(self, *nodes: Node):
        """Creates or updates a node. Any non-id properties will be overwritten."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.upsert(n)

    def update(self, *nodes: Node, properties: tuple[Property, ...]):
        """Updates an existing node. Cannot move. The given properties are overwritten."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.update(n, properties)

    def move(self, *nodes: Node):
        """Moves and updates an existing node."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.move(n)

    def delete(self, *nodes: Node):
        """Deletes a node with the option to recover it for a limited time."""
        assert self._tx is not None, f"no transaction in {self!r}"
        for n in nodes:
            self._tx.soft_delete(n)

    def restore(self, *nodes: Node):
        """Restore a soft deleted node."""
        assert self._tx is not None, f"no transaction in {self!r}"
        for n in nodes:
            self._tx.restore(n)

    def archive(self, *nodes: Node):
        """Marks a node as archived, so it will be hidden by default."""
        assert self._tx is not None, f"no transaction in {self!r}"
        for n in nodes:
            self._check_not_active(n)
            self._tx.archive(n)

    def unarchive(self, *nodes: Node):
        """Re-track a node from the archive in its original place."""
        assert self._tx is not None, f"no transaction in {self!r}"
        for n in nodes:
            self._tx.unarchive(n)

    def hard_delete_forever(self, *nodes: Node):
        """Irreversibly deletes a node."""
        assert self._tx is not None, f"no transaction in {self!r}"
        for n in nodes:
            self._check_not_active(n)
            self._tx.delete(n)

    def _check_not_active(self, n: Node):
        """Checks if the node or any of its ancestors are active."""
        if n.ck in self._active_nodes_by_ck:
            block = self._active_nodes_by_ck[n.ck]
            raise RuntimeError(f"cannot delete ancestor {n!r} of running block: {block!r}")

    def _records_changed(self, database: "Block", record_ids: Collection[UUID]):
        self._tx._records_changed(database, record_ids)

    #
    # Stack: runs/logs
    #

    @property
    def stacktrace(self):
        return self._stacktrace

    @property
    def current_run(self) -> Optional[Run]:
        if self._stacktrace:
            return self._stacktrace[-1]
        return None

    async def _flush_logs(self) -> None:
        """Flushes pending logs to OS."""

        from bench.search.engine import pack_struct
        from bench.proto import wiring

        with self._runtime_tracing_lock:
            logs = self._pending_logs
            self._pending_logs = []
        if not logs:
            return
        logger.debug("session.write_logs", logs=len(logs))
        ops: list[dict] = []
        os_name = self.package.os_name
        logs = [wiring.pack_struct(log) for log in logs]
        for log in logs:
            ops.append({"index": {"_index": os_name, "_id": str(log.id)}})
            ops.append(pack_struct(log))
        ret = await os_client.bulk(ops)
        if ret["errors"]:
            raise RuntimeError(f"failed to write logs: {get_os_errors(ret)}")
        await self._host.notify_logs(logs)
        logger.debug("session.write_logs.done", logs=len(logs))

    def _track_run(self, run: Run):
        # replace if already exists by id (runs are updated)
        self._runs_by_id[run.id] = run
        self._pending_runs_by_id[run.id] = run

    def _track_log(self, log: LogEntry):
        self._pending_logs.append(log)
        self._cached_logs.append(log)

    def _pop_stacktrace(self) -> Run:
        run = self._stacktrace.pop()
        self._update_stacktrace_ancestors()
        # update cached info in parent(s)
        if run.value.cached_at is not None:
            self._update_cached_info()
        return run

    def _update_stacktrace_ancestors(self):
        """Maintains the stacktrace ancestors cache (using the current traced stacktrace)."""
        self._active_nodes_by_ck.clear()
        for run in self._stacktrace:
            parent = run.block
            while parent is not None and parent.ck not in self._active_nodes_by_ck:
                self._active_nodes_by_ck[parent.ck] = run.block
                parent = parent.parent

    def _run_enter(self, block: "Block", inputs):
        # we set invalid values to none here unlike in other packing places because
        #  these values may be written even if invalid
        from bench.language.value import check_type, pack_value

        assert not self.session.closed_at, f"cannot run {block!r} in session {self.session!r}"

        run = self._create_run(
            block=block,
            inputs=pack_value(inputs, block, is_output=False, none_if_invalid=True),
        )
        with self._runtime_tracing_lock:
            self._stacktrace.append(run)
            self._update_stacktrace_ancestors()
            _set_active_run(run)
            self._track_run(run)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.run.enter", run=run, stackdepth=len(self._stacktrace))

        # pre-run validation
        try:
            if len(self.stacktrace) >= MAX_STACK_DEPTH:
                raise RecursionError(f"maximum stack depth exceeded: {MAX_STACK_DEPTH}")
            check_type(inputs, block, is_output=False)
        except BaseException as e:
            self._run_exception(block, e)
            raise e

    def _run_exit(self, block: "Block", outputs):
        from bench.language.value import check_type

        assert not self.session.closed_at, f"cannot run {block!r} in session {self.session!r}"

        # post-run validation
        try:
            check_type(outputs, block, is_output=True)
        except BaseException as e:
            self._run_exception(block, e)
            raise e

        with self._runtime_tracing_lock:
            run = self._pop_stacktrace()
            assert run.block == block, f"bad stack in {self!r}: {run!r} got {block!r}"
            run.terminated_at = utcnow_with_tz()
            run.outputs_packed = _pack_and_truncate_value(
                outputs, block, is_output=True, none_if_invalid=True
            )
            run.status = RunStatus.COMPLETED
            self._track_run(run)
            _clear_active_run(run)
        logger.debug("trace.run.exit", run=run, stackdepth=len(self.stacktrace))

    def _run_exception(self, block: "Block", exception: BaseException):
        assert not self.session.closed_at, f"cannot run {block!r} in session {self.session!r}"
        with self._runtime_tracing_lock:
            run = self._pop_stacktrace()
            assert run.block == block, f"bad stack in {self!r}: {run!r} got {block!r}"
            run.terminated_at = utcnow_with_tz()
            run.error = RunError.from_exception(exception, block)
            if isinstance(exception, asyncio.CancelledError):
                run.status = RunStatus.ABORTED
            else:
                run.status = RunStatus.FAILED
            self._track_run(run)
            _clear_active_run(run)
        logger.debug("trace.run.exception", run=run, stackdepth=len(self.stacktrace))

    def _run_cached(
        self,
        block: "Block",
        inputs,
        outputs,
        generated_at: datetime,
        generated_in: UUID,
        duration: float,
    ):
        assert not self.session.closed_at, f"cannot run {block!r} in session {self.session!r}"
        run = self._create_run(block=block, trace=True)
        run.terminated_at = utcnow_with_tz()
        run.inputs_packed = _pack_and_truncate_value(
            inputs, block, is_output=False, none_if_invalid=True
        )
        run.outputs_packed = _pack_and_truncate_value(
            outputs, block, is_output=True, none_if_invalid=True
        )
        run.status = RunStatus.COMPLETED
        run.value.cached_at = generated_at
        run.value.cached_in = generated_in
        run.value.cached_duration = duration
        custom_value = _custom_value.get()
        for k, v in (custom_value or {}).items():
            run.value[k] = v
        with self._runtime_tracing_lock:
            self._track_run(run)
            self._update_cached_info()
        logger.debug("trace.run.cached", run=run, stackdepth=len(self.stacktrace))

    def _update_cached_info(self):
        for run in self.stacktrace:
            run.value.cached_at = min(
                r.value.cached_at for r in run.walk_descendants() if r.value.cached_at
            )
            run.value.cached_duration = sum(
                r.value.cached_duration for r in run.walk_descendants() if r.value.cached_duration
            )

    def _create_run(
        self,
        block: Optional["Block"] = None,
        inputs: dict[str, Any] | None = None,
        queue_position: int | None = None,
        trace: bool = True,
        trigger_type: TriggerType | None = None,
        trigger: Union["Trigger", UUID, None] = None,
    ):
        active_run = _get_active_run()
        if trace and active_run is not None:
            root = active_run.root or active_run
            parent = active_run
        else:
            root = None
            parent = self.session
        if root:
            trigger_type = trigger_type or TriggerType.INVOKE
        if not trigger_type and root is None:
            # inherit trigger type from session if we're not nested
            # (this will be wrong once we process other triggers within a session)
            trigger_type = self.session.trigger_type
            trigger = self.session.trigger_id
        run_ck = self._root_run_ck if root is None else uuid4()
        run = Run(
            ck=run_ck,
            id=get_node_id(self.package.id, run_ck),
            bench_id=self.session.package.bench_id,
            server=self.session.server_id,
            server_process_id=self.session.server_process_id,
            block=block,
            trigger_type=trigger_type,
            trigger_id=trigger.id if not isinstance(trigger, UUID) else trigger,
            started_at=utcnow_with_tz(),
            inputs=inputs,
            status=RunStatus.QUEUED if queue_position is not None else RunStatus.RUNNING,
            value=(self._root_run_value or {}) if root is None else None,
            _track=NodeTrackingLevel.NONE,
            _session=UNSET,  # ensure run isn't validated/tracked in active session
        )
        run._session = None  # reset to None so it can be tracked
        # we track session nodes manually :ManualSessionTracking
        parent.runs.append(run)
        custom_value = _custom_value.get()
        if root is None and self._root_run_value:
            run.value.update(self._root_run_value)
        if custom_value:
            run.value.update(custom_value)
        if self._init_run_value:
            run.value.update(self._init_run_value)
        return run


#
# Log collection
# (will obviously move out soon)
# TODO @Performance!: revamp contextual stdout/stderr capture

stderr_track: ContextVar[Callable[[str], None] | None] = ContextVar("stderr_track", default=None)
stdout_track: ContextVar[Callable[[str], None] | None] = ContextVar("stdout_track", default=None)


class _ContextRedirectedStream:
    """Redirect stdout/stderr for dual-writing to context-specific track functions."""

    def __init__(self, native, contextvar: ContextVar[Callable[[str], None]]):
        self.native = native
        self.contextvar = contextvar
        self._just_saw_newline = False

    def write(self, data: str) -> int:
        ret = self.native.write(data)
        track = self.contextvar.get()
        if track and (data != "\n" or self._just_saw_newline):
            # TODO @Robustness: figure out better way of collecting stdout/stderr
            #  This is very hacky because we don't know who called print and want to skip
            #  some of our own log messages. Unfortunately we can't just trivially
            #  provide a custom 'print' since many libraries use the real 'print' internally (?)
            if not ("[debug" in data or "[info" in data):
                track(data)
        self._just_saw_newline = data == "\n"
        return ret

    def flush(self) -> None:
        self.native.flush()


def _redirect_std_streams_if_needed():
    """Redirect stdout/stderr to the current context's track functions if they are set."""
    if not isinstance(sys.stdout, _ContextRedirectedStream):
        sys.stdout = _ContextRedirectedStream(sys.stdout, stdout_track)
    if not isinstance(sys.stderr, _ContextRedirectedStream):
        sys.stderr = _ContextRedirectedStream(sys.stderr, stderr_track)


class LogCollector:
    def __init__(self, track: Callable[[LogEntry], None], stream: str, session: "Session"):
        self.track = track
        self.session = session
        self.stream = stream
        self.package_id = session.package.id

    def _track(self, message: str) -> None:
        active_run = _get_active_run()
        if active_run:
            block = active_run.block
            run = active_run
        else:
            block = None
            run = None
        package = self.session.package
        log_entry = LogEntry(
            id=uuid4(),
            bench_id=package.bench_id,
            package=package,
            created_at=utcnow_with_tz(),
            stream=self.stream,
            session=self.session,
            block=block,
            run=run,
            message=message,
        )
        self.track(log_entry)

    def start(self):
        _redirect_std_streams_if_needed()
        if self.stream == "stderr":
            stderr_track.set(self._track)
        elif self.stream == "stdout":
            stdout_track.set(self._track)
        else:
            raise ValueError(f"invalid stream: {self.stream}")

    def stop(self):
        if self.stream == "stderr":
            stderr_track.set(None)
        elif self.stream == "stdout":
            stdout_track.set(None)
        else:
            raise ValueError(f"invalid stream: {self.stream}")


LOG_CACHE_SIZE = 1000
MAX_STACK_DEPTH = 8 if IS_DEBUG else 16

# We track the active root in a contextvar but not children
#  because they may be in different contexts, and we cannot reset across contexts.
# This will need to be expanded when we get to parallel runs.
_active_root_run: ContextVar[Run | None] = ContextVar("active_root_run", default=None)
_active_run_by_root: dict[UUID, Run] = {}
_custom_value: ContextVar[dict[str, Any] | None] = ContextVar("custom_value", default=None)


def _get_active_run() -> Run | None:
    root = _active_root_run.get()
    if root is not None:
        return _active_run_by_root[root.id]
    return None


def _clear_active_run(run: Run):
    root = run.root or run
    if root.id in _active_run_by_root:
        if run.parent is None:
            del _active_run_by_root[root.id]
        else:
            _active_run_by_root[root.id] = run.parent
    if _active_root_run.get() == run:
        _active_root_run.set(None)


def _set_active_run(run: Run):
    root = run.root or run
    _active_run_by_root[root.id] = run
    if _active_root_run.get() is None:
        _active_root_run.set(root)


def _pack_and_truncate_value(
    value: Any,
    type: "Block",
    ignore_array: bool = False,
    ignore_outer: bool = False,
    none_if_invalid: bool = False,
    is_output: bool = None,
) -> Any:
    from bench.language.value import map_value, pack_value_flat

    def _truncate_value(value: Any, type: "TypeInfo", *args, **kwargs) -> Any:
        if type.primitive_type == PrimitiveType.VECTOR:
            if type.is_array:
                return []
            else:
                return None
        else:
            return value

    return map_value(
        value=value,
        type=type,
        map_k=lambda f: (f.py_ident, f.identity_key),
        map_v=pack_value_flat,
        premap_v=_truncate_value,
        ignore_array=ignore_array,
        ignore_outer=ignore_outer,
        none_if_invalid=none_if_invalid,
        is_output=is_output,
    )
