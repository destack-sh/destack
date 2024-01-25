import asyncio
import sys
import threading
from collections import defaultdict, deque
from concurrent.futures import ThreadPoolExecutor
from contextvars import ContextVar
from datetime import datetime
from itertools import chain
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Collection,
    NamedTuple,
    Optional,
    Union,
    cast,
)
from uuid import UUID, uuid4

import psycopg
import structlog

from bench.language.builtin import _active_session
from bench.language.const import (
    NTL,
    EditKind,
    NodeTrackingLevel,
    NodeType,
    RunStatus,
    StructType,
    TriggerType,
)
from bench.language.node import (
    _NC,
    UNSET,
    Module,
    Node,
    ScopeNode,
    Struct,
    get_node_id,
    node,
    node_parent,
    struct,
    struct_internal,
    struct_runtime,
)
from bench.language.run import Run, RunError
from bench.language.value import HasValue
from bench.os.client import get_os_errors, os_client
from bench.proto.wire import EditData, ModuleHostStub
from bench.sql.client import get_pg_connection_pool
from bench.sql.core import ColumnType
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import _auto_async_to_sync
from bench.utils.utils import DEBUG
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Policy, Statement, Trigger, Worker
    from bench.language.cache import Cache

logger = structlog.get_logger(__name__)


@struct(StructType.LOG_ENTRY, index_in_os=True)
class LogEntry(Struct):
    """An entry. In a log."""

    id: UUID = struct_internal(2, default_factory=uuid4)
    module: Module = struct_internal(5, require=True, array=False, references=NodeType.MODULE)
    bench: Module = struct_internal(6, require=True, array=False, references=NodeType.BENCH)
    created_at: datetime = struct_internal(32, default_factory=utcnow_with_tz)
    stream: str = struct_internal(33)
    session: "Session" = struct_internal(34, require=False, array=True, references=NodeType.SESSION)
    level: Optional[str] = struct_internal(35, default=None)
    logger: Optional[str] = struct_internal(36, default=None)
    statement: Optional["Statement"] = struct_internal(
        37, require=False, array=False, references=NodeType.STATEMENT
    )
    run: Optional["Run"] = struct_internal(38, require=False, array=False, references=NodeType.RUN)
    message: Optional[str] = struct_internal(39, default=None)
    value: dict[str, Any] | None = struct_internal(
        40,
        require=False,
        default=None,
        column_type=ColumnType.JSON,
        ignore_conflicts_with=(HasValue,),
    )

    def __str__(self):
        return f"'{self.message}' ({self.created_at})"

    def __repr__(self):
        return f"<LogEntry {self}>"


_Edit = NamedTuple(
    "_Edit",
    [
        ("kind", EditKind),
        ("node", Node),
        ("properties", list[str] | None),
    ],
)
_CombinedEdits = NamedTuple(
    "_Edits",
    [
        ("global_edits", list[EditData] | None),
        ("session_edits", list[EditData] | None),
        ("local_edits", list[EditData] | None),
    ],
)

_executor: ThreadPoolExecutor | None = ThreadPoolExecutor(max_workers=1)


