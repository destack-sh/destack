import asyncio
import dataclasses
import sys
import threading
import traceback
from collections import defaultdict, deque
from concurrent.futures import ThreadPoolExecutor
from contextvars import ContextVar
from dataclasses import dataclass
from datetime import datetime
from typing import TYPE_CHECKING, Any, Callable, Collection, Optional, Union
from uuid import UUID, uuid4

import structlog
from asgiref.sync import async_to_sync, sync_to_async

from bench.language.const import (
    EMPTY_SCOPE,
    TERMINAL_RUN_STATUSES,
    BenchError,
    EditType,
    NodeStatus,
    NodeTrackingLevel,
    NodeType,
    RunErrorKind,
    RunStatus,
    StoreEngineType,
    StructType,
    TriggerType,
    _active_session,
)
from bench.language.field import TypeInfo
from bench.language.graph import NodeList
from bench.language.node import (
    UNSET,
    Node,
    Property,
    Struct,
    _Passthrough,
    get_node_id,
    node,
    node_component,
    p_ancestor,
    p_child,
    p_internal,
    p_parent,
    p_runtime,
    p_secret_value_packed,
    p_system,
    p_value_packed,
    p_value_runtime,
    struct,
)
from bench.language.query import StoreConnection, StoreEngine
from bench.language.value import HasValues
from bench.opensearch.client import get_os_errors, os_client
from bench.proto.wire import BenchHostStub, EditData, GraphScope, SupervisorStub
from bench.sql.core import PrimitiveType
from bench.utils.dt import utcnow_with_tz
from bench.utils.env import IS_DEBUG
from bench.utils.func import _auto_async_to_sync, uuid_to_str

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        BenchError,
        Block,
        Branch,
        Environment,
        Node,
        Package,
        PrimitiveType,
        Server,
        Struct,
        Trigger,
    )

logger = structlog.get_logger(__name__)


@node(
    NodeType.SIGNAL, passthrough=(("value", _Passthrough.Full),), index_in_search=True, local=True
)
class Signal(HasValues):
    """A signal received in this Bench. May be emitted by a Bench or an external source."""

    parent: "Package" = p_parent(4, NodeType.PACKAGE)
    type: Optional["Block"] = p_internal(
        30, require=False, array=False, references=NodeType.BLOCK, index_in_pg=True
    )
    object: Optional["Block"] = p_internal(
        31, require=False, array=False, references=NodeType.BLOCK, index_in_pg=True
    )
    sender: Optional["Block"] = p_internal(
        32, require=False, array=False, references=NodeType.BLOCK, index_in_pg=True
    )
    value_packed: Any | None = p_value_packed(33)
    secret_value_packed: Any | None = p_secret_value_packed(34)
    value = p_value_runtime(33, 34, type=30)


