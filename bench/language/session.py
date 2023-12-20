import abc
import asyncio
import contextlib
import sys
import threading
from collections import defaultdict, deque
from concurrent.futures import ThreadPoolExecutor
from contextvars import ContextVar
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from typing import (
    TYPE_CHECKING,
    Any,
    Awaitable,
    Callable,
    Collection,
    Coroutine,
    Optional,
    Union,
    cast,
)
from uuid import UUID, uuid4

import asgiref.sync
import psycopg
import structlog

from bench.language.builtin import _active_session, _auto_async_to_sync, symbolx_lib
from bench.language.const import (
    INTERP_NODE_TYPES,
    NTL,
    NodeTrackingLevel,
    NodeType,
    ProjectRegion,
    RunStatus,
    SessionAccessLevel,
    StructType,
    TriggerType,
    TypeFlag,
    TypeTag,
    WorkerProfile,
    WorkerSetStatus,
)
from bench.language.edit import EditData, EditKind, EditType
from bench.language.module import (
    _NC,
    UNSET,
    Module,
    Node,
    NodeList,
    NRel,
    ScopeNode,
    Struct,
    node,
    node_children,
    struct,
    struct_internal,
    struct_runtime,
)
from bench.language.run import Run, RunError
from bench.language.statement import Statement
from bench.language.value import HasValue
from bench.search.client import get_os_errors, os_client
from bench.sql.client import get_pg_connection_pool
from bench.sql.core import ColumnType
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import DEBUG
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Blob, HasDatabase, HasFields, Secret, Trigger
    from bench.language.cache import Cache
    from bench.language.wire import LogEntryData, RunData

logger = structlog.get_logger(__name__)


class RuntimeHost(abc.ABC):
    """Central Bench runtime server for synchronizing modules and sessions."""

    @property
    def session_lock(self) -> asyncio.Lock:
        raise NotImplementedError

    async def commit_edits(self, edits: Collection["EditData"]) -> None:
        raise NotImplementedError

    async def notify_runs_changed(self, runs: Collection["RunData"]) -> None:
        raise NotImplementedError

    async def notify_logs_changed(self, logs: Collection["LogEntryData"]) -> None:
        raise NotImplementedError

    async def notify_databases_changed(self, databases: Collection["HasDatabase"]) -> None:
        raise NotImplementedError

    async def download_blob(self, blob: "Blob") -> str:
        raise NotImplementedError

    async def prepare_upload_blob(self, blob: "Blob") -> tuple["Blob", Optional[str]]:
        raise NotImplementedError

    async def mark_uploaded_blob(self, blob: "Blob") -> None:
        raise NotImplementedError

    async def reveal_secret(self, secret: "Secret") -> Any:
        raise NotImplementedError

    async def run_proxy_statement(self, statement: Statement, inputs: dict) -> dict:
        raise NotImplementedError

    async def run_proxy_inference(self, statement: Statement, inputs: dict, timeout: float) -> dict:
        raise NotImplementedError


@struct(StructType.LOG_ENTRY)
class LogEntry(Struct):
    """
    An entry. In a log.
    """

    id: UUID = struct_internal(2, default_factory=uuid4)
    module: Module = struct_internal(21, references=NodeType.MODULE)
    created_at: datetime = struct_internal(22, default_factory=utcnow_with_tz)
    stream: str = struct_internal(23)
    session: "Session" = struct_internal(24, references=NodeType.SESSION)
    level: Optional[str] = struct_internal(25, default=None)
    logger: Optional[str] = struct_internal(26, default=None)
    statement: Optional["Statement"] = struct_internal(27, references=NodeType.STATEMENT)
    run: Optional["Run"] = struct_internal(28, references=NodeType.RUN)
    message: Optional[str] = struct_internal(29, default=None)
    value: dict[str, Any] | None = struct_internal(
        30,
        is_required=False,
        default=None,
        store_as=ColumnType.JSON,
        ignore_conflicts_with=(HasValue,),
    )

    def __str__(self):
        return f"'{self.message}' ({self.created_at})"

    def __repr__(self):
        return f"<LogEntry {self}>"


