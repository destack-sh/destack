import dataclasses
from collections import defaultdict
from typing import TYPE_CHECKING, Any, Collection, Literal, Optional, cast
from uuid import UUID

import structlog
from betterproto.lib.google.protobuf import Struct as ProtoStruct
from opentelemetry import trace

from bench.language.connection import (
    InMemoryEngine,
    SplitConnection,
    StoreConnection,
    StoreEngine,
    scope_includes,
)
from bench.language.const import UNSET, BenchError, EditType, NodeType
from bench.language.graph import DetachedNodeGraph, NodeDataGraph, NodeGraph
from bench.language.node import EditSubject, Node, Property
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.proto.wire import AnyNodeData, ClientOrigin, EditData, GraphScope
from bench.utils.dt import utcnow
from bench.utils.func import uuid_to_str
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import ReadOptions, Session

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def new_edit_id() -> str:
    return str(UUIDT())


@dataclasses.dataclass(slots=True)
class Transaction:
    """
    A transaction in the Bench state graph.
    Edits in a transaction are atomic (in our primary Postgres/Relational stores).
    """

    id: UUID
    session: "Session"
    is_readonly: bool = dataclasses.field(default=False)
    _read_connection: StoreConnection = dataclasses.field(init=False)
    _connections_by_engine_id: dict[Any, StoreConnection] = dataclasses.field(default_factory=dict)

    """All edits from this transaction (since the previous commit)."""
    edits: list[EditData] = dataclasses.field(default_factory=list)
    cascaded_edits: list[EditData] = dataclasses.field(default_factory=list)
    _used_engine_ids: set[Any] = dataclasses.field(default_factory=set)

    """Pending (unflushed) edits."""
    pending_edits: list[EditData] = dataclasses.field(default_factory=list)
    _pending_edits_by_engine_id: dict[Any, list[EditData]] = dataclasses.field(
        default_factory=lambda: defaultdict(list)
    )
    _pending_updates_idx: dict[Node, tuple[Any, int]] = dataclasses.field(default_factory=dict)
    _pending_nodes_by_ck: dict[UUID, Node] = dataclasses.field(default_factory=dict)

    def __post_init__(self):
        self._read_connection = SplitConnection(self.session)

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

    async def _get_engine_connection(self, engine: StoreEngine) -> StoreConnection:
        """Gets or creates a store connection"""
        connection = self._connections_by_engine_id.get(engine.id)
        if connection is None:
            connection = await engine.connect(self.session)
            self._connections_by_engine_id[engine.id] = connection
        return connection

    def _get_scope_for_node(self, n: Node) -> GraphScope:
        """Gets the explicit or implicit scope for a node."""
        scope = GraphScope()
        if "bench" in n.__properties__:
            scope.bench_id = uuid_to_str(n.bench_id) or self.session._default_scope.bench_id
        if "package" in n.__properties__:
            scope.package_id = uuid_to_str(n.package_id) or self.session._default_scope.package_id
        return scope

    def _get_engine(
        self,
        scope: GraphScope,
        node_types: NodeType | Collection[NodeType],
        *,
        is_readonly: bool,
        best_match: Collection[NodeType] | None = None,
    ) -> StoreEngine:
        """Gets the appropriate engine"""
        node_types = (node_types,) if isinstance(node_types, NodeType) else node_types
        candidate_engines = [
            engine
            for engine in self.session._engines
            if (
                (is_readonly or not engine.is_readonly)
                and scope_includes(engine.scope, scope)
                and all(t in engine.node_types for t in node_types)
            )
        ]
        if not candidate_engines:
            raise BenchError(
                f"no engine for [scope={scope!r}, node_types={[t.bench_name for t in node_types]}] in {self.session!r}"
                f" (engines: {self.session._engines!r})"
            )
        if best_match is None or len(candidate_engines) < 2:
            return candidate_engines[0]
        else:
            # try to find best match (most type overlap, best first)
            candidate_engines.sort(key=lambda e: -len([t for t in best_match if t in e.node_types]))
            if any(isinstance(e, InMemoryEngine) for e in candidate_engines):
                # prefer in-memory engines
                return next(e for e in candidate_engines if isinstance(e, InMemoryEngine))
            return candidate_engines[0]

    #
    # Edits
    #

    def _make_simple_edit(
        self,
        edit_type: Literal[
            EditType.CREATE,
            EditType.UPSERT,
            EditType.ARCHIVE,
            EditType.UNARCHIVE,
            EditType.SOFT_DELETE,
            EditType.RESTORE,
            EditType.DELETE,
        ],
        node_: Node,
        subject: EditSubject | None,
        origin: ClientOrigin | None,
    ) -> EditData:
        """Creates a simple non-update/move edit and adds it to the pending edits."""
        assert self.session is not None, f"no session for {self!r}"
        if self.is_readonly:
            raise RuntimeError(
                f"cannot {edit_type.bench_name} {node_!r} in read-only {self.session}"
            )

        from bench.proto import wiring

        if edit_type in (EditType.CREATE, EditType.UPSERT):
            new_node_packed = pack_node_delta(node_._to_data())
            old_node_packed = None
        elif edit_type == EditType.DELETE:
            new_node_packed = None
            old_node_packed = pack_node_delta(node_._to_data())
        else:
            new_node_packed = None
            old_node_packed = None

        # make edit
        edit = EditData(
            id=new_edit_id(),
            type=wiring.pack_enum(EditType, edit_type),
            node_ptr=node_.to_ref()._to_data(),
            new_node_packed=new_node_packed,
            old_node_packed=old_node_packed,
            scope=self._get_scope_for_node(node_),
            origin=origin,
            subject_ptr=subject.to_ref()._to_data() if subject is not None else None,
            edited_at=utcnow(),
        )
        return edit

    def create(self, node_: Node, subject: EditSubject | None, origin: ClientOrigin | None):
        edit = self._make_simple_edit(EditType.CREATE, node_, subject=subject, origin=origin)
        self._add_pending_edit(edit, node_)

    def upsert(self, node_: Node, subject: EditSubject | None, origin: ClientOrigin | None):
        edit = self._make_simple_edit(EditType.UPSERT, node_, subject=subject, origin=origin)
        self._add_pending_edit(edit, node_)

    def _do_update(
        self,
        edit_type: Literal[EditType.UPDATE, EditType.MOVE],
        node_: Node,
        subject: EditSubject | None,
        origin: ClientOrigin | None,
        properties: Collection[Property],
        old_values: dict[int, Any],
    ):
        """Update or move a node."""
        from bench.language.value import pack_value
        from bench.proto import wiring

        existing_edit_idx = self._pending_updates_idx.get(node_)
        if existing_edit_idx is None:
            # new update/move
            old_node_packed = {}
            new_node_packed = {}
            for prop in properties:
                if prop.reference_wired_ptr is not None:
                    prop = prop.reference_wired_ptr
                old_value = old_values.get(prop.id, UNSET)
                assert old_value is not UNSET, f"missing old value for {prop!r} in {node_!r}"
                prop_type = prop.as_type_info
                old_node_packed[prop.id_as_str], _ = pack_value(
                    old_value, prop_type, wrap_primitive=False
                )
                new_value = getattr(node_, prop.name)
                new_node_packed[prop.id_as_str], _ = pack_value(
                    new_value, prop_type, wrap_primitive=False
                )
            edit = EditData(
                id=new_edit_id(),
                type=wiring.pack_enum(EditType, edit_type),
                node_ptr=node_.to_ref()._to_data(),
                properties=[prop.id for prop in properties],
                old_node_packed=wiring.pack_proto_json(old_node_packed),
                new_node_packed=wiring.pack_proto_json(new_node_packed),
                scope=self._get_scope_for_node(node_),
                origin=origin,
                subject_ptr=subject.to_ref()._to_data() if subject is not None else None,
                edited_at=utcnow(),
            )
            engine = self._add_pending_edit(edit, node_)
            edit_idx = len(self._pending_edits_by_engine_id[engine.id]) - 1
            self._pending_updates_idx[node_] = engine.id, edit_idx
        else:
            # update existing edit in place ('debounce')
            # NOTE :Performance: unpacking/repacking proto json is inefficient
            assert node_._updated_properties is not None, f"missing property mask for {node_!r}"
            engine_id, current_update_idx = existing_edit_idx
            edit = self._pending_edits_by_engine_id[engine_id][current_update_idx]
            edit.properties = list(node_._unmask_properties_ids(node_._updated_properties))
            assert edit.new_node_packed and edit.old_node_packed, f"missing node data for {edit!r}"
            new_node_packed = wiring.unpack_proto_json(edit.new_node_packed)
            old_node_packed = wiring.unpack_proto_json(edit.old_node_packed)
            for prop in properties:
                if prop.reference_wired_ptr is not None:
                    prop = prop.reference_wired_ptr
                if prop.id_as_str not in old_node_packed:
                    # add old value if it doesn't already exist
                    old_value = old_values.get(prop.id, UNSET)
                    assert old_value is not UNSET, f"missing old value for {prop!r} in {node_!r}"
                    old_node_packed[prop.id_as_str], _ = pack_value(
                        old_value, prop.as_type_info, wrap_primitive=False
                    )
                # and update new value
                new_value = getattr(node_, prop.name)
                new_node_packed[prop.id_as_str], _ = pack_value(
                    new_value, prop.as_type_info, wrap_primitive=False
                )
            edit.new_node_packed = wiring.pack_proto_json(new_node_packed)
            edit.old_node_packed = wiring.pack_proto_json(old_node_packed)
            if edit_type == EditType.MOVE and edit.type != EditType.MOVE:
                edit.type = wiring.pack_enum(EditType, EditType.MOVE)

    def update(
        self,
        node_: Node,
        subject: EditSubject | None,
        origin: ClientOrigin | None,
        properties: Collection[Property],
        old_values: dict[int, Any],
    ):
        self._do_update(EditType.UPDATE, node_, subject, origin, properties, old_values)

    def move(
        self,
        node_: Node,
        subject: EditSubject | None,
        origin: ClientOrigin | None,
        properties: Collection[Property],
        old_values: dict[int, Any],
    ):
        self._do_update(EditType.MOVE, node_, subject, origin, properties, old_values)

    def soft_delete(self, node_: Node, subject: EditSubject | None, origin: ClientOrigin | None):
        edit = self._make_simple_edit(EditType.SOFT_DELETE, node_, subject=subject, origin=origin)
        self._add_pending_edit(edit, node_)

    def restore(self, node_: Node, subject: EditSubject | None, origin: ClientOrigin | None):
        edit = self._make_simple_edit(EditType.RESTORE, node_, subject=subject, origin=origin)
        self._add_pending_edit(edit, node_)

    def archive(self, node_: Node, subject: EditSubject | None, origin: ClientOrigin | None):
        edit = self._make_simple_edit(EditType.ARCHIVE, node_, subject=subject, origin=origin)
        self._add_pending_edit(edit, node_)

    def unarchive(self, node_: Node, subject: EditSubject | None, origin: ClientOrigin | None):
        edit = self._make_simple_edit(
            EditType.UNARCHIVE, node_=node_, subject=subject, origin=origin
        )
        self._add_pending_edit(edit, node_)

    def delete(self, node_: Node, subject: EditSubject | None, origin: ClientOrigin | None):
        edit = self._make_simple_edit(EditType.DELETE, node_, subject=subject, origin=origin)
        self._add_pending_edit(edit, node_)

    #
    # Transaction management
    #

    async def open(self):
        pass

    def _add_pending_edit(self, edit: EditData, node: Optional[Node]) -> StoreEngine:
        from bench.proto import wiring

        node_type = wiring.unpack_enum(NodeType, edit.node_ptr.type)
        engine = self._get_engine(edit.scope, node_type, is_readonly=False)
        self.edits.append(edit)
        self.pending_edits.append(edit)
        self._pending_edits_by_engine_id[engine.id].append(edit)
        if node is not None:
            self._pending_nodes_by_ck[node.ck] = node
        # bump epoch if we have one
        if self.session._epoch is not None:
            self.session._epoch += 1
            edit.epoch = self.session._epoch
        return engine

    def _add_pending_edits(self, edits: Collection[EditData]):
        for edit in edits:
            self._add_pending_edit(edit, node=None)

    async def _do_flush(self, *, commit: bool):
        assert self.session is not None, f"no session for {self!r}"
        log = logger.bind(edits=len(self.edits), transaction=self)

        # TODO :Robustness!: use :2PC in Transaction.commit (if there are more than 2 engines)
        for engine in self.session._engines:
            # prepare edits & connection
            pending_edits = self._pending_edits_by_engine_id.get(engine.id, [])
            if not (pending_edits or (commit and engine.id in self._used_engine_ids)):
                continue  # nothing to do
            connection = await self._get_engine_connection(engine)

            # flush/commit
            message = "transaction.commit.engine" if commit else "transaction.flush.engine"
            with tracer.start_as_current_span(
                message, attributes={"engine": engine.__class__.__name__}
            ):
                if commit:
                    flush = await connection.commit(pending_edits)
                else:
                    flush = await connection.flush(pending_edits)
                log.trace(message, engine=engine, edits=len(pending_edits))
            assert len(flush.revisions or ()) == len(pending_edits), "revisions mismatch"
            for edit, new_revision in zip(pending_edits, cast(list[int], flush.revisions)):
                edit.revision = new_revision
            pending_edits.clear()
            self.cascaded_edits.extend(flush.cascaded_edits)
            self._used_engine_ids.add(engine.id)
        self.pending_edits.clear()
        self._pending_edits_by_engine_id.clear()
        self._pending_updates_idx.clear()

        # mark nodes as flushed
        for n in self._pending_nodes_by_ck.values():
            n._flushed_self()
        self._pending_nodes_by_ck.clear()
        log.trace("transaction.commit" if commit else "transaction.flush")

    @tracer.start_as_current_span("transaction.flush")
    async def flush(self) -> tuple[list[EditData], list[EditData]]:
        """Canonicalizes and flushes any pending edits (without committing)."""
        await self._do_flush(commit=False)
        return self.edits, self.cascaded_edits

    @tracer.start_as_current_span("transaction.commit")
    async def commit(self) -> tuple[list[EditData], list[EditData]]:
        """Commits the transaction (also flushing any pending edits)."""
        await self._do_flush(commit=True)
        edits, cascaded_edits = self.edits, self.cascaded_edits
        self.edits, self.cascaded_edits = [], []
        self._used_engine_ids.clear()
        return edits, cascaded_edits

    @tracer.start_as_current_span("transaction.rollback")
    async def rollback(self):
        """Rolls back uncommitted edits in primary stores."""
        raise NotImplementedError("not yet supported")  # :2PC

    async def close(self):
        """Closes the transaction and associated store engines, rolling back uncommitted edits."""
        for connection in self._connections_by_engine_id.values():
            await connection.close()
        self._connections_by_engine_id.clear()