@struct(StructType.LOG_ENTRY, index_in_search=True)
class LogEntry(Struct):
    """An entry. In a log."""

    id: UUID = p_internal(2, default_factory=uuid4)
    package: "Package" = p_internal(5, require=True, array=False, references=NodeType.PACKAGE)
    bench: "Package" = p_internal(6, require=True, array=False, references=NodeType.BENCH)
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
    bench: Optional["Bench"] = p_internal(30, require=False, array=False, references=NodeType.BENCH)
    environment: Optional["Environment"] = p_internal(
        31, require=False, array=False, references=NodeType.ENVIRONMENT
    )
    branch: Optional["Branch"] = p_internal(
        32, require=False, array=False, references=NodeType.BRANCH
    )
    package: Optional["Package"] = p_internal(
        33, require=False, array=False, references=NodeType.PACKAGE
    )
    module: Optional["Block"] = p_internal(
        34, require=False, array=False, references=NodeType.BLOCK
    )
    page: Optional["Block"] = p_internal(35, require=False, array=False, references=NodeType.BLOCK)
    # log: ...

    # custom
    # value_packed: Any = p_value_packed(40)
    # secret_value_packed: Any = p_secret_value_packed(41)
    # value: Any = p_value_runtime(40, 41)


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
    session: "Session" = dcfield(default=None)
    is_read_only: bool = dcfield(default=False)
    _connections_by_engine_id: dict[Any, StoreConnection | None] = dcfield(default_factory=dict)

    edits: list[EditData] = dcfield(default_factory=list)
    _pending_edits_by_engine_id: dict[Any, list[EditData]] = dcfield(
        default_factory=lambda: defaultdict(list)
    )
    _pending_updates_idx: dict[Node, tuple[Any, int]] = dcfield(default_factory=dict)

    # for syncing databases (should probably generalize into' untracked edits')
    _schema_changed: bool = dcfield(default=False)

    @property
    def has_edits(self) -> bool:
        return len(self.edits) > 0

    @property
    def has_pending_edits(self) -> bool:
        return any(self._pending_edits_by_engine_id.values())

    @staticmethod
    def from_existing(edits: Collection[EditData]):
        tx = Transaction()
        for edit in edits:  # replay edits
            tx._add_pending_edit(edit)
        return tx

    async def connect_store_to(self, base: Node | None, node_type: NodeType) -> StoreConnection:
        root = base.root if base is not None else None
        if root is not None:
            scope = GraphScope(
                bench_id=uuid_to_str(root.bench_id), package_id=uuid_to_str(root.package_id)
            )
        else:
            scope = EMPTY_SCOPE
        for engine in self.session._engines:
            if engine.supports(scope, node_type):
                return await self._get_engine_connection(engine)
        raise BenchError(f"no engine for [base={base!r}, node={node_type}] in {self.session!r}")

    async def _get_engine_connection(self, engine: StoreEngine) -> StoreConnection:
        connection = self._connections_by_engine_id.get(engine.id)
        if connection is None:
            connection = await engine.connect(self.session)
            self._connections_by_engine_id[engine.id] = connection
        return connection

    def _get_engine_for_edit(self, scope: GraphScope, node_type: NodeType) -> StoreEngine:
        for engine in self.session._engines:
            if engine.supports(scope, node_type):
                return engine
        raise BenchError(
            f"no engine for edit [scope={scope!r}, node_type={node_type}] in {self.session!r}"
        )

    #
    # Edits
    #

    def _make_edit(self, type: EditType, n: Node) -> EditData:
        """Creates an edit and adds it to the pending edits."""
        if self.is_read_only:
            raise RuntimeError(f"cannot {type.bench_name} {n!r} in read-only {self.session}")

        from bench.proto import wiring

        # TODO @Performance: pack only edited node properties
        node_data = n._to_data()
        if n._updated_properties:
            properties = n._unmask_properties_ids(n._updated_properties)
        else:
            properties = None
        if n.__is_in_bench__:
            scope = GraphScope(
                bench_id=uuid_to_str(n.bench.id) if n.bench is not None else None,
                package_id=uuid_to_str(n.package_id) if n.package is not None else None,
            )
        else:
            scope = EMPTY_SCOPE
        edit = EditData(
            type=wiring.pack_enum(EditType, type),
            node_type=node_data.metatype,
            node=wiring.wrap_some_node(node_data),
            properties=properties,
            scope=scope,
        )
        return edit

    def _add_pending_edit(self, edit: EditData) -> StoreEngine:
        if edit.node_type == NodeType.FIELD:
            self._schema_changed = True

        engine = self._get_engine_for_edit(edit.scope, edit.node_type)
        self.edits.append(edit)
        self._pending_edits_by_engine_id[engine.id].append(edit)
        return engine

    def create(self, n: Node):
        edit = self._make_edit(EditType.CREATE, n)
        self._add_pending_edit(edit)

    def upsert(self, n: Node):
        edit = self._make_edit(EditType.UPSERT, n)
        self._add_pending_edit(edit)

    def update(self, n: Node, properties: tuple[Property, ...]):
        from bench.proto import wiring

        edit = self._pending_updates_idx.get(n)
        if edit is None:
            # new update
            edit = self._make_edit(EditType.UPDATE, n)
            engine = self._add_pending_edit(edit)
            self._pending_updates_idx[n] = engine.id, len(self.edits) - 1
            return

        # update existing edit in place
        #  (to avoid re-packing everything for successive updates)
        engine_id, current_update_idx = edit
        edit = self._pending_edits_by_engine_id[engine_id][current_update_idx]
        edit.properties = n._unmask_properties_ids(n._updated_properties)
        node_data = wiring.unwrap_some_node(edit.node)
        for prop in properties:
            if prop.reference_wired_ptr:
                prop = prop.reference_wired_ptr
            value = getattr(n, prop.name)
            value = wiring._pack_struct_prop(prop, value, ignore_array=False)
            setattr(node_data, prop.name, value)

    def move(self, n: Node):
        edit = self._make_edit(EditType.MOVE, n)
        self._add_pending_edit(edit)

    def soft_delete(self, n: Node):
        edit = self._make_edit(EditType.SOFT_DELETE, n)
        self._add_pending_edit(edit)

    def restore(self, n: Node):
        edit = self._make_edit(EditType.RESTORE, n)
        self._add_pending_edit(edit)

    def archive(self, n: Node):
        edit = self._make_edit(EditType.ARCHIVE, n)
        self._add_pending_edit(edit)

    def unarchive(self, n: Node):
        edit = self._make_edit(EditType.UNARCHIVE, n)
        self._add_pending_edit(edit)

    def delete(self, n: Node):
        edit = self._make_edit(EditType.DELETE, n)
        self._add_pending_edit(edit)

    def _records_changed(self, database: "Block", record_ids: Collection[UUID]):
        self._changed_record_ids_by_base_id[database.id].update(record_ids)

    #
    # Transaction management
    #

    async def open(self):
        pass

    async def flush(self, *only_engine_types: StoreEngineType):
        """
        Flushes any pending edits to the primary stores (without committing).
        If specific engines are given, only flushes to those engines.
        """

        if only_engine_types:
            engines = [e for e in self.session._engines if e.type in only_engine_types]
        else:
            engines = self.session._engines
        log = logger.bind(edits=len(self.edits), engines=len(engines), transaction=self)
        for engine in engines:
            pending_edits = self._pending_edits_by_engine_id.get(engine.id, ())
            if pending_edits:
                connection = await self._get_engine_connection(engine)
                log.debug("transaction.flush", engine=engine, flushed=len(pending_edits))
                await connection.flush(pending_edits)
                pending_edits.clear()

    async def commit(self):
        """Commits the transaction (flushing any pending edits). Syncs to secondary stores."""
        log = logger.bind(edits=len(self.edits), transaction=self)

        # TODO @Robustness!: use 2PC in Transaction.commit
        #  (if there are more than 2 engines to commit to)
        for engine in self.session._engines:
            pending_edits = self._pending_edits_by_engine_id.get(engine.id, ())
            if pending_edits or engine.id in self._connections_by_engine_id:
                connection = await self._get_engine_connection(engine)
                log.debug("transaction.commit", engine=engine, flushed=len(pending_edits))
                await connection.commit(pending_edits)
                pending_edits.clear()

    async def rollback(self):
        """Rolls back uncommitted edits in primary stores."""
        raise NotImplementedError("not yet supported")

    async def close(self):
        """Closes the transaction and associated store engines, rolling back uncommitted edits."""
        for connection in self._connections_by_engine_id.values():
            if connection is not None:
                await connection.close()