@struct(StructType.WORKER_SET)
class WorkerSet(Struct):
    """
    Set of workers to run a Bench's modules.
    """

    id: UUID = struct_internal(2, default_factory=uuid4)
    created_at: datetime = struct_internal(10, default_factory=utcnow_with_tz)
    updated_at: datetime = struct_internal(11, default_factory=utcnow_with_tz)
    project_id: UUID = struct_internal(20)
    region: ProjectRegion = struct_internal(21)
    profile: WorkerProfile = struct_internal(22)
    sleeping: bool = struct_internal(23)
    status: WorkerSetStatus = struct_internal(24)
    desired_replicas: int = struct_internal(25)
    target_replicas: int = struct_internal(26)
    available_replicas: int = struct_internal(27)
    ready_replicas: int = struct_internal(28)
    last_active_at: datetime = struct_internal(29, default_factory=utcnow_with_tz)


@struct(StructType.ENVIRONMENT)
class Environment(Struct):
    language: str = struct_internal(20)
    version: str = struct_internal(21)
    platform: str = struct_internal(22)
    packages: dict[str, str] = struct_internal(23, store_as=ColumnType.JSON)


@node(NodeType.SESSION, detached=True)
class Session(ScopeNode):
    """
    A managed context for running a Bench module (in a worker).
    """

    access_level: SessionAccessLevel = struct_internal(20)
    worker_node_id: str = struct_internal(21, reflect=True)
    worker_process_id: Optional[str] = struct_internal(22, reflect=True)
    trigger_type: TriggerType = struct_internal(23, reflect=True)
    trigger_id: Optional[UUID] = struct_internal(24, default=None, reflect=True)
    opened_at: Optional[datetime] = struct_internal(25, default=None, reflect=True)
    closed_at: Optional[datetime] = struct_internal(26, default=None, reflect=True)
    inference_timeout: int = struct_internal(27, default=300)
    inference_retries: int = struct_internal(28, default=5)
    runs: NodeList[Run] = node_children(NodeType.RUN, flags=NRel.Flat)
    _runtime: RuntimeHost | None = struct_runtime(default=None)
    _root_run_id: UUID | None = struct_runtime(default=None)
    _root_run_value: dict | None = struct_runtime(default=None)
    _global_run_value: dict | None = struct_runtime(default=None)
    _cache: Union["Cache", None] = struct_runtime(default=None)
    _failed_commit: bool = struct_runtime(default=False)

    def _init_inner(self):
        from bench.language.blob import Blobs
        from bench.language.cache import Cache

        self._session = self  # special case for session
        self._cache = Cache(self.module)
        self._blobs = Blobs(self.module)
        self._executor = ThreadPoolExecutor(max_workers=1)
        self._log = logger.bind(session=self)
        self._tracer = SessionTracer(
            self,
            root_run_id=self._root_run_id,
            root_run_value=self._root_run_value,
            global_run_value=self._global_run_value,
        )
        self._primary_pg_cursor: psycopg.AsyncCursor | None = None
        self._foreign_pg_cursors: dict[str, psycopg.AsyncCursor] = {}
        self._dangling_nodes_by_ck: dict[UUID, Node] = {}

    def __str__(self):
        if self.closed_at:
            status = "closed"
        elif self.opened_at:
            status = "open"
        else:
            status = "not opened"
        return (
            f"{self.module.name} ({self.access_level.name}, {status}, "
            f"{len(self._tracer._local_edits)} local edits, {len(self._tracer._host_module_edits)} host edits"
            f")"
        )

    def __repr__(self):
        return f"<Session {self}>"

    @property
    def path(self):
        return f"<session:{self.id}>"

    def sync_to_async(self, fn: Callable) -> Callable[..., Awaitable]:
        return asgiref.sync.sync_to_async(fn, thread_sensitive=False, executor=self._executor)  # type: ignore

    def async_to_sync(self, fn: Awaitable | Callable | Coroutine) -> Callable:
        return asgiref.sync.async_to_sync(fn)  # type: ignore

    @property
    def current_run(self) -> "Run":
        return self._tracer.current_run

    def capture_runs(self) -> "_RunCapture":
        return self._tracer.start_capture()

    def bind_run_value(self, **kwargs):
        return self._tracer.value(**kwargs)

    @contextlib.contextmanager
    def bind_access_level(self, access_level: SessionAccessLevel):
        if access_level > self.access_level:
            raise PermissionError(
                f"cannot increase access level from {self.access_level} to {access_level}"
            )
        old_access = self.access_level
        self.access_level = access_level
        try:
            yield
        finally:
            self.access_level = old_access

    def check_access(self, access_level: SessionAccessLevel):
        if access_level > self.access_level:
            access_level_name = SessionAccessLevel(access_level).name.lower()
            raise PermissionError(f"cannot {access_level_name} in {self!r}")

    @property
    def dangling(self) -> list[Node]:
        return [n for n in self._dangling_nodes_by_ck.values() if not n.parent]

    def dangling_like(self, type: type[Node]) -> list[Node]:
        return [n for n in self.dangling if isinstance(n, type)]

    @property
    def is_open(self) -> bool:
        return self.opened_at is not None and self.closed_at is None

    @property
    def pg_cursor(self) -> psycopg.AsyncCursor:
        assert (
            self._primary_pg_cursor is not None
        ), "pg_cursor is only available during session execution"
        return self._primary_pg_cursor

    async def pg_cursor_to(self, module: Module) -> psycopg.AsyncCursor:
        if module == self.module:
            return self.pg_cursor
        if module.pg_name not in self._foreign_pg_cursors:
            logger.debug("session.open_foreign_pg", module=module)
            pg_pool = get_pg_connection_pool(module.pg_name)
            pg_connection = await pg_pool.getconn(timeout=3)
            self._foreign_pg_cursors[module.pg_name] = pg_connection.cursor()
        return self._foreign_pg_cursors[module.pg_name]

    @property
    def _should_autocommit(self):
        # ensure edits are committed before we exit out of topmost run for error propagation
        return self._tracer.has_edits and len(self._tracer.stacktrace) == 1

    async def _open(self):
        """Opens the session for execution and modification."""
        if self.opened_at is not None:
            raise RuntimeError(f"session already opened {self}")

        # prepare session
        self.opened_at = utcnow_with_tz()
        if _active_session.get() is not None:
            raise RuntimeError(f"another session is active: {_active_session.get()}")
        _active_session.set(self)
        await self._tracer.open()

        # prepare local postgres
        pg_pool = get_pg_connection_pool(self.module.pg_name)
        pg_connection = await pg_pool.getconn(timeout=2)
        self._primary_pg_cursor = pg_connection.cursor()

        self._log.debug("session.open")

    @_auto_async_to_sync
    async def flush(self):
        """Flushes edits"""
        await self.flush_local()

    @_auto_async_to_sync
    async def flush_local(self):
        """Flushes local Postgres edits (leaves other edits pending)."""
        from bench.sql.engine import update_pg_schema, write_local_edits_to_pg

        # if the schema changed, also flush PG schema
        if self._tracer._schema_changed:
            await update_pg_schema(self.module.pg_name, self.module)
            self._tracer._schema_changed = False

        _, local_edits = self._tracer.eat_module_edits(include_host=False)
        await write_local_edits_to_pg(self.pg_cursor, self.module, local_edits)

    @_auto_async_to_sync
    async def commit(self):
        """Commits module edits and syncs committed local edits to OS."""
        from bench.search.engine import sync_pg_databases_to_os
        from bench.sql.engine import write_local_edits_to_pg

        assert not self._failed_commit, f"session {self!r} is broken after failed commit"

        if not self._tracer.has_edits:
            self._log.debug("session.commit.skip")
            return  # nothing to commit

        touched_databases_by_id = {**self._tracer._touched_databases_by_id}
        host_edits, local_edits = self._tracer.eat_module_edits(include_host=True)
        log = self._log.bind(
            host_edits=host_edits,
            local_edits_preview=local_edits[:16],
            local_edits_len=len(local_edits),
        )
        log.debug("session.commit")

        # commit
        try:
            # commit host edits
            if host_edits:
                await self._runtime.commit_edits(host_edits)
            # commit local edits
            if local_edits:
                await write_local_edits_to_pg(
                    cur=self.pg_cursor,
                    module=self.module,
                    edits=local_edits,
                    old_databases_by_id=touched_databases_by_id,
                )
            await self._primary_pg_cursor.connection.commit()
            self.module._apply_edits_to_source(host_edits)
            log.debug("session.commit.done")
        except Exception as e:
            # 'unwind' module state, mark session as broken
            log.exception("session.commit.failed", exc_info=True)
            self.module._reset_from_source()
            self._failed_commit = True
            raise RuntimeError(
                f"failed to write edits ({len(host_edits)} host, {len(local_edits)} local): {e}"
            ) from e

        # manually ensure locally changed records are synced & notified
        if self._tracer._changed_record_ids_by_db_id:
            # TODO @Robustness: repair index in case of local PG/OS sync failures
            # sync local edits to index
            log.debug("session.commit.index")
            changed_records: list[tuple["HasDatabase", set[UUID]]] = [
                (self._tracer._touched_databases_by_id[db_id], record_ids)
                for db_id, record_ids in self._tracer._changed_record_ids_by_db_id.items()
            ]
            await sync_pg_databases_to_os(self.module, self.pg_cursor, changed_records)
            self._tracer._changed_record_ids_by_db_id.clear()
            self._tracer._touched_databases_by_id.clear()

            # publish local edits
            await self._runtime.notify_databases_changed(touched_databases_by_id.values())

    async def _close(self):
        """Closes the session, committing any edits and preventing further execution/edit."""
        if self.closed_at is not None:
            raise RuntimeError(f"session already closed {self}")
        self._log.debug("session.close")

        # close postgres connections
        if self._primary_pg_cursor:
            await self.pg_cursor.connection.rollback()  # any DB operation starts a tx in psycopg
            pg_pool = get_pg_connection_pool(self.module.pg_name)
            await pg_pool.putconn(self._primary_pg_cursor.connection)
            self._primary_pg_cursor = None
        if self._foreign_pg_cursors:
            for pg_name, pg_cursor in self._foreign_pg_cursors.items():
                await pg_cursor.connection.rollback()
                pg_pool = get_pg_connection_pool(pg_name)
                await pg_pool.putconn(pg_cursor.connection)

        # close session
        self.closed_at = utcnow_with_tz()
        _active_session.set(None)
        await self._tracer.close()

        if self.dangling:
            self._log.warn("session.close.dangling", dangling=self.dangling)
        self._log.debug("session.close.done")

    async def _write_logs(self, logs: list[LogEntry]) -> None:
        from bench.language import wiring
        from bench.search import mirror

        if not logs:
            return
        self._log.debug("session.write_logs", logs=len(logs))
        ops: list[dict] = []
        os_name = self.module.os_name
        logs = [wiring.pack_struct(log) for log in logs]
        for log in logs:
            ops.append({"index": {"_index": os_name, "_id": str(log.id)}})
            ops.append(mirror.unpack_node_flat(self.module, log, None).to_dict())
        ret = await os_client.bulk(ops)
        if ret["errors"]:
            raise RuntimeError(f"failed to write logs: {get_os_errors(ret)}")
        await self._runtime.notify_logs_changed(logs)
        self._log.debug("session.write_logs.done", logs=len(logs))