def pack_node_delta(
    node_data: AnyNodeData, *, only: Collection[Property | Any] | None = None
) -> ProtoStruct:
    """Packs a node into its edit representation. If 'only' is set, only those properties are packed."""
    from bench.language.value import pack_struct_value_scalar_data

    node_packed = pack_struct_value_scalar_data(node_data, only=only)
    return ProtoStruct.from_dict(node_packed)  # type: ignore


def unpack_node_delta(
    node_packed: dict[str, Any] | ProtoStruct,
    *,
    node_type: NodeType | None = None,
    only: Collection[Property | Any] | None = None,
) -> AnyNodeData:
    """Unpacks a node from its packed edit representation. If 'only' is set, only those properties are unpacked."""
    from bench.language.value import unpack_struct_value_scalar_data
    from bench.proto import wiring

    if not isinstance(node_packed, dict):
        node_packed = node_packed.to_dict()

    proto_cls = wiring.PROTO_CLASS_BY_TYPE[node_type] if node_type is not None else None
    node_data = unpack_struct_value_scalar_data(node_packed, expect=proto_cls, only=only)
    return cast(AnyNodeData, node_data)


@tracer.start_as_current_span("graph.edit_graph")
def edit_graph(
    graph: NodeGraph["Node"] | DetachedNodeGraph["Node"],
    edits: Collection[EditData],
    options: "ReadOptions | None",
) -> None:
    """Applies the edits to the graph (in place!)."""
    trace.get_current_span().set_attribute("edits", len(edits))

    from bench.language.query import DEFAULT_READ_OPTIONS
    from bench.proto import wiring

    if options is None:
        options = DEFAULT_READ_OPTIONS

    for edit in edits:
        assert edit.epoch is not None, f"missing epoch for {edit!r}"
        assert edit.revision is not None, f"missing revision for {edit!r}"
        edit_type = cast(EditType, edit.type)
        node_type = NodeType(edit.node_ptr.type)
        node_id = UUID(edit.node_ptr.id)

        if edit_type in (EditType.CREATE, EditType.UPSERT) or (
            not options.include_hidden and edit_type in (EditType.UNARCHIVE, EditType.RESTORE)
        ):
            assert edit.new_node_packed, f"missing new node for {edit!r}"
            new_node_data = unpack_node_delta(edit.new_node_packed)
            # inline implicit metadata
            new_node_data.created_at = new_node_data.updated_at = edit.edited_at
            if hasattr(new_node_data, "created_epoch"):
                setattr(new_node_data, "created_epoch", edit.edited_at)
                setattr(new_node_data, "updated_epoch", edit.edited_at)
            new_node_data.created_by_ptr = new_node_data.updated_by_ptr = edit.subject_ptr
            # unpack
            if new_node_data.parent_ptr is not None:
                parent = graph.get(UUID(new_node_data.parent_ptr.id))
            else:
                parent = None
            node = wiring.unpack_node(new_node_data, parent)
            if edit_type == EditType.CREATE or node.id not in graph:
                graph.add(node)
            else:
                graph.update(node)
        elif edit_type == EditType.DELETE or (
            not options.include_hidden and edit_type in (EditType.ARCHIVE, EditType.SOFT_DELETE)
        ):
            node = graph.get(node_id)
            assert node is not None, f"missing node {node_id!r} for remove: {edit!r}"
            graph.remove(node)
        else:
            # some update
            assert edit.new_node_packed, f"missing new node for {edit!r}"
            new_node_data = unpack_node_delta(edit.new_node_packed, node_type=node_type)
            node = graph.get(node_id)
            assert node is not None, f"missing node {node_id!r} for update: {edit!r}"
            # directly edited properties
            for prop_id in edit.properties:
                prop = node.__properties_by_id__.get(prop_id)
                assert prop is not None, f"no property {prop_id} for {node!r} in {edit!r}"
                if prop.reference_wired_ptr is not None:
                    prop = prop.reference_wired_ptr
                new_value_data = getattr(new_node_data, prop.name)
                new_value = wiring.unpack_struct_prop(prop, new_value_data)
                setattr(node, prop.name, new_value)
            # implicit metadata
            node.updated_at = edit.edited_at
            if "updated_epoch" in node.__properties__:
                node.updated_epoch = edit.epoch
            setattr(node, "updated_by_ptr", edit.subject_ptr)
            node.revision = edit.revision
            if edit_type == EditType.ARCHIVE:
                node.archived_at = edit.edited_at
            elif edit_type == EditType.UNARCHIVE:
                node.archived_at = None
            elif edit_type == EditType.SOFT_DELETE:
                node.deleted_at = edit.edited_at
            elif edit_type == EditType.RESTORE:
                node.deleted_at = None
            elif edit_type == EditType.MOVE:
                if new_node_data.parent_ptr is not None:
                    new_parent = graph.get(UUID(new_node_data.parent_ptr.id))
                else:
                    new_parent = None
                node.parent = new_parent
            graph.update(node)