_executor: ThreadPoolExecutor | None = ThreadPoolExecutor(max_workers=1)
_runtime_tracing_lock: threading.Lock = threading.Lock()


@node(NodeType.SESSION, index_in_search=True, local=True)
class Session(Node):
    """
    A managed session for interacting with Bench nodes and (if on a Server) running them.
    """

    parent: "Package" = p_parent(4, NodeType.PACKAGE, is_system=True)
    server: Optional["Server"] = p_system(
        31, require=False, array=False, references=NodeType.SERVER
    )
    opened_at: Optional[datetime] = p_system(32, default=None)
    closed_at: Optional[datetime] = p_system(33, default=None)
    is_runtime: bool = p_system(34, default=False)
    is_read_only: bool = p_system(35, default=False)

    # transaction
    _tx: Transaction | None = p_runtime(default=None)
    _engines: tuple["StoreEngine", ...] = p_runtime(default_factory=tuple)
    _supervisor: Optional["SupervisorStub"] = p_runtime(default=None)
    _host: Optional["BenchHostStub"] = p_runtime(default=None)
    _dangling_nodes_by_ck: dict[UUID, Node] = p_runtime(default_factory=dict)

    # runtime
    _stacktrace: list["Run"] | None = p_runtime(default=None)
    _runs_by_id: dict[UUID, "Run"] | None = p_runtime(default=None)
    _pending_runs_by_id: dict[UUID, "Run"] | None = p_runtime(default=None)
    _active_nodes_by_ck: dict[UUID, Node] | None = p_runtime(default=None)

    # logs
    _cached_logs: deque[LogEntry] | None = p_runtime(default=None)
    _pending_logs: list[LogEntry] | None = p_runtime(default=None)
    _flush_session_loop: asyncio.Task | None = p_runtime(default=None)
    _stdout_collector: Optional["LogCollector"] = p_runtime(default=None)
    _stderr_collector: Optional["LogCollector"] = p_runtime(default=None)

    def __content_str__(self):
        if self.closed_at:
            status_str = "closed"
        elif self.opened_at:
            status_str = "open"
        else:
            status_str = "pending"
        return (
            f"{status_str}, "
            f"{self._tx or '<no tx>'}, "
            f"{len(self._runs_by_id) if self._runs_by_id is not None else 0} runs"
        )

    def _init_inner(self) -> None:
        self._session = self

    @property
    def dangling_nodes(self) -> tuple[Node, ...]:
        return tuple(n for n in self._dangling_nodes_by_ck.values() if not n.parent)

    @property
    def tx(self) -> Transaction:
        assert self._tx is not None, f"no active transaction in {self!r}"
        return self._tx

    @property
    def has_edits(self) -> bool:
        """Whether this session has any non-session edits."""
        return self._tx is not None and self._tx.has_edits

    @property
    def has_pending_edits(self):
        """Whether this session has any pending (unflushed) edits."""
        return self._tx is not None and self._tx.has_pending_edits

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
        self._tx = Transaction(session=self, is_read_only=self.is_read_only)

        self.opened_at = utcnow_with_tz()
        logger.debug("session.open.done")

    @_auto_async_to_sync
    async def flush(self):
        assert self.is_open, f"cannot flush {self!r} when closed"
        await self._tx.flush()

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

        if self.dangling_nodes:
            logger.warn("session.close.dangling", dangling=self.dangling_nodes)

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()

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

    async def _flush_logs(self) -> None:
        """Flushes pending logs to OS."""

        from bench.opensearch.engine import pack_struct
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

    def _track_run(self, run: "Run"):
        # replace if already exists by id (runs are updated)
        self._runs_by_id[run.id] = run
        self._pending_runs_by_id[run.id] = run

    def _track_log(self, log: LogEntry):
        self._pending_logs.append(log)
        self._cached_logs.append(log)

    def _pop_stacktrace(self) -> "Run":
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