@dataclass(frozen=True)
class EditEvent:
    """Tiny edit representation to capture every edit event. Later coalesce into real Edits."""

    type: EditType
    node: Node
    target: NodeType | None = None
    properties: list[str] | None = None
    file_id: UUID | None = None
    statement_id: UUID | None = None


class SessionTracer:
    def __init__(
        self,
        session: Session,
        root_run_id: UUID = None,
        root_run_value: dict = None,
        global_run_value: dict = None,
    ):
        self.session = session
        self._cached_logs: deque[LogEntry] = deque(maxlen=LOG_CACHE_SIZE)
        self._pending_logs: list[LogEntry] = []
        self._pending_runs: dict[UUID, Run] = {}
        self._flush_cancel: asyncio.Event | None = None
        self._flush_task: asyncio.Task | None = None
        self._root_run_id = root_run_id

        from bench.language.packer import unpack_value

        RunMetadata = symbolx_lib.resolve(".reflect.RunMetadata")
        self._root_run_value = unpack_value(
            root_run_value, RunMetadata, map_k=lambda f: (f.py_ident, f.py_ident)
        )
        self._global_run_value = unpack_value(
            global_run_value, RunMetadata, map_k=lambda f: (f.py_ident, f.py_ident)
        )

        self.stdout_collector = LogCollector(self._track_log, "stdout", session)
        self.stderr_collector = LogCollector(self._track_log, "stderr", session)
        self.session = session
        self.runs = {}
        self._stacktrace: list[Run] = []
        self._stacktrace_ancestors_cks: dict[UUID, Statement] = {}  # protect running statements
        self._tracing_lock = threading.Lock()
        self._created_nodes_ck: set[UUID] = set()
        self._updated_nodes_event_by_ck: dict[UUID, int] = {}
        self._changed_record_ids_by_db_id: dict[UUID, set[UUID]] = defaultdict(set)
        self._touched_databases_by_id: dict[UUID, "HasDatabase"] = {}
        self._local_edits: list[EditEvent] = []
        self._host_module_edits: list[EditEvent] = []
        self._schema_changed: bool = False
        self._flushed_session_node_ids: set[UUID] = set()

    def __str__(self):
        return f"{len(self.stacktrace)} stack, {len(self.runs)} runs"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def stacktrace(self):
        return self._stacktrace

    @property
    def has_edits(self) -> bool:
        return (
            len(self._local_edits) > 0
            or len(self._host_module_edits) > 0
            or len(self._changed_record_ids_by_db_id) > 0
        )

    def _update_stacktrace_ancestors(self):
        """Maintains the stacktrace ancestors cache (using the current traced stacktrace)."""
        self._stacktrace_ancestors_cks.clear()
        for run in self._stacktrace:
            parent = run.statement
            while parent is not None and parent.ck not in self._stacktrace_ancestors_cks:
                self._stacktrace_ancestors_cks[parent.ck] = run.statement
                parent = parent.parent

    def _stacktrace_pop(self) -> Run:
        run = self._stacktrace.pop()
        self._update_stacktrace_ancestors()
        return run

    def _stacktrace_push(self, run: Run) -> None:
        self._stacktrace.append(run)
        self._update_stacktrace_ancestors()

    #
    # Module
    # Edits are actually written to local source in session commit.
    # We have the :InterpFilter because interp edits are tracked in the runtime host only.
    #
    # NOTE: We don't support restore/soft-delete/move in code yet.
    #

    def eat_module_edits(
        self, *, include_host: bool
    ) -> tuple[list[EditData] | None, list[EditData]]:
        """
        Converts the edit events into proper edits (for hosts and local)
        Host edits = any module edits not in a localized node (like records).
        Local edits = any record edits.
        """
        from bench.language import Record
        from bench.language.wiring import pack_node

        module = self.session.module
        with self._tracing_lock:
            # create local edits
            local_edits: list[EditData] = []
            local_seen_cks: set[UUID] = set()
            for event in self._local_edits:
                node = cast(Record, event.node)
                edit = EditData(
                    type=event.type,
                    project_version_id=module.id,
                    file_id=node.parent.file.id,
                    statement_id=node.parent.id,
                    properties=event.properties,
                )
                edit.node = pack_node(node)
                if not include_host:  # not needed if including everything
                    local_seen_cks.add(node.ck)
                local_edits.append(edit)
            self._local_edits.clear()

            # create host edits (if needed)
            host_edits: list[EditData] | None = [] if include_host else None
            if include_host:
                for event in self._host_module_edits:
                    node = event.node
                    edit = EditData(
                        type=event.type,
                        project_version_id=module.id,
                        file_id=event.file_id,
                        statement_id=event.statement_id,
                        properties=event.properties,
                    )
                    edit.node = pack_node(node)
                    host_edits.append(edit)
                self._host_module_edits.clear()

            # reset
            if include_host:  # just clear all
                self._created_nodes_ck.clear()
                self._updated_nodes_event_by_ck.clear()
            else:  # clear only local seen
                self._created_nodes_ck.difference_update(local_seen_cks)
                self._updated_nodes_event_by_ck = {
                    ck: idx
                    for ck, idx in self._updated_nodes_event_by_ck.items()
                    if ck not in local_seen_cks
                }

        return host_edits, local_edits

    def _edit(
        self, kind: EditKind, node: Node, properties: list[str] = None, target: NodeType = None
    ):
        """Register an edit to a node (local or host)."""
        is_local = node.metatype == NodeType.RECORD
        if not is_local:
            tree = self.session.module._local_tree
            file = tree.get_ancestor(node.ck, NodeType.FILE)
            statement = tree.get_ancestor(node.ck, NodeType.STATEMENT)
        else:
            file, statement = None, None
        edit = EditEvent(
            type=EditType.from_nt(kind, target or node.metatype),
            node=node,
            properties=properties,
            file_id=file.id if file else None,
            statement_id=statement.id if statement else None,
            target=target,
        )
        assert not self.session.closed_at, f"cannot {edit!r} in closed session {self.session!r}"
        edits = self._local_edits if is_local else self._host_module_edits

        if edit.type.kind == EditKind.CREATE:
            self._created_nodes_ck.add(edit.node.ck)
        elif edit.type.kind == EditKind.UPDATE:
            if edit.node.ck in self._created_nodes_ck:
                return  # ignore updates to newly created nodes
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

        if edit.type.node_type == NodeType.RECORD:
            self._changed_record_ids_by_db_id[edit.node.parent_id].add(edit.node.id)
            self._touched_databases_by_id[edit.node.parent_id] = edit.node.parent

        if edit.node.ck in self.session._dangling_nodes_by_ck:
            del self.session._dangling_nodes_by_ck[edit.node.ck]

    def _records_changed(self, database: "HasDatabase", record_ids: Collection[UUID]):
        self._touched_databases_by_id[database.id] = database
        self._changed_record_ids_by_db_id[database.id].update(record_ids)

    def node_create(self, *nodes: Node):
        # assumes you've called node_create_preflight first (to check permission)
        with self._tracing_lock:  # do it
            for n in nodes:
                if n.metatype == NodeType.FIELD or n.metatype == NodeType.RESOLVED_FIELD:
                    self._schema_changed = True
                if n.metatype in INTERP_NODE_TYPES or not (n._track & NTL.FULL):  # :InterpFilter
                    continue
                self._edit(EditKind.CREATE, node=n)

    def node_create_preflight(self, *nodes: Node):
        # used to check permission before modifying state locally
        # (only for create since this is the only edit fired 'after' making an irreversible change)
        if (
            any(n for n in nodes if n.metatype not in INTERP_NODE_TYPES and n._track & NTL.FULL)
            and self.session.access_level < SessionAccessLevel.Create
        ):
            raise PermissionError(f"{self.session!r} may not create {nodes!r}")

    def node_update(self, node: Node, properties: list[str]):
        if node.metatype == NodeType.FIELD or node.metatype == NodeType.RESOLVED_FIELD:
            self._schema_changed = True
        if node.metatype in INTERP_NODE_TYPES or not (node._track & NTL.FULL):  # :InterpFilter
            return
        if self.session.access_level < SessionAccessLevel.Update:
            raise PermissionError(f"{self.session!r} may not update {node!r}")

        with self._tracing_lock:  # do it
            if node.ck in self._created_nodes_ck:
                return  # ignore updates to newly created nodes
            self._edit(EditKind.UPDATE, node=node, properties=properties)

    def node_delete(self, *nodes: Node):
        if (
            any(n for n in nodes if n.metatype not in INTERP_NODE_TYPES and n._track & NTL.FULL)
            and self.session.access_level < SessionAccessLevel.Delete
        ):
            raise PermissionError(f"{self.session!r} may not delete {nodes!r}")
        # ensure node is not ancestor of any running statements
        if any(n.ck in self._stacktrace_ancestors_cks for n in nodes):
            ancestor = next(n for n in nodes if n.ck in self._stacktrace_ancestors_cks)
            statement = self._stacktrace_ancestors_cks[ancestor.ck]
            if ancestor == statement:
                raise RuntimeError(f"cannot delete running statement {statement!r}")
            else:
                raise RuntimeError(
                    f"cannot delete ancestor {ancestor!r} of running statement: {statement!r}"
                )

        with self._tracing_lock:  # do it
            for n in nodes:
                if n.metatype == NodeType.FIELD or n.metatype == NodeType.RESOLVED_FIELD:
                    self._schema_changed = True
                # :InterpFilter
                if n.metatype in INTERP_NODE_TYPES or not (n._track & NTL.FULL):  # :InterpFilter
                    continue
                self._edit(EditKind.DELETE, node=n)

    def node_truncate(self, node: Node, node_type: NodeType):
        if node.metatype in INTERP_NODE_TYPES or not (node._track & NTL.FULL):  # :InterpFilter
            return
        if self.session.access_level < SessionAccessLevel.Delete:
            raise PermissionError(f"{self.session!r} may not truncate {node!r}")
        if node_type == NodeType.RECORD:
            raise ValueError(f"cannot truncate records: {node!r}")
        self._edit(EditKind.TRUNCATE, node, target=node_type)

    #
    # Session
    #

    def _track_run(self, run: Run):
        # replace if already exists by id (runs are updated)
        self.runs[run.id] = run
        self._pending_runs[run.id] = run

    @property
    def current_run(self) -> Optional[Run]:
        if self._stacktrace:
            return self._stacktrace[-1]
        return None

    @property
    def cached_logs(self) -> list[LogEntry]:
        return list(self._cached_logs)

    @property
    def pending_logs(self) -> list[LogEntry]:
        return self._pending_logs

    def _track_log(self, log: LogEntry):
        self._pending_logs.append(log)
        self._cached_logs.append(log)

    def pop_stacktrace(self) -> Run:
        run = self._stacktrace_pop()
        # update cached info in parent(s)
        if run.value.cached_at is not None:
            self._update_cached_info()
        return run

    def run_enter(self, statement: "Statement", is_async: bool, inputs):
        # we set invalid values to none here unlike in other packing places because
        #  these values may be written even if invalid
        from bench.language.packer import check_type, pack_value

        assert not self.session.closed_at, f"cannot run {statement!r} in session {self.session!r}"

        run = self._create_run(
            statement=statement,
            inputs=pack_value(inputs, statement, is_output=False, none_if_invalid=True),
        )
        with self._tracing_lock:
            self._stacktrace_push(run)
            _set_active_run(run)
            self._track_run(run)  # tracker may mutate/do other things, so log after it's run
        logger.debug("trace.run.enter", run=run, stackdepth=len(self.stacktrace))

        # pre-run validation
        try:
            if len(self.stacktrace) >= MAX_STACK_DEPTH:
                raise RecursionError(f"maximum stack depth exceeded: {MAX_STACK_DEPTH}")
            check_type(inputs, statement, is_output=False)
        except BaseException as e:
            self.run_exception(statement, e)
            raise e

    def run_exit(self, statement: "Statement", outputs):
        from bench.language.packer import check_type

        assert not self.session.closed_at, f"cannot run {statement!r} in session {self.session!r}"

        # post-run validation
        try:
            check_type(outputs, statement, is_output=True)
        except BaseException as e:
            self.run_exception(statement, e)
            raise e

        with self._tracing_lock:
            run = self.pop_stacktrace()
            assert run.statement == statement, f"bad stack in {self!r}: {run!r} got {statement!r}"
            run.terminated_at = utcnow_with_tz()
            run.outputs = _pack_and_truncate_value(
                outputs, statement, is_output=True, none_if_invalid=True
            )
            run.status = RunStatus.COMPLETED
            self._track_run(run)
            _clear_active_run(run)
        logger.debug("trace.run.exit", run=run, stackdepth=len(self.stacktrace))

    def run_exception(self, statement: "Statement", exception: BaseException):
        assert not self.session.closed_at, f"cannot run {statement!r} in session {self.session!r}"
        with self._tracing_lock:
            run = self.pop_stacktrace()
            assert run.statement == statement, f"bad stack in {self!r}: {run!r} got {statement!r}"
            run.terminated_at = utcnow_with_tz()
            run.error = RunError.from_exception(exception, statement)
            if isinstance(exception, asyncio.CancelledError):
                run.status = RunStatus.ABORTED
            else:
                run.status = RunStatus.FAILED
            self._track_run(run)
            _clear_active_run(run)
        logger.debug("trace.run.exception", run=run, stackdepth=len(self.stacktrace))

    def run_cached(
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
        statement: Optional[Statement] = None,
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
        run_id = self._root_run_id if root is None else UUIDT()
        run = Run(
            id=run_id,
            ck=run_id,  # "detached"
            statement=statement,
            trigger_type=trigger_type,
            trigger=trigger,
            started_at=utcnow_with_tz(),
            inputs=inputs,
            status=RunStatus.QUEUED if queue_position is not None else RunStatus.RUNNING,
            value=(self._root_run_value or {}) if root is None else {},
            _track=NodeTrackingLevel.NONE,
            _session=UNSET,  # ensure run isn't validated/tracked in session
        )
        run._session = None  # reset to None so it can be activated
        # we track session nodes manually :ManualSessionTracking
        parent.runs.append(run, _trigger=_NC.UpdateLists, _create=False)
        custom_value = _custom_value.get()
        if root is None and self._root_run_value:
            run.value.update(self._root_run_value)
        if custom_value:
            run.value.update(custom_value)
        if self._global_run_value:
            run.value.update(self._global_run_value)
        return run

    @contextlib.contextmanager
    def capture(self) -> "_CapturedRuns":
        """Get all runs that are created within the context."""
        start_ids = set(self.runs.keys())
        capture = _CapturedRuns()
        try:
            yield
        finally:
            capture.runs = [run for run in self.runs.values() if run.id not in start_ids]

    @contextlib.contextmanager
    def value(self, **kwargs):
        """Set custom value for all runs created within the context."""
        old = _custom_value.get() or {}
        _custom_value.set({**old, **kwargs})
        try:
            yield
        finally:
            _custom_value.set(old or None)

    def start_capture(self) -> "_RunCapture":
        """Start capturing runs."""
        capture = _RunCapture(self)
        capture.start()
        return capture

    DEFAULT_RUN_UPDATE_PROPERTIES = (
        "status",
        "started_at",
        "terminated_at",
        "inputs",
        "outputs",
        "value",
        "error",
    )

    async def _flush(self, force: bool = False, kill_pending_runs: bool = False) -> None:
        """Flushes session data."""
        from bench.language import wiring

        if not force and not self._pending_logs and not self._pending_runs:
            return  # skip if nothing to commit

        with self._tracing_lock:
            runs_to_flush = list(self._pending_runs.values())
            self.session._log.debug("trace.flush", runs=runs_to_flush, logs=len(self._pending_logs))
            self._pending_runs.clear()

            if kill_pending_runs:
                # abort any remaining active runs
                for run in chain(runs_to_flush, self.runs.values()):
                    run._mark_dead_if_active()

            logs_to_flush = self._pending_logs
            self._pending_logs = []

        async with self.session._runtime.session_lock:
            # :ManualSessionTracking
            # turn session and runs into create/update edits (always update session)
            session_edits: list[EditData] = []
            runs_data: list[RunData] = []
            for n in chain((self.session,), runs_to_flush):
                edit_kind = (
                    EditKind.CREATE
                    if n.id not in self._flushed_session_node_ids
                    else EditKind.UPDATE
                )
                properties = (
                    self.DEFAULT_RUN_UPDATE_PROPERTIES
                    if edit_kind == EditKind.UPDATE and n.metatype == NodeType.RUN
                    else None
                )
                edit = EditData(
                    type=EditType.from_nt(edit_kind, n.metatype),
                    project_version_id=n.module.id,
                    file_id=None,
                    statement_id=None,
                    properties=properties,
                    revision=n.revision,
                )
                run_data: RunData = wiring.pack_node(n)
                edit.node = run_data
                session_edits.append(edit)
                if edit.node_type == NodeType.RUN:
                    runs_data.append(run_data)
                self._flushed_session_node_ids.add(n.id)

            await self.session._runtime.commit_edits(session_edits)
            await self.session._runtime.notify_runs_changed(runs_data)
        await self.session._write_logs(logs_to_flush)

    async def open(self, flush_interval: float = 0.1):
        self.stdout_collector.start()
        self.stderr_collector.start()

        _cancel = asyncio.Event()

        async def _flush_loop():
            while not _cancel.is_set():
                await self._flush()
                await asyncio.sleep(flush_interval)

        self._flush_cancel = _cancel
        self._flush_task = asyncio.create_task(_flush_loop())

    async def close(self):
        self.stdout_collector.stop()
        self.stderr_collector.stop()

        self._flush_cancel.set()
        await self._flush(force=True, kill_pending_runs=True)  # commit pending edits


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
            statement = active_run.statement
            run = active_run
        else:
            statement = None
            run = None
        log_entry = LogEntry(
            id=UUIDT(),
            module=self.session.module,
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


class PermissionError(Exception):
    pass


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
    type: Statement,
    ignore_array: bool = False,
    ignore_outer: bool = False,
    none_if_invalid: bool = False,
    is_output: bool = None,
) -> Any:
    from bench.language.packer import map_value, pack_value_flat

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


@dataclass
class _CapturedRuns:
    runs: list[Run] = None


class _RunCapture:
    def __init__(self, tracer: SessionTracer):
        self.tracer = tracer
        self._start_ids: set[UUID] | None = None

    def start(self):
        self._start_ids = set(self.tracer.runs.keys())

    def stop(self) -> list[Run]:
        runs = [run for run in self.tracer.runs.values() if run.id not in self._start_ids]
        self._start_ids = None
        return runs

    def stop_one_or_none(self) -> Run | None:
        runs = self.stop()
        if len(runs) == 0:
            return None
        if len(runs) > 1:
            raise ValueError(f"expected 1 run, got {len(runs)}")
        return runs[0]


@dataclass
class LazyRun:
    """Run that's not loaded."""

    id: UUID

    def load(self) -> "Run":
        raise NotImplementedError
