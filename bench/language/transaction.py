import asyncio
import dataclasses
from collections import defaultdict
from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, Literal, Optional, Union, cast
from uuid import UUID

import structlog

from bench.language.connection import StoreConnection, StoreEngine
from bench.language.const import BenchError, EditType, NodeType
from bench.language.node import Node, Property
from bench.proto import wire
from bench.proto.wire import AnyNodeData, ClientOrigin, EditData, GraphScope
from bench.utils.dt import utcnow
from bench.utils.func import uuid_to_str
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Run, Session, User

logger = structlog.get_logger(__name__)

dataclasses.field = dataclasses.field
EditSubject = Union["User", "Run"]


def new_edit_id() -> str:
    return str(UUIDT())


IMPLICIT_EDIT_PROPERTIES_NAMES: dict[EditType, tuple[str, ...]] = {
    EditType.CREATE: ("created_at", "created_epoch", "created_by_ptr"),
    EditType.UPSERT: (
        "created_at",
        "created_epoch",
        "created_by_ptr",
        "updated_at",
        "updated_epoch",
        "updated_by_ptr",
    ),
    EditType.UPDATE: ("updated_at", "updated_epoch", "updated_by_ptr"),
    EditType.MOVE: ("updated_at", "updated_epoch", "updated_by_ptr", "parent_ptr"),
    EditType.SOFT_DELETE: ("updated_at", "updated_epoch", "updated_by_ptr", "deleted_at"),
    EditType.RESTORE: ("updated_at", "updated_epoch", "updated_by_ptr", "deleted_at"),
    EditType.ARCHIVE: ("updated_at", "updated_epoch", "updated_by_ptr", "archived_at"),
    EditType.UNARCHIVE: ("updated_at", "updated_epoch", "updated_by_ptr", "archived_at"),
    EditType.DELETE: ("updated_at", "updated_epoch", "updated_by_ptr", "deleted_at"),
}
IMPLICIT_EDIT_PROPERTIES_IDS: dict[EditType, tuple[int, ...]] = {
    edit_type: tuple(Node.__properties__[name].id for name in names)
    for edit_type, names in IMPLICIT_EDIT_PROPERTIES_NAMES.items()
}


def _get_create_metadata(subject: EditSubject | None, now: datetime | None = None):
    subject_ptr = subject.to_ref()._to_data() if subject is not None else None
    return {"created_at": now or utcnow(), "created_epoch": -1, "created_by_ptr": subject_ptr}


def _get_update_metadata(subject: EditSubject | None, now: datetime | None = None):
    subject_ptr = subject.to_ref()._to_data() if subject is not None else None
    return {"updated_at": now or utcnow(), "updated_epoch": -1, "updated_by_ptr": subject_ptr}