# We track the active root in a contextvar but not children
#  because they may be in different contexts, and we cannot reset across contexts.
# This will need to be expanded when we get to parallel runs.
@node(NodeType.RUN, index_in_search=True, local=True)
class Run(HasValues):
    """
    A 'run' of a block (in a session).
    """

    # NOTE we don't 'track' runs in sessions yet
    #  (because we don't edit them outside of the source session,
    #   and because it's unclear how run/session edits should interact with 'regular' package edits)

    parent: Union["Session", "Run"] = p_parent(4, NodeType.SESSION, NodeType.RUN)
    session: "Session" = p_ancestor(
        30, NodeType.SESSION, require=True, store=True, wire=True, index_in_pg=True
    )
    root: Optional["Run"] = p_ancestor(
        31,
        NodeType.RUN,
        require=False,
        nearest=False,
        include_self=False,
        store=True,
        wire=True,
        index_in_pg=True,
    )
    server: Optional["Server"] = p_internal(
        32, index_in_pg=True, require=False, array=False, references=NodeType.SERVER
    )
    block: Optional["Block"] = p_internal(
        33, references=NodeType.BLOCK, require=False, array=False, index_in_pg=True
    )
    # block_path?
    scheduled_at: Optional[datetime] = p_internal(35, default=None)
    started_at: Optional[datetime] = p_internal(36, default=None)
    terminated_at: Optional[datetime] = p_internal(37, default=None)
    duration: float = p_internal(38, default=0)
    status: RunStatus = p_internal(39, index_in_pg=True)

    inputs_packed: Any = p_value_packed(50)
    inputs_secret_packed: Any = p_secret_value_packed(51)
    inputs: Any = p_value_runtime(50, 51)
    outputs_packed: Any = p_value_packed(52)
    outputs_secret_packed: Any = p_secret_value_packed(53)
    outputs: Any = p_value_runtime(52, 53)
    value_packed: Any = p_value_packed(54)
    value_secret_packed: Any = p_secret_value_packed(55)
    value: Any = p_value_runtime(54, 55)
    error: Optional["RunError"] = p_internal(56, default=None, primitive_type=PrimitiveType.JSON)

    runs: list["Run"] = p_child(NodeType.RUN)

    def __content_str__(self):
        value_keys_str = ", ".join(self.value.keys()) if self.value else ""
        return f"{self.block} ({self.status}, value={value_keys_str or '<none>'}, {self.id})"

    @property
    def active(self) -> bool:
        return self.status not in TERMINAL_RUN_STATUSES


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