@node(NodeType.SESSION, local=True, index_in_os=True)
class Session(ScopeNode):
    """
    A managed context for running a Bench module (in a worker).
    """

    parent: Module = node_parent(4, NodeType.MODULE)
    policies: list["Policy"] | None = struct_internal(30, default=None, struct_t=StructType.POLICY)
    worker: Optional["Worker"] = struct_internal(
        31, require=False, array=False, references=NodeType.WORKER
    )
    worker_process_id: Optional[str] = struct_internal(32, default=None)
    trigger_type: Optional[TriggerType] = struct_internal(33, default=None)
    trigger_id: Optional[UUID] = struct_internal(34, default=None)
    opened_at: Optional[datetime] = struct_internal(35, default=None)
    closed_at: Optional[datetime] = struct_internal(36, default=None)

    _host: ModuleHostStub | None = struct_runtime(default=None)
    _root_run_ck: UUID | None = struct_runtime(default=None)
    _root_run_value: dict | None = struct_runtime(default=None)
    _init_run_value: dict | None = struct_runtime(default=None)
    _cache: Union["Cache", None] = struct_runtime(default=None)
    _log: structlog.BoundLogger = struct_runtime(default=None)

    _failed_commit: bool = struct_runtime(default=False)
    _dangling_nodes_by_ck: dict[UUID, Node] = struct_runtime(default_factory=dict)
    _global_pg_cursor: psycopg.AsyncCursor | None = struct_runtime(default=None)
    _local_pg_cursor: psycopg.AsyncCursor | None = struct_runtime(default=None)
    _other_local_pg_cursors: dict[str, psycopg.AsyncCursor] = struct_runtime(default_factory=dict)
    _tracing_lock: threading.Lock = struct_runtime(default_factory=threading.Lock)

    _created_nodes_ck: set[UUID] = struct_runtime(default_factory=set)
    _updated_nodes_event_by_ck: dict[UUID, int] = struct_runtime(default_factory=dict)
    _local_edits: list[_Edit] = struct_runtime(default_factory=list)
    _global_edits: list[_Edit] = struct_runtime(default_factory=list)
    _changed_record_ids_by_db_id: dict[UUID, set[UUID]] = struct_runtime(
        default_factory=lambda: defaultdict(set)
    )
    _touched_databases_by_id: dict[UUID, "Statement"] = struct_runtime(default_factory=dict)
    _schema_changed: bool = struct_runtime(default=False)

    _cached_logs: deque[LogEntry] | None = struct_runtime(default=None)
    _pending_logs: list[LogEntry] | None = struct_runtime(default=None)
    _runs_by_id: dict[UUID, Run] | None = struct_runtime(default=None)
    _pending_runs_by_id: dict[UUID, Run] | None = struct_runtime(default=None)
    _flush_session_loop: asyncio.Task | None = struct_runtime(default=None)
    _stdout_collector: Optional["LogCollector"] = struct_runtime(default=None)
    _stderr_collector: Optional["LogCollector"] = struct_runtime(default=None)
    _stacktrace: list[Run] | None = struct_runtime(default_factory=list)
    _active_nodes_by_ck: dict[UUID, Node] | None = struct_runtime(default=None)

    def __content_str__(self):
        if self.closed_at:
            status = "closed"
        elif self.opened_at:
            status = "open"
        else:
            status = "pending"
        return (
            f" ({status}, "
            f"{len(self._local_edits)} local edits, "
            f"{len(self._global_edits)} global edits, "
            f"{len(self._runs_by_id) if self._runs_by_id is not None else 0} runs"
            f")"
        )

    @property
    def host(self) -> ModuleHostStub:
        assert self._host is not None, f"host not available in {self!r}"
        return self._host

    @property
    def dangling(self) -> tuple[Node, ...]:
        return tuple(n for n in self._dangling_nodes_by_ck.values() if not n.parent)

    def dangling_like(self, type_: type[Node]) -> tuple[Node, ...]:
        return tuple(n for n in self.dangling if isinstance(n, type_))

    @property
    def global_pg_cursor(self) -> psycopg.AsyncCursor:
        assert self._global_pg_cursor is not None, "global_pg_cursor is unavailable"
        return self._global_pg_cursor

    @property
    def local_pg_cursor(self) -> psycopg.AsyncCursor:
        assert self._local_pg_cursor is not None, "local_pg_cursor is unavailable"
        return self._local_pg_cursor

    async def pg_cursor_to_local(self, module: Module) -> psycopg.AsyncCursor:
        if module == self.module:
            return self.local_pg_cursor
        if module.pg_name not in self._other_local_pg_cursors:
            logger.debug("session.open_foreign_pg", module=module)
            pg_pool = await get_pg_connection_pool(module.pg_name)
            pg_connection = await pg_pool.getconn(timeout=3)
            self._other_local_pg_cursors[module.pg_name] = pg_connection.cursor()
        return self._other_local_pg_cursors[module.pg_name]

    async def open(self, session_flush_interval: float = 0.1):
        """Opens the session for regular business."""
        if self.opened_at is not None:
            raise RuntimeError(f"session already opened {self}")
        if _active_session.get() is not None:
            raise RuntimeError(f"another session is active: {_active_session.get()}")
        _active_session.set(self)
        self._log.debug("session.open")

        # flush loop
        assert session_flush_interval > 0.05, f"flush_interval {session_flush_interval} < 0.05s"

        async def _flush_session_loop():
            while True:
                await self.flush_session()
                await self.flush_logs()
                await asyncio.sleep(session_flush_interval)

        self._flush_session_loop = asyncio.create_task(_flush_session_loop())

        # prepare session
        self.opened_at = utcnow_with_tz()
        self._cached_logs = deque(maxlen=LOG_CACHE_SIZE)
        self._pending_logs = []
        self._runs_by_id = {}
        self._pending_runs_by_id = {}
        self._active_nodes_by_ck = {}
        self._stacktrace = []
        self._active_nodes_by_ck = {}
        self._cache = Cache(self.module)
        self._log = logger.bind(session=self)
        self._stdout_collector = LogCollector(self._track_log, "stdout", self)
        self._stderr_collector = LogCollector(self._track_log, "stderr", self)
        self._stdout_collector.start()
        self._stderr_collector.start()

        # prepare local postgres
        if self._local_pg_cursor is None:
            pg_pool = await get_pg_connection_pool(self.module.pg_name)
            pg_connection = await pg_pool.getconn(timeout=2)
            self._local_pg_cursor = pg_connection.cursor()

        self._log.debug("session.open.done")

    @_auto_async_to_sync
    async def close(self):
        """Closes the session *without committing*. Prevent further runs & (tracked) edits."""
        if self.closed_at is not None:
            raise RuntimeError(f"session already closed {self}")
        self._log.debug("session.close")

        # close postgres connections
        if self._local_pg_cursor:
            await self.local_pg_cursor.connection.rollback()  # any DB operation starts a tx in psycopg
            pg_pool = await get_pg_connection_pool(self.module.pg_name)
            await pg_pool.putconn(self._local_pg_cursor.connection)
            self._local_pg_cursor = None
        for pg_name, pg_cursor in self._other_local_pg_cursors.items():
            await pg_cursor.connection.rollback()
            pg_pool = await get_pg_connection_pool(pg_name)
            await pg_pool.putconn(pg_cursor.connection)

        # close session
        self.closed_at = utcnow_with_tz()
        _active_session.set(None)
        self._stdout_collector.stop()
        self._stderr_collector.stop()

        self._flush_session_loop.cancel()
        if self.dangling:
            self._log.warn("session.close.dangling", dangling=self.dangling)
        self._log.debug("session.close.done")

    @property
    def has_regular_edits(self) -> bool:
        """Whether this session has any non-session edits."""
        return (
            len(self._local_edits) > 0
            or len(self._global_edits) > 0
            or len(self._changed_record_ids_by_db_id) > 0
        )

    @_auto_async_to_sync
    async def flush_local(self):
        """Flushes local Postgres edits."""
        from bench.sql.engine import pg_write_record_edits, update_dynamic_local_pg_schema

        # if the schema changed, also flush PG schema
        if self._schema_changed:
            await update_dynamic_local_pg_schema(self.module.pg_name, self.module)
            self._schema_changed = False

        edits = self._eat_edits(local=True)
        await pg_write_record_edits(self.local_pg_cursor, self.module, edits.local_edits)

    @_auto_async_to_sync
    async def flush_session(self, force: bool = False, kill_pending_runs: bool = False) -> None:
        """Flushes session edits."""

        edits = self._eat_edits(session=True, kill_pending_runs=kill_pending_runs)
        await self.session.host.push_edits(edits.session_edits)

    @_auto_async_to_sync
    async def flush_logs(self) -> None:
        """Flushes session logs. This is non-transactional, so it's separate from flush_session."""
        from bench.os.engine import pack_struct
        from bench.proto import wiring

        with self._tracing_lock:
            logs = self._pending_logs
            self._pending_logs = []
        if not logs:
            return
        self._log.debug("session.write_logs", logs=len(logs))
        ops: list[dict] = []
        os_name = self.module.os_name
        logs = [wiring.pack_struct(log) for log in logs]
        for log in logs:
            ops.append({"index": {"_index": os_name, "_id": str(log.id)}})
            ops.append(pack_struct(log))
        ret = await os_client.bulk(ops)
        if ret["errors"]:
            raise RuntimeError(f"failed to write logs: {get_os_errors(ret)}")
        await self._host.push_logs(logs)
        self._log.debug("session.write_logs.done", logs=len(logs))

    @_auto_async_to_sync
    async def commit(self):
        """Commits module edits and syncs committed local edits to OS."""
        from bench.os.engine import sync_pg_databases_to_os
        from bench.sql.engine import pg_write_record_edits

        assert not self._failed_commit, f"session {self!r} is broken after failed commit"

        # prepare
        touched_databases_by_id = {**self._touched_databases_by_id}
        edits = self._eat_edits(global_=True, local=True, session=True)
        log = self._log.bind(
            global_edits=edits.global_edits,
            local_edits=len(edits.local_edits),
            session_edits=edits.session_edits,
            local_edits_preview=edits.local_edits[:16],
        )
        log.debug("session.commit")

        # commit
        try:
            if edits.global_edits:
                await self._host.commit_edits(edits.global_edits)
            if edits.local_edits or edits.session_edits:
                await pg_write_record_edits(
                    cur=self.local_pg_cursor,
                    module=self.module,
                    edits=[*(edits.local_edits or ()), *(edits.session_edits or ())],
                    old_databases_by_id=touched_databases_by_id,
                )
            await self._local_pg_cursor.connection.commit()
            self.module._apply_edits_to_source(edits.global_edits)
            log.debug("session.commit.done")
        except Exception as e:
            # 'unwind' module state, mark session as broken
            log.exception("session.commit.failed", exc_info=True)
            self.module._reset_from_source()
            self._failed_commit = True
            # nocheckin: put session commit error in session/outermost run/???
            raise RuntimeError(
                f"failed to commit edits ({len(edits.global_edits)} global, {len(edits.local_edits)} local, {len(edits.session_edits)} session): {e}"
            ) from e

        # sync to os & push edits to already applied local records
        if self._changed_record_ids_by_db_id:
            # TODO @Robustness: repair index in case of local PG/OS sync failures
            # sync local edits to index
            log.debug("session.commit.index")
            changed_records: list[tuple["Statement", set[UUID]]] = [
                (self._touched_databases_by_id[db_id], record_ids)
                for db_id, record_ids in self._changed_record_ids_by_db_id.items()
            ]
            await sync_pg_databases_to_os(self.module, self.local_pg_cursor, changed_records)
            self._changed_record_ids_by_db_id.clear()
            self._touched_databases_by_id.clear()

            # publish local edits
            await self._host.notify_databases_changed(touched_databases_by_id.values())

    @_auto_async_to_sync
    async def rollback(self):
        raise NotImplementedError  # unclear what this should do

    #
    # Module
    #

    def create(self, node: Node):
        """Creates a new node. Errors if the node already exists."""
        self._edit(EditKind.CREATE, node=node)

    def create_many(self, *nodes: Node):
        for node in nodes:
            self._edit(EditKind.CREATE, node=node)

    def upsert(self, node: Node):
        """Creates or updates a node. Any non-id properties will be overwritten."""
        self._edit(EditKind.UPSERT, node=node)

    def upsert_many(self, *nodes: Node):
        for node in nodes:
            self._edit(EditKind.UPSERT, node=node)

    def update(self, node: Node, properties: list[str]):
        """Updates an existing node. Cannot move. The given properties are overwritten."""
        if node.ck not in self._created_nodes_ck:
            self._edit(EditKind.UPDATE, node=node, properties=properties)

    def update_many(self, *nodes: Node, properties: list[str]):
        for node in nodes:
            if node.ck not in self._created_nodes_ck:
                self._edit(EditKind.UPDATE, node=node, properties=properties)

    def move(self, node: Node, properties: list[str] = None):
        """Moves and updates an existing node. Can update any properties."""
        self._edit(EditKind.MOVE, node=node, properties=properties)

    def move_many(self, *nodes: Node, properties: list[str] = None):
        for node in nodes:
            self._edit(EditKind.MOVE, node=node, properties=properties)

    def soft_delete(self, node: Node):
        """Deletes a node with the option to recover it for a limited time."""
        self._check_not_active(node)
        self._edit(EditKind.SOFT_DELETE, node=node)

    def soft_delete_many(self, *nodes: Node):
        for node in nodes:
            self._check_not_active(node)
            self._edit(EditKind.SOFT_DELETE, node=node)

    def restore(self, node: Node):
        """Restore a soft deleted node."""
        self._edit(EditKind.RESTORE, node=node)

    def restore_many(self, *nodes: Node):
        for node in nodes:
            self._edit(EditKind.RESTORE, node=node)

    def archive(self, node: Node):
        """Marks a node as archived, so it will be hidden by default."""
        self._check_not_active(node)
        self._edit(EditKind.ARCHIVE, node=node)

    def archive_many(self, *nodes: Node):
        for node in nodes:
            self._check_not_active(node)
            self._edit(EditKind.ARCHIVE, node=node)

    def unarchive(self, node: Node):
        """Re-activate a node from the archive in its original place."""
        self._edit(EditKind.UNARCHIVE, node=node)

    def unarchive_many(self, *nodes: Node):
        for node in nodes:
            self._edit(EditKind.UNARCHIVE, node=node)

    def delete(self, node: Node):
        """Irreversibly deletes a node."""
        self._check_not_active(node)
        self._edit(EditKind.DELETE, node=node)

    def delete_many(self, *nodes: Node):
        for node in nodes:
            self._check_not_active(node)
            self._edit(EditKind.DELETE, node=node)

    def _check_not_active(self, node: Node):
        """Checks if the node or any of its ancestors are active."""
        if node.ck in self._active_nodes_by_ck:
            statement = self._active_nodes_by_ck[node.ck]
            raise RuntimeError(
                f"cannot delete ancestor {node!r} of running statement: {statement!r}"
            )

    def _records_changed(self, database: "Statement", record_ids: Collection[UUID]):
        self._touched_databases_by_id[database.id] = database
        self._changed_record_ids_by_db_id[database.id].update(record_ids)

    def _edit(self, kind: EditKind, node: Node, properties: list[str] = None):
        """Register a non-session edit event to a node (local or global)."""
        assert self.closed_at is None, f"cannot {kind.name} {node!r} in closed session {self!r}"
        if not node._track & NTL.FULL:
            return
        if node.metatype == NodeType.FIELD:
            self._schema_changed = True
        edit = _Edit(kind=kind, node=node, properties=properties)
        assert not self.session.closed_at, f"cannot {edit!r} in closed session {self.session!r}"
        edits = self._local_edits if node.__is_local__ else self._global_edits

        if kind == EditKind.CREATE:
            self._created_nodes_ck.add(edit.node.ck)
        elif kind == EditKind.UPDATE:
            # merge with previous update if there is one
            update_idx = self._updated_nodes_event_by_ck.get(edit.node.ck)
            if update_idx is not None:
                for prop in edit.properties:
                    if prop not in edits[update_idx].properties:
                        edits[update_idx].properties.append(prop)
                return  # merged, ignore this edit
            else:  # remember update event index
                self._updated_nodes_event_by_ck[edit.node.ck] = len(edits)
        edits.append(edit)

        if node.metatype == NodeType.RECORD:
            self._changed_record_ids_by_db_id[edit.node.parent_id].add(edit.node.id)
            self._touched_databases_by_id[edit.node.parent_id] = node.parent

        if edit.node.ck in self.session._dangling_nodes_by_ck:
            del self.session._dangling_nodes_by_ck[edit.node.ck]

    def _eat_edits(
        self,
        *,
        local: bool = False,
        session: bool = False,
        global_: bool = False,
        kill_pending_runs: bool = False,
    ) -> _CombinedEdits:
        """
        Converts all edit into proper edits.
        Global edits = any module edits that aren't local.
        Local edits = any record or not-in-session session edits.
        Session edits = any runs/sessions that happened in this session.
        """
        from bench.language.database import Record
        from bench.proto.wiring import pack_node, wrap_some_node

        with self._tracing_lock:
            # create local edits
            local_edits: list[EditData] = []
            local_seen_cks: set[UUID] = set()

            if local:
                for event in self._local_edits:
                    node = cast(Record, event.node)
                    edit = EditData(
                        kind=event.kind,
                        node=wrap_some_node(pack_node(node)),
                        properties=event.properties,
                    )
                    if not global_:  # not needed if including everything
                        local_seen_cks.add(node.ck)
                    local_edits.append(edit)
                self._local_edits.clear()

            # create global edits (if needed)
            global_edits: list[EditData] | None = [] if global_ else None
            if global_:
                for event in self._global_edits:
                    edit = EditData(
                        kind=event.kind,
                        node=wrap_some_node(pack_node(event.node)),
                        properties=event.properties,
                    )
                    global_edits.append(edit)
                self._global_edits.clear()

            # reset regular edits
            if global_:  # just clear all
                self._created_nodes_ck.clear()
                self._updated_nodes_event_by_ck.clear()
            else:  # clear only local seen
                self._created_nodes_ck.difference_update(local_seen_cks)
                self._updated_nodes_event_by_ck = {
                    ck: idx
                    for ck, idx in self._updated_nodes_event_by_ck.items()
                    if ck not in local_seen_cks
                }

            if session:
                runs = list(self._pending_runs_by_id.values())
                self._pending_runs_by_id.clear()
                if kill_pending_runs:
                    # abort any remaining active runs
                    for run in chain(runs, self._runs_by_id.values()):
                        run._mark_dead_if_active()
                session_edits: list[EditData] | None = []
                for n in chain((self.session,), runs):
                    edit = EditData(kind=EditKind.UPSERT, node=wrap_some_node(pack_node(n)))
                    session_edits.append(edit)
            else:
                session_edits = None

        return _CombinedEdits(
            global_edits=global_edits,
            session_edits=session_edits,
            local_edits=local_edits,
        )

    #
    # Session
    #

    @property
    def stacktrace(self):
        return self._stacktrace

    @property
    def current_run(self) -> Optional[Run]:
        if self._stacktrace:
            return self._stacktrace[-1]
        return None

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
            parent = run.node
            while parent is not None and parent.ck not in self._active_nodes_by_ck:
                self._active_nodes_by_ck[parent.ck] = run.node
                parent = parent.parent

    def _run_enter(self, statement: "Statement", inputs):
        # we set invalid values to none here unlike in other packing places because
        #  these values may be written even if invalid
        from bench.language.value import check_type, pack_value

        assert not self.session.closed_at, f"cannot run {statement!r} in session {self.session!r}"

        run = self._create_run(
            statement=statement,
            inputs=pack_value(inputs, statement, is_output=False, none_if_invalid=True),
        )
        with self._tracing_lock:
            self._stacktrace.append(run)
            self._update_stacktrace_ancestors()
            _set_active_run(run)
            self._track_run(run)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.run.enter", run=run, stackdepth=len(self._stacktrace))

        # pre-run validation
        try:
            if len(self.stacktrace) >= MAX_STACK_DEPTH:
                raise RecursionError(f"maximum stack depth exceeded: {MAX_STACK_DEPTH}")
            check_type(inputs, statement, is_output=False)
        except BaseException as e:
            self._run_exception(statement, e)
            raise e

    def _run_exit(self, statement: "Statement", outputs):
        from bench.language.value import check_type

        assert not self.session.closed_at, f"cannot run {statement!r} in session {self.session!r}"

        # post-run validation
        try:
            check_type(outputs, statement, is_output=True)
        except BaseException as e:
            self._run_exception(statement, e)
            raise e

        with self._tracing_lock:
            run = self._pop_stacktrace()
            assert run.node == statement, f"bad stack in {self!r}: {run!r} got {statement!r}"
            run.terminated_at = utcnow_with_tz()
            run.outputs = _pack_and_truncate_value(
                outputs, statement, is_output=True, none_if_invalid=True
            )
            run.status = RunStatus.COMPLETED
            self._track_run(run)
            _clear_active_run(run)
        logger.debug("trace.run.exit", run=run, stackdepth=len(self.stacktrace))

    def _run_exception(self, statement: "Statement", exception: BaseException):
        assert not self.session.closed_at, f"cannot run {statement!r} in session {self.session!r}"
        with self._tracing_lock:
            run = self._pop_stacktrace()
            assert run.node == statement, f"bad stack in {self!r}: {run!r} got {statement!r}"
            run.terminated_at = utcnow_with_tz()
            run.error = RunError.from_exception(exception, statement)
            if isinstance(exception, asyncio.CancelledError):
                run.status = RunStatus.ABORTED
            else:
                run.status = RunStatus.FAILED
            self._track_run(run)
            _clear_active_run(run)
        logger.debug("trace.run.exception", run=run, stackdepth=len(self.stacktrace))

    def _run_cached(
        self,
        statement: "Statement",
        inputs,
        outputs,
        generated_at: datetime,
        generated_in: UUID,
        duration: float,
    ):
        assert not self.session.closed_at, f"cannot run {statement!r} in session {self.session!r}"
        run = self._create_run(statement=statement, trace=True)
        run.terminated_at = utcnow_with_tz()
        run.inputs = _pack_and_truncate_value(
            inputs, statement, is_output=False, none_if_invalid=True
        )
        run.outputs = _pack_and_truncate_value(
            outputs, statement, is_output=True, none_if_invalid=True
        )
        run.status = RunStatus.COMPLETED
        run.value.cached_at = generated_at
        run.value.cached_in = generated_in
        run.value.cached_duration = duration
        custom_value = _custom_value.get()
        for k, v in (custom_value or {}).items():
            run.value[k] = v
        with self._tracing_lock:
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
        statement: Optional["Statement"] = None,
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
        run_ck = self._root_run_ck if root is None else UUIDT()
        run = Run(
            ck=run_ck,
            id=get_node_id(self.module.id, run_ck),
            bench_id=self.session.module.bench_id,
            worker=self.session.worker_id,
            worker_process_id=self.session.worker_process_id,
            statement=statement,
            trigger_type=trigger_type,
            trigger_id=trigger.id if not isinstance(trigger, UUID) else trigger,
            started_at=utcnow_with_tz(),
            inputs=inputs,
            status=RunStatus.QUEUED if queue_position is not None else RunStatus.RUNNING,
            value=(self._root_run_value or {}) if root is None else {},
            _track=NodeTrackingLevel.NONE,
            _session=UNSET,  # ensure run isn't validated/tracked in session
        )
        run._session = None  # reset to None so it can be activated
        # we track session nodes manually :ManualSessionTracking
        parent.runs.append(run, _trigger=_NC.Ignore, _create=False)
        custom_value = _custom_value.get()
        if root is None and self._root_run_value:
            run.value.update(self._root_run_value)
        if custom_value:
            run.value.update(custom_value)
        if self._init_run_value:
            run.value.update(self._init_run_value)
        return run