@dataclasses.dataclass(slots=True)
class Transaction:
    """
    A transaction in the Bench state graph.
    Edits in a transaction are atomic (in our primary Postgres/Relational stores).
    """

    id: UUID = dataclasses.field(default_factory=UUIDT)
    session: Optional["Session"] = dataclasses.field(default=None)
    origin: ClientOrigin | None = dataclasses.field(default=None)
    is_readonly: bool = dataclasses.field(default=False)
    _connections_by_engine_id: dict[Any, StoreConnection] = dataclasses.field(default_factory=dict)

    """All edits from this transaction (since the previous commit)."""
    edits: list[EditData] = dataclasses.field(default_factory=list)
    cascaded_edits: list[EditData] = dataclasses.field(default_factory=list)
    _used_engine_ids: set[Any] = dataclasses.field(default_factory=set)

    """Pending (unflushed) edits."""
    _pending_edits_by_engine_id: dict[Any, list[EditData]] = dataclasses.field(
        default_factory=lambda: defaultdict(list)
    )
    _pending_updates_idx: dict[Node, tuple[Any, int]] = dataclasses.field(default_factory=dict)
    _pending_nodes_by_ck: dict[UUID, Node] = dataclasses.field(default_factory=dict)

    def __str__(self):
        return f"[id={self.id}] ({len(self.edits)} edits, {len(self._pending_nodes_by_ck)} pending nodes)"

    def __repr__(self):
        return f"<Transaction {self}>"

    @property
    def has_edits(self) -> bool:
        return len(self.edits) > 0

    @property
    def has_pending_edits(self) -> bool:
        return any(self._pending_edits_by_engine_id.values())

    async def connect(self, base: Node | GraphScope | None, node_type: NodeType) -> StoreConnection:
        """Gets a connection to the Store for some access."""
        assert self.session is not None, f"no session in {self!r}"
        if isinstance(base, Node):
            scope = self._get_scope_for_node(base)
        elif isinstance(base, GraphScope):
            scope = base
        else:
            assert base is None, f"unexpected base: {base!r}"
            scope = self.session._default_scope
        engine = self._get_engine_for(scope, node_type)
        return await self._get_engine_connection(engine)

    async def _get_engine_connection(self, engine: StoreEngine) -> StoreConnection:
        """Gets or creates a Store connection"""
        connection = self._connections_by_engine_id.get(engine.id)
        if connection is None:
            assert self.session is not None, f"no session in {self!r}"
            connection = await engine.connect(self.session)
            self._connections_by_engine_id[engine.id] = connection
        return connection

    def _get_scope_for_node(self, n: Node) -> GraphScope:
        """Gets the explicit or implicit scope for a node."""
        assert self.session is not None, f"no session in {self!r}"
        scope = GraphScope(
            bench_id=uuid_to_str(n.bench_id) if "bench" in n.__properties__ else None,
            package_id=uuid_to_str(n.package_id) if "package" in n.__properties__ else None,
        )
        scope = GraphScope()
        if "bench" in n.__properties__:
            scope.bench_id = uuid_to_str(n.bench_id) or self.session._default_scope.bench_id
        if "package" in n.__properties__:
            scope.package_id = uuid_to_str(n.package_id) or self.session._default_scope.package_id
        return scope

    def _get_engine_for(self, scope: GraphScope, node_type: NodeType) -> StoreEngine:
        """Gets the appropriate engine"""
        assert self.session is not None, f"no session in {self!r}"
        for engine in self.session._engines:
            if engine.supports(scope, node_type):
                return engine
        raise BenchError(
            f"no engine for [scope={scope!r}, node_type={node_type.bench_name}] in {self.session!r}"
            f" (engines: {self.session._engines!r})"
        )

    #
    # Edits
    #

    def _make_edit(
        self,
        type: EditType,
        n: Node,
        metadata: dict[str, Any],
    ) -> EditData:
        """Creates an edit and adds it to the pending edits."""
        assert self.session is not None, f"no session for {self!r}"
        if self.is_readonly:
            raise RuntimeError(f"cannot {type.bench_name} {n!r} in read-only {self.session}")

        from bench.proto import wiring

        # pack node (with extra data)
        node_data = cast(AnyNodeData, n._to_data())
        if n._updated_properties:
            properties = list(n._unmask_properties_ids(n._updated_properties))
        else:
            properties = []
        for key, value in metadata.items():
            prop = n.__properties__.get(key)
            if prop is not None:
                setattr(node_data, key, value)

        # make edit
        scope = self._get_scope_for_node(n)
        edit = EditData(
            id=new_edit_id(),
            type=wiring.pack_enum(EditType, type),
            node_type=cast(wire.NodeType, node_data.metatype),
            node=wiring.wrap_some_node(node_data),
            properties=list(properties),
            scope=scope,
            origin=self.origin,
        )
        return edit

    def _add_pending_edit(self, edit: EditData, node: Optional[Node]) -> StoreEngine:
        from bench.proto import wiring

        node_type = wiring.unpack_enum(NodeType, edit.node_type)
        engine = self._get_engine_for(edit.scope, node_type)
        self.edits.append(edit)
        self._pending_edits_by_engine_id[engine.id].append(edit)
        if node is not None:
            self._pending_nodes_by_ck[node.ck] = node
        return engine

    def _add_pending_edits(self, edits: Collection[EditData]):
        """Adds a collection of edits to the pending edits."""
        for edit in edits:
            self._add_pending_edit(edit, node=None)

    def create(self, n: Node, subject: EditSubject | None):
        now = utcnow()
        edit = self._make_edit(
            type=EditType.CREATE,
            n=n,
            metadata={**_get_create_metadata(subject, now), **_get_update_metadata(subject, now)},
        )
        self._add_pending_edit(edit, n)

    def upsert(self, n: Node, subject: EditSubject | None):
        now = utcnow()
        edit = self._make_edit(
            type=EditType.UPSERT,
            n=n,
            metadata={**_get_create_metadata(subject, now), **_get_update_metadata(subject, now)},
        )
        self._add_pending_edit(edit, n)

    def _update(
        self,
        edit_type: Literal[EditType.UPDATE, EditType.MOVE],
        n: Node,
        subject: EditSubject | None,
        properties: Collection[Property],
    ):
        """Update or move a node."""
        from bench.proto import wiring

        existing_edit_idx = self._pending_updates_idx.get(n)
        if existing_edit_idx is None:
            # new update/move
            edit = self._make_edit(edit_type, n, _get_update_metadata(subject))
            engine = self._add_pending_edit(edit, n)
            edit_idx = len(self._pending_edits_by_engine_id[engine.id]) - 1
            self._pending_updates_idx[n] = engine.id, edit_idx
        else:
            # update existing edit in place
            #  (to avoid re-packing everything for successive updates)
            assert n._updated_properties is not None, f"missing property mask for {n!r}"
            engine_id, current_update_idx = existing_edit_idx
            edit = self._pending_edits_by_engine_id[engine_id][current_update_idx]
            edit.properties = list(n._unmask_properties_ids(n._updated_properties))
            node_data = wiring.unwrap_some_node(edit.node)
            for prop in properties:
                if prop.reference_wired_ptr:
                    prop = prop.reference_wired_ptr
                value = getattr(n, prop.name)
                value = wiring.pack_struct_prop(prop, value, ignore_array=False)
                setattr(node_data, prop.name, value)
            # coalesce any successive move/update into a move
            if edit_type == EditType.MOVE and edit.type != EditType.MOVE:
                edit.type = wiring.pack_enum(EditType, EditType.MOVE)

    def update(self, n: Node, subject: EditSubject | None, properties: Collection[Property]):
        self._update(EditType.UPDATE, n, subject, properties)

    def move(self, n: Node, subject: EditSubject | None):
        self._update(EditType.MOVE, n, subject, [])

    def soft_delete(self, n: Node, subject: EditSubject | None):
        now = utcnow()
        edit = self._make_edit(
            type=EditType.SOFT_DELETE,
            n=n,
            metadata={**_get_update_metadata(subject, now), "deleted_at": now},
        )
        self._add_pending_edit(edit, n)

    def restore(self, n: Node, subject: EditSubject | None):
        edit = self._make_edit(
            type=EditType.RESTORE,
            n=n,
            metadata={**_get_update_metadata(subject), "deleted_at": None},
        )
        self._add_pending_edit(edit, n)

    def archive(self, n: Node, subject: EditSubject | None):
        now = utcnow()
        edit = self._make_edit(
            type=EditType.ARCHIVE,
            n=n,
            metadata={**_get_update_metadata(subject, now), "archived_at": now},
        )
        self._add_pending_edit(edit, n)

    def unarchive(self, n: Node, subject: EditSubject | None):
        edit = self._make_edit(
            type=EditType.UNARCHIVE,
            n=n,
            metadata={**_get_update_metadata(subject), "archived_at": None},
        )
        self._add_pending_edit(edit, n)

    def delete(self, n: Node, subject: EditSubject | None):
        now = utcnow()
        edit = self._make_edit(
            type=EditType.DELETE,
            n=n,
            metadata={**_get_update_metadata(subject, now), "deleted_at": now},
        )
        self._add_pending_edit(edit, n)

    #
    # Transaction management
    #

    async def open(self):
        pass

    async def _do_flush(self, *, commit: bool):
        assert self.session is not None, f"no session for {self!r}"
        start = asyncio.get_running_loop().time()
        log = logger.bind(edits=len(self.edits), transaction=self)

        # TODO :Robustness!: use :2PC in Transaction.commit (if there are more than 2 engines)
        for engine in self.session._engines:
            # prepare edits & connection
            pending_edits = self._pending_edits_by_engine_id.get(engine.id, [])
            if not (pending_edits or (commit and engine.id in self._used_engine_ids)):
                continue  # nothing to do
            connection = await self._get_engine_connection(engine)

            # flush/commit
            if commit:
                flush = await connection.commit(pending_edits)
                log.trace("transaction.commit.engine", engine=engine, edits=len(pending_edits))
            else:
                flush = await connection.flush(pending_edits)
                log.trace("transaction.flush.engine", engine=engine, edits=len(pending_edits))
            assert len(flush.revisions or ()) == len(pending_edits), "revisions mismatch"
            for edit, new_revision in zip(pending_edits, cast(list[int], flush.revisions)):
                edit.revision = new_revision
            pending_edits.clear()
            self.cascaded_edits.extend(flush.cascaded_edits)
            self._used_engine_ids.add(engine.id)
        self._pending_edits_by_engine_id.clear()
        self._pending_updates_idx.clear()

        # mark nodes as flushed
        for n in self._pending_nodes_by_ck.values():
            n._flushed_self()
        self._pending_nodes_by_ck.clear()
        log.trace("transaction.flush", duration=asyncio.get_running_loop().time() - start)

    async def flush(self) -> tuple[list[EditData], list[EditData]]:
        """Canonicalizes and flushes any pending edits (without committing)."""
        await self._do_flush(commit=False)
        return self.edits, self.cascaded_edits

    async def commit(self) -> tuple[list[EditData], list[EditData]]:
        """Commits the transaction (also flushing any pending edits)."""
        await self._do_flush(commit=True)
        edits, cascaded_edits = self.edits, self.cascaded_edits
        self.edits, self.cascaded_edits = [], []
        self._used_engine_ids.clear()
        return edits, cascaded_edits

    async def rollback(self):
        """Rolls back uncommitted edits in primary stores."""
        raise NotImplementedError("not yet supported")  # :2PC

    async def close(self):
        """Closes the transaction and associated store engines, rolling back uncommitted edits."""
        for connection in self._connections_by_engine_id.values():
            await connection.close()
        self._connections_by_engine_id.clear()