@node_component
class HasRun(Node):
    """A runnable block"""

    triggers: NodeList["Trigger"] = p_child(NodeType.TRIGGER)

    @property
    def _is_async(self) -> Optional[bool]:  # set in supporting components e.g. HasCode
        """Whether this block is async."""
        return None

    def _call_inner(self, *args, **kwargs):
        assert (
            self.is_attached and self._status == NodeStatus.TRACKED
        ), f"cannot call {self!r} (status={self._status!r})"
        try:
            asyncio.get_running_loop()
            is_outer_async = True
        except RuntimeError:
            is_outer_async = False
        inner_call = self._call_inner_async if self._is_async else self._call_inner_sync

        if is_outer_async and not self._is_async:
            inner_call = sync_to_async(inner_call)
        elif not is_outer_async and self._is_async:
            inner_call = async_to_sync(inner_call)

        return inner_call(*args, **kwargs)

    def _call_inner_sync(self, *args, **kwargs):
        raise NotImplementedError

    async def _call_inner_async(self, *args, **kwargs):
        raise NotImplementedError


@struct(StructType.RUN_CODE_FRAME)
class RunCodeFrame(Struct):
    node: Node = p_internal(30, array=False, require=True, references=NodeType.BLOCK)
    lineno: int = p_internal(31)
    name: str = p_internal(32)
    locals: Optional[dict[str, Any]] = p_internal(
        33, default=None, primitive_type=PrimitiveType.JSON
    )
    line: str = p_internal(34)

    @staticmethod
    def clean(
        stack: list["RunCodeFrame"], from_block: "Block", session: "Session"
    ) -> list["RunCodeFrame"]:
        from bench.language.block import Block
        from bench.language.code_ import HasCode

        code_by_method: dict[str, HasCode] = {
            node._transform.method_name: node
            for node in list(session.package._nodes)
            if isinstance(node, Block) and getattr(node, "_transform", None)
        }
        if getattr(from_block, "_transform", None):
            # from block may not be in package (e.g. if detached when running anonymous code)
            code_by_method[from_block._transform.method_name] = from_block

        found_start = False
        cleaned_stack = []
        for frame in stack:
            if not found_start:
                # impute bench source info into instantiated code callables
                code = code_by_method.get(frame.name)
                if code is not None:
                    if code == from_block:
                        found_start = True
                    elif not found_start:
                        continue  # ignore
                    frame.node = from_block
                    frame.name = from_block.name or "<unnamed>"
                    frame.lineno = frame.lineno - code._transform.start_offset
                    frame.line = code.code.splitlines()[frame.lineno - 1]
                    frame.locals = frame.locals or {}
                    for ident, var in code._block_references.items():
                        if ident not in frame.locals and var.id in session.package._graph:
                            frame.locals[ident] = repr(session.package._graph[var.id])
            if found_start:
                # trim file path for python packages
                python_version = f"{sys.version_info.major}.{sys.version_info.minor}"
                if python_version in frame.node:
                    frame.node = frame.node.split(python_version)[-1][1:]  # skip slash
                cleaned_stack.append(frame)
        return [f for f in cleaned_stack if f.line]


@struct(StructType.RUN_ERROR)
class RunError(Struct, BenchError):
    kind: RunErrorKind = p_internal(30)
    type: str = p_internal(31)
    message: Optional[str] = p_internal(32, default=None)
    node: Optional["Node"] = p_internal(33, require=False, array=False, references=NodeType.BLOCK)
    traceback: list[RunCodeFrame] = p_internal(
        34, default_factory=list, struct=StructType.RUN_CODE_FRAME
    )

    @staticmethod
    def from_exception(e: BaseException, block: Optional["Block"]) -> "RunError":
        if isinstance(e, RunError):
            return e
        stack = RunCodeFrame.from_stack(traceback.extract_tb(e.__traceback__))
        stack = RunCodeFrame.clean(stack, block, block.session)
        if isinstance(e, SyntaxError):  # ignore (..., line x) because it's not useful
            err_str = e.msg
        else:
            err_str = str(e)
        return RunError(
            kind=RunErrorKind.RUNTIME,
            type=type(e).__name__,
            message=err_str,
            block=block,
            traceback=stack,
        )


@node(NodeType.PAUSE, local=True)
class Pause(Node):
    """A resumable interruption in the execution (Run) of a block."""

    parent: "Run" = p_parent(4, NodeType.RUN)
    session: "Session" = p_ancestor(30, NodeType.SESSION, require=True, store=True)
    # (placeholder)


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
