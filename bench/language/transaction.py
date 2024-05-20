import asyncio
import dataclasses
from collections import defaultdict
from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast
from uuid import UUID

import structlog

from bench.language.connection import StoreConnection, StoreEngine
from bench.language.const import BenchError, EditType, NodeType
from bench.language.node import Node, Property
from bench.proto import wire
from bench.proto.wire import AnyNodeData, EditData, GraphScope, NodeReferenceData
from bench.utils.dt import utcnow
from bench.utils.func import uuid_to_str
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Run, Session, User

logger = structlog.get_logger(__name__)

dcfield = dataclasses.field
EditSubject = Union["User", "Run"]  # noqa


def new_edit_id() -> str:
    return str(UUIDT())


@dataclasses.dataclass(slots=True)
class Transaction:
    """
    A transaction in the Bench state graph.
    Edits in a transaction are atomic (in our primary Postgres/Relational stores).
    """

    id: UUID = dcfield(default_factory=UUIDT)
    session: Optional["Session"] = dcfield(default=None)
    is_readonly: bool = dcfield(default=False)
    _connections_by_engine_id: dict[Any, StoreConnection] = dcfield(default_factory=dict)

    edits: list[EditData] = dcfield(default_factory=list)
    _pending_edits_by_engine_id: dict[Any, list[EditData]] = dcfield(
        default_factory=lambda: defaultdict(list)
    )
    _pending_updates_idx: dict[Node, tuple[Any, int]] = dcfield(default_factory=dict)
    _pending_nodes_by_ck: dict[UUID, Node] = dcfield(default_factory=dict)

    # for syncing databases (should probably generalize into' untracked edits')
    _schema_changed: bool = dcfield(default=False)

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
        connection = self._connections_by_engine_id.get(engine.id)
        if connection is None:
            assert self.session is not None, f"no session in {self!r}"
            connection = await engine.connect(self.session)
            self._connections_by_engine_id[engine.id] = connection
        return connection

    def _get_scope_for_node(self, n: Node) -> GraphScope:
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

    def _make_edit(self, type: EditType, n: Node, subject: EditSubject | None) -> EditData:
        """Creates an edit and adds it to the pending edits."""
        assert self.session is not None, f"no session for {self!r}"
        if self.is_readonly:
            raise RuntimeError(f"cannot {type.bench_name} {n!r} in read-only {self.session}")

        from bench.proto import wiring

        node_data = cast(AnyNodeData, n._to_data())
        if n._updated_properties:
            properties = n._unmask_properties_ids(n._updated_properties)
        else:
            properties = None
        scope = self._get_scope_for_node(n)
        subject_data = (
            cast(NodeReferenceData, wiring.pack_struct(subject.to_ref()))
            if subject is not None
            else None
        )
        edit = EditData(
            id=new_edit_id(),
            type=wiring.pack_enum(EditType, type),
            node_type=cast(wire.NodeType, node_data.metatype),
            node=wiring.wrap_some_node(node_data),
            properties=list(properties) if properties is not None else [],
            scope=scope,
            subject=subject_data,
            revision=None,  # not known yet
        )
        return edit

    def _add_pending_edit(self, edit: EditData, node: Optional[Node]) -> StoreEngine:
        from bench.proto import wiring

        if edit.node_type == NodeType.FIELD:
            self._schema_changed = True

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
        edit = self._make_edit(EditType.CREATE, n, subject)
        self._add_pending_edit(edit, n)

    def upsert(self, n: Node, subject: EditSubject | None):
        edit = self._make_edit(EditType.UPSERT, n, subject)
        self._add_pending_edit(edit, n)

    def update(self, n: Node, subject: EditSubject | None, properties: Collection[Property]):
        from bench.proto import wiring

        existing_edit_idx = self._pending_updates_idx.get(n)
        if existing_edit_idx is None:
            # new update
            edit = self._make_edit(EditType.UPDATE, n, subject)
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

    def move(self, n: Node, subject: EditSubject | None):
        edit = self._make_edit(EditType.MOVE, n, subject)
        self._add_pending_edit(edit, n)

    def soft_delete(self, n: Node, subject: EditSubject | None):
        edit = self._make_edit(EditType.SOFT_DELETE, n, subject)
        self._add_pending_edit(edit, n)

    def restore(self, n: Node, subject: EditSubject | None):
        edit = self._make_edit(EditType.RESTORE, n, subject)
        self._add_pending_edit(edit, n)

    def archive(self, n: Node, subject: EditSubject | None):
        edit = self._make_edit(EditType.ARCHIVE, n, subject)
        self._add_pending_edit(edit, n)

    def unarchive(self, n: Node, subject: EditSubject | None):
        edit = self._make_edit(EditType.UNARCHIVE, n, subject)
        self._add_pending_edit(edit, n)

    def delete(self, n: Node, subject: EditSubject | None):
        edit = self._make_edit(EditType.DELETE, n, subject)
        self._add_pending_edit(edit, n)

    #
    # Transaction management
    #

    @staticmethod
    def canonicalize_edits(now: datetime, edits: Collection[EditData]):
        """
        'Canonicalizes' the edits in place by imputing the tracking info (e.g. 'updated_at', 'updated_by').
        We do this in the untrusted runtimes as well as in the system, but only the system counts,
         because the tracking properties are not directly updatable (being system properties).
        NOTE :Architecture :Cleanup: edit canonicalization is necessary? but confusing :EditCanonicalization
        """
        from bench.proto import wiring

        for edit in edits:
            node = wiring.unwrap_some_node(edit.node)
            if edit.type in (EditType.CREATE, EditType.UPSERT):
                node.created_at = now
                node.created_by_ptr = edit.subject
                node.updated_at = now
                node.updated_by_ptr = edit.subject
            elif edit.type in (EditType.MOVE, EditType.UPDATE):
                node.updated_at = now
                node.updated_by_ptr = edit.subject
            elif edit.type == EditType.ARCHIVE:
                node.archived_at = now
            elif edit.type == EditType.UNARCHIVE:
                node.archived_at = None
            elif edit.type == EditType.SOFT_DELETE:
                node.deleted_at = now
            elif edit.type == EditType.RESTORE:
                node.deleted_at = None
            elif edit.type == EditType.DELETE:
                node.deleted_at = now  # technically unnecessary but convenient
            else:
                raise ValueError(f"unexpected edit type: {edit.type.name}")

    async def open(self):
        pass

    async def _do_flush(self, *, commit: bool):
        assert self.session is not None, f"no session for {self!r}"
        start = asyncio.get_running_loop().time()
        log = logger.bind(edits=len(self.edits), transaction=self)
        now = utcnow()

        # TODO :Robustness!: use :2PC in Transaction.commit
        #  (if there are more than 2 engines to commit to)
        for engine in self.session._engines:
            # prepare edits & connection
            pending_edits = self._pending_edits_by_engine_id.get(engine.id, ())
            if not pending_edits:
                continue
            Transaction.canonicalize_edits(now, pending_edits)
            connection = await self._get_engine_connection(engine)

            # flush/commit
            log.trace(
                "transaction.flush.engine", engine=engine, edits=len(pending_edits), commit=commit
            )
            if commit:
                accepted_revisions = await connection.commit(pending_edits)
            else:
                accepted_revisions = await connection.flush(pending_edits)
            assert len(accepted_revisions or ()) == len(pending_edits), "revisions mismatch"
            for edit, new_revision in zip(pending_edits, cast(list[int], accepted_revisions)):
                edit.revision = new_revision
            pending_edits.clear()
        self._pending_edits_by_engine_id.clear()
        self._pending_updates_idx.clear()

        # mark nodes as flushed
        for n in self._pending_nodes_by_ck.values():
            n._flushed_self()
        self._pending_nodes_by_ck.clear()
        log.trace("transaction.flush", duration=asyncio.get_running_loop().time() - start)

    async def flush(self):
        """Canonicalizes and flushes any pending edits (without committing)."""
        await self._do_flush(commit=False)

    async def commit(self):
        """Commits the transaction (also flushing any pending edits)."""
        await self._do_flush(commit=True)

    async def rollback(self):
        """Rolls back uncommitted edits in primary stores."""
        raise NotImplementedError("not yet supported")  # :2PC

    async def close(self):
        """Closes the transaction and associated store engines, rolling back uncommitted edits."""
        for connection in self._connections_by_engine_id.values():
            await connection.close()
        self._connections_by_engine_id.clear()