@tracer.start_as_current_span("graph.edit_data_graph")
def edit_data_graph(
    graph: NodeDataGraph[AnyNodeData],
    edits: Collection[EditData],
    options: "ReadOptions | None",
    *,
    is_prepass: bool = False,
) -> None:
    """
    Applies the edits to the data graph (edited nodes are copied before update).
    For the 'prepass' (before validating & applying the edits, with ground truth loaded)
     we do some extra work ensure the edits are in a consistent state.
    """
    trace.get_current_span().set_attribute("edits", len(edits))

    from bench.language.query import DEFAULT_READ_OPTIONS
    from bench.language.value import pack_value_data
    from bench.proto import wiring

    if options is None:
        options = DEFAULT_READ_OPTIONS

    for edit in edits:
        assert edit.epoch is not None, f"missing epoch for {edit!r}"
        edit_type = cast(EditType, edit.type)
        node_type = NodeType(edit.node_ptr.type)
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        node_id = edit.node_ptr.id
        assert node_id is not None, f"missing node id for {edit!r}"

        if edit_type in (EditType.CREATE, EditType.UPSERT) or (
            not options.include_hidden
            and edit_type in (EditType.UNARCHIVE, EditType.RESTORE)
            and not is_prepass
        ):
            # add
            assert edit.new_node_packed, f"missing new node for {edit!r}"
            new_node_data = unpack_node_delta(edit.new_node_packed)
            # inline implicit metadata
            new_node_data.created_at = new_node_data.updated_at = edit.edited_at
            if hasattr(new_node_data, "created_epoch"):
                setattr(new_node_data, "created_epoch", edit.epoch)
                setattr(new_node_data, "updated_epoch", edit.epoch)
            new_node_data.created_by_ptr = new_node_data.updated_by_ptr = edit.subject_ptr
            if edit_type == EditType.CREATE or new_node_data.id not in graph:
                graph.add(new_node_data)
            else:
                graph.update(new_node_data)
        elif (
            edit_type == EditType.DELETE
            or (
                not options.include_hidden and edit_type in (EditType.ARCHIVE, EditType.SOFT_DELETE)
            )
        ) and not is_prepass:
            # remove
            old_node_data = graph.get(node_id)
            assert old_node_data is not None, f"missing node {edit.node_ptr!r} for {edit!r}"
            graph.remove(old_node_data)
        else:
            # update
            # (or any other edit if prepass, where we assume all nodes are loaded in the graph)
            if edit.new_node_packed is not None:
                new_node_data = unpack_node_delta(edit.new_node_packed, node_type=node_type)
            else:
                new_node_data = None
            updated_node_data = graph.get(node_id)
            assert updated_node_data is not None, f"missing node {edit.node_ptr!r} for {edit!r}"
            updated_node_data = wiring.copy_struct(updated_node_data)

            # directly edited properties
            old_node_data = {}
            if edit_type in (EditType.UPDATE, EditType.MOVE):
                for prop_id in edit.properties:
                    prop = node_cls.__properties_by_id__.get(prop_id)
                    assert prop is not None, f"missing property {prop_id} for update: {edit!r}"
                    if prop.reference_wired_ptr is not None:
                        prop = prop.reference_wired_ptr
                    if is_prepass:
                        old_value = getattr(updated_node_data, prop.name)
                        old_node_data[prop.id_as_str], _ = pack_value_data(
                            old_value, prop.as_type_info, wrap_primitive=False
                        )
                    new_value_data = getattr(new_node_data, prop.name)
                    setattr(updated_node_data, prop.name, new_value_data)
                
            # prepass: 'reset' externally provided data to known ground truth (from graph)
            if is_prepass:
                if edit_type in (EditType.UPDATE, EditType.MOVE):
                    # reset only partial old data
                    edit.old_node_packed = wiring.pack_proto_json(old_node_data)
                elif edit_type in (EditType.ARCHIVE, EditType.SOFT_DELETE, EditType.DELETE):
                    # reset full node data as 'old'
                    edit.old_node_packed = pack_node_delta(updated_node_data)
                elif edit_type in (EditType.UNARCHIVE, EditType.RESTORE):
                    # reset full node data as 'new'
                    edit.new_node_packed = pack_node_delta(updated_node_data)

            # implicit metadata
            updated_node_data.updated_at = edit.edited_at
            if "updated_epoch" in node_cls.__properties_by_id__:
                setattr(updated_node_data, "updated_epoch", edit.epoch)
            setattr(updated_node_data, "updated_by_ptr", edit.subject_ptr)
            # Edit.revision may be unset when editing before flushing for validation
            updated_node_data.revision = edit.revision if edit.revision is not None else -1
            if edit_type == EditType.ARCHIVE:
                updated_node_data.archived_at = edit.edited_at
            elif edit_type == EditType.UNARCHIVE:
                updated_node_data.archived_at = None
            elif edit_type == EditType.SOFT_DELETE:
                updated_node_data.deleted_at = edit.edited_at
            elif edit_type == EditType.RESTORE:
                updated_node_data.deleted_at = None

            graph.update(updated_node_data)