# TODO @Performance: improve performance of contextual stdout/stderr capture

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
        self.module_id = session.module.id

    def _track(self, message: str) -> None:
        active_run = _get_active_run()
        if active_run:
            statement = active_run.node
            run = active_run
        else:
            statement = None
            run = None
        module = self.session.module
        log_entry = LogEntry(
            id=UUIDT(),
            bench_id=module.bench_id,
            module=module,
            created_at=utcnow_with_tz(),
            stream=self.stream,
            session=self.session,
            statement=statement,
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
MAX_STACK_DEPTH = 8 if DEBUG else 16

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
    type: "Statement",
    ignore_array: bool = False,
    ignore_outer: bool = False,
    none_if_invalid: bool = False,
    is_output: bool = None,
) -> Any:
    from bench.language.value import map_value, pack_value_flat

    def _is_type_truncated(type: "HasFields") -> bool:
        return type.tag in (TypeTag.VECTOR,)

    def _truncate_value(value: Any, type: "HasFields", *args, **kwargs) -> Any:
        if _is_type_truncated(type):
            if type.flags & TypeFlag.IS_ARRAYABLE or type.flags & TypeFlag.IS_ARRAY:
                return []
            return None
        return value

    return map_value(
        value=value,
        type=type,
        map_k=lambda f: (f.py_ident, f.metatyped_key),
        map_v=pack_value_flat,
        premap_v=_truncate_value,
        ignore_array=ignore_array,
        ignore_outer=ignore_outer,
        none_if_invalid=none_if_invalid,
        is_output=is_output,
    )
