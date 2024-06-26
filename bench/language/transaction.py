import dataclasses
from collections import defaultdict
from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, Literal, Optional, cast
from uuid import UUID

import structlog
from betterproto.lib.google.protobuf import Struct as ProtoStruct
from opentelemetry import trace

from bench.language.connection import (
    Channel,
    GraphEngine,
    WritableChannel,
)
from bench.language.const import (
    NODE_TYPES,
    UNSET,
    EditType,
    EnumType,
    NodeType,
    PrimitiveType,
    StructType,
    enum_,
)
from bench.language.graph import NodeDataGraph, NodeGraph
from bench.language.node import (
    EDIT_SUBJECT_TYPES,
    ClientOrigin,
    EditSubject,
    GraphScope,
    InlineStruct,
    Node,
    Property,
    Struct,
    struct_,
)
from bench.language.property import p_internal, p_system
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.proto.wire import (
    AnyNodeData,
    ClientOriginData,
    EditContextData,
    EditData,
    NodeReferenceData,
)
from bench.utils.func import IdEnum
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import (
        Code,
        EditContext,
        Expression,
        Log,
        NodeSuperGraph,
        ReadOptions,
        Session,
    )

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def new_edit_id() -> str:
    return str(UUIDT())


@struct_(StructType.EDIT, inline=True)
class Edit(InlineStruct):
    """
    An edit to a Node.
    For updates/moves, the old/new node values are just the edited properties.
    For 'remove's, the old node is the full node.
      (technically we don't *need* if for non-hard deletes, but it's very convenient)
    Similarly, for 'adds', the new node is the full node.
    """

    # content
    id: UUID = p_system(
        2,
        default_factory=UUIDT,
        require=True,
        description="Unique identifier for the edit within a session.",
    )
    type: EditType = p_system(30, require=True, description="Type of edit.")
    node: Node = p_system(31, require=True, references=NODE_TYPES.tuple, description="Which node.")
    properties: list[int] = p_system(
        32, array=True, description="Which non-tracking properties are edited."
    )
    old_node_packed: Any | None = p_system(
        33,
        primitive_type=PrimitiveType.JSON,
        description="The previous values for the edited properties (if any.)",
    )
    new_node_packed: Any | None = p_system(
        34,
        primitive_type=PrimitiveType.JSON,
        description="The new values for the edited properties (if any.)",
    )

    # context
    scope: GraphScope = p_system(
        40, require=True, struct=StructType.GRAPH_SCOPE, description="Enclosing scope of the edit."
    )
    change_key: UUID | None = p_system(
        41, require=False, description="The change that this edit is part of."
    )
    subject: EditSubject | None = p_system(
        42, require=False, references=EDIT_SUBJECT_TYPES, description="Who made the edit."
    )
    origin: ClientOrigin | None = p_system(
        43, require=False, struct=StructType.CLIENT_ORIGIN, description="Where the edit came from."
    )
    context: "EditContext | None" = p_system(
        44,
        require=False,
        struct=StructType.EDIT_CONTEXT,
        description="Additional per edit context for servers.",
    )
    edited_at: datetime = p_system(45, require=True, description="When the edit was made.")
    revision: int | None = p_system(
        46, require=False, description="New revision of the edited node."
    )
    epoch: int | None = p_system(
        47,
        require=False,
        description="Epoch at that edit (client if submitting, system if accepted).",
    )


@enum_(EnumType.CHANGE_KIND)
class ChangeKind(IdEnum):
    """The kind of Change."""

    CODE = 1
    MATERIALIZED = 2
    LOGS = 3
    LOGS_QUERY = 4


@struct_(StructType.CHANGE)
class Change(Struct):
    """A change is a sequence of related edits."""

    key: UUID = p_system(30, default_factory=UUIDT)
    kind: ChangeKind = p_internal(31, require=True, description="The kind of change.")
    scope: Optional["Node"] = p_internal(32, require=False, references=NODE_TYPES.tuple)
    code: Optional["Code"] = p_internal(33, require=False, array=False, struct=StructType.CODE)
    edits: list[Edit] = p_internal(
        34, array=True, struct=StructType.EDIT, description="The materialized edits in the change."
    )
    logs: list["Log"] = p_internal(
        35, require=False, array=True, references=NodeType.LOG, description="Logs for the change."
    )
    logs_filter: Optional["Expression"] = p_internal(
        36, require=False, array=False, struct=StructType.EXPRESSION, description="Filter for logs."
    )


@dataclasses.dataclass(slots=True)
class Transaction:
    """
    A transaction is an atomic list of Edits (may be multiple sets of changes).
    """

    id: UUID
    session: "Session"
    is_readonly: bool = dataclasses.field(default=False)
    _split_read_channel: Optional[Channel] = dataclasses.field(default=None)
    _channels: list[Channel] = dataclasses.field(default_factory=list)

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

    def __str__(self):
        return f"[id={self.id}] ({len(self.edits)} edits, {len(self.cascaded_edits)} cascaded, {len(self.pending_edits)} pending)"

    def __repr__(self):
        return f"<Transaction {self}>"

    @property
    def has_edits(self) -> bool:
        return len(self.edits) > 0

    @property
    def has_pending_edits(self) -> bool:
        return any(self._pending_edits_by_engine_id.values())

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
            EditType.DELETE,
            EditType.RESTORE,
            EditType.ERASE,
        ],
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ) -> EditData:
        """Creates a simple non-update/move edit and adds it to the pending edits."""
        from bench.proto import wiring

        assert self.session is not None, f"no session for {self!r}"
        if self.is_readonly:
            raise RuntimeError(
                f"cannot {edit_type.bench_name} {node!r} in read-only {self.session}"
            )

        # pack 'old' and 'new' node deltas
        old_node_packed = None
        new_node_packed = None
        if edit_type in (EditType.CREATE, EditType.UPSERT):
            new_node_packed = pack_node_delta(node._to_data())
        elif edit_type in (EditType.ARCHIVE, EditType.DELETE, EditType.ERASE):
            old_node_packed = pack_node_delta(node._to_data())
        elif edit_type == EditType.UNARCHIVE:
            # put old archived_at in 'old', put full restored node in new
            assert node.archived_at, f"cannot unarchive {node!r} that is not archived"
            new_node = node._to_data()
            old_node_packed = pack_node_delta(new_node, only=(type(node).archived_at,))
            new_node.archived_at = None
            new_node_packed = pack_node_delta(new_node)
        elif edit_type == EditType.RESTORE:
            # put old deleted_at in 'old', put full restored node in new
            assert node.deleted_at, f"cannot restore {node!r} that is not deleted"
            new_node = node._to_data()
            old_node_packed = pack_node_delta(new_node, only=(type(node).deleted_at,))
            new_node.deleted_at = None
            new_node_packed = pack_node_delta(new_node)
        else:
            raise ValueError(f"unexpected edit type for {node!r}: {edit_type.name}")

        # make edit
        edit = EditData(
            id=new_edit_id(),
            type=wiring.pack_enum(EditType, edit_type),
            node_ptr=node._to_ref_data(),
            new_node_packed=new_node_packed,
            old_node_packed=old_node_packed,
            scope=self.session._get_scope_for_node(node),
            origin=origin,
            subject_ptr=subject,
            context=context,
            edited_at=now,
        )
        return edit

    def create(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        edit = self._make_simple_edit(
            EditType.CREATE, node, subject=subject, origin=origin, context=context, now=now
        )
        self._add_pending_edit(edit, node)

    def upsert(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        edit = self._make_simple_edit(
            EditType.UPSERT, node, subject=subject, origin=origin, context=context, now=now
        )
        self._add_pending_edit(edit, node)

    def _do_update(
        self,
        edit_type: Literal[EditType.UPDATE, EditType.MOVE],
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        properties: Collection[Property],
        old_values: dict[int, Any],
        now: datetime,
    ):
        """Update or move a node."""
        from bench.language.value import pack_value
        from bench.proto import wiring

        existing_edit_idx = self._pending_updates_idx.get(node)
        if existing_edit_idx is None:
            # new update/move
            old_node_packed = {}
            new_node_packed = {}
            for prop in properties:
                if prop.reference_wired_ptr is not None:
                    prop = prop.reference_wired_ptr
                old_value = old_values.get(prop.id, UNSET)
                assert old_value is not UNSET, f"missing old value for {prop!r} in {node!r}"
                prop_type = prop.type_info
                old_node_packed[prop.id_as_str], _ = pack_value(
                    old_value, prop_type, wrap_primitive=False
                )
                new_value = getattr(node, prop.name)
                new_node_packed[prop.id_as_str], _ = pack_value(
                    new_value, prop_type, wrap_primitive=False
                )
            edit = EditData(
                id=new_edit_id(),
                type=wiring.pack_enum(EditType, edit_type),
                node_ptr=node._to_ref_data(),
                properties=[prop.id for prop in properties],
                old_node_packed=wiring.pack_proto_json(old_node_packed),
                new_node_packed=wiring.pack_proto_json(new_node_packed),
                scope=self.session._get_scope_for_node(node),
                subject_ptr=subject,
                origin=origin,
                context=context,
                edited_at=now,
            )
            engine = self._add_pending_edit(edit, node)
            edit_idx = len(self._pending_edits_by_engine_id[engine.id]) - 1
            self._pending_updates_idx[node] = engine.id, edit_idx
        else:
            # update existing edit in place ('debounce') :DebouncedUpdate
            # NOTE :Performance: unpacking/repacking proto json is obviously inefficient
            assert node._updated_properties is not None, f"missing property mask for {node!r}"
            engine_id, current_update_idx = existing_edit_idx
            edit = self._pending_edits_by_engine_id[engine_id][current_update_idx]
            edit.properties = list(node._unmask_properties_ids(node._updated_properties))
            assert edit.new_node_packed and edit.old_node_packed, f"missing node data for {edit!r}"
            new_node_packed = wiring.unpack_proto_json(edit.new_node_packed)
            old_node_packed = wiring.unpack_proto_json(edit.old_node_packed)
            for prop in properties:
                if prop.reference_wired_ptr is not None:
                    prop = prop.reference_wired_ptr
                if prop.id_as_str not in old_node_packed:
                    # add old value if it doesn't already exist
                    old_value = old_values.get(prop.id, UNSET)
                    assert old_value is not UNSET, f"missing old value for {prop!r} in {node!r}"
                    old_node_packed[prop.id_as_str], _ = pack_value(
                        old_value, prop.type_info, wrap_primitive=False
                    )
                # and update new value
                new_value = getattr(node, prop.name)
                new_node_packed[prop.id_as_str], _ = pack_value(
                    new_value, prop.type_info, wrap_primitive=False
                )
            edit.new_node_packed = wiring.pack_proto_json(new_node_packed)
            edit.old_node_packed = wiring.pack_proto_json(old_node_packed)
            if edit_type == EditType.MOVE and edit.type != EditType.MOVE:
                edit.type = wiring.pack_enum(EditType, EditType.MOVE)
            edit.edited_at = now

    def update(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        properties: Collection[Property],
        old_values: dict[int, Any],
        now: datetime,
    ):
        self._do_update(
            EditType.UPDATE, node, subject, origin, context, properties, old_values, now
        )

    def move(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        properties: Collection[Property],
        old_values: dict[int, Any],
        now: datetime,
    ):
        self._do_update(EditType.MOVE, node, subject, origin, context, properties, old_values, now)

    def delete(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        edit = self._make_simple_edit(
            EditType.DELETE, node, subject=subject, origin=origin, context=context, now=now
        )
        self._add_pending_edit(edit, node)

    def restore(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        edit = self._make_simple_edit(
            EditType.RESTORE, node, subject=subject, origin=origin, context=context, now=now
        )
        self._add_pending_edit(edit, node)

    def archive(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        edit = self._make_simple_edit(
            EditType.ARCHIVE, node, subject=subject, origin=origin, context=context, now=now
        )
        self._add_pending_edit(edit, node)

    def unarchive(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        edit = self._make_simple_edit(
            EditType.UNARCHIVE,
            node=node,
            subject=subject,
            origin=origin,
            context=context,
            now=now,
        )
        self._add_pending_edit(edit, node)

    def erase(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        edit = self._make_simple_edit(
            EditType.ERASE, node, subject=subject, origin=origin, context=context, now=now
        )
        self._add_pending_edit(edit, node)

    #
    # Transaction management
    #

    async def open(self):
        pass

    def _add_pending_edit(self, edit: EditData, node: Optional[Node]) -> GraphEngine:
        """Adds an edit to the pending (unflushed, uncommitted)"""
        from bench.proto import wiring

        node_type = wiring.unpack_enum(NodeType, edit.node_ptr.type)
        engine = self.session._get_engine_for(edit.scope, node_type, is_readonly=False)
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

        # TODO :Robustness!: use :2PC in Transaction.commit (if there are more than 2 channels)
        for engine in self.session._engines:
            # prepare edits & channel
            pending_edits = self._pending_edits_by_engine_id.get(engine.id, [])
            if not (pending_edits or (commit and engine.id in self._used_engine_ids)):
                continue  # nothing to do
            channel = await self.session._get_channel(engine)
            assert isinstance(channel, WritableChannel), f"read-only {channel!r} for {engine!r}"

            # flush/commit
            message = "transaction.commit.engine" if commit else "transaction.flush.engine"
            with tracer.start_as_current_span(
                message, attributes={"engine": engine.__class__.__name__}
            ):
                if commit:
                    flush = await channel.commit(pending_edits)
                else:
                    flush = await channel.flush(pending_edits)
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

    async def reset(self):
        """Resets the transaction, any edits and channels (without closing)."""
        self.edits.clear()
        self.cascaded_edits.clear()
        self.pending_edits.clear()
        self._pending_edits_by_engine_id.clear()
        self._pending_updates_idx.clear()
        self._pending_nodes_by_ck.clear()
        self._used_engine_ids.clear()


def pack_node_delta(
    node_data: AnyNodeData, *, only: Collection[Property | Any] | None = None
) -> ProtoStruct:
    """Packs a node into its edit representation. If 'only' is set, only those properties are packed."""
    from bench.language.value import pack_builtin_object_data

    node_packed = pack_builtin_object_data(node_data, only=only)
    return ProtoStruct.from_dict(node_packed)  # type: ignore


def unpack_node_delta(
    node_packed: dict[str, Any] | ProtoStruct,
    *,
    node_type: NodeType | None = None,
    only: Collection[Property | Any] | None = None,
) -> AnyNodeData:
    """Unpacks a node from its packed edit representation. If 'only' is set, only those properties are unpacked."""
    from bench.language.value import unpack_builtin_object_data
    from bench.proto import wiring

    if not isinstance(node_packed, dict):
        node_packed = node_packed.to_dict()

    proto_cls = wiring.PROTO_CLASS_BY_TYPE[node_type] if node_type is not None else None
    node_data = unpack_builtin_object_data(node_packed, expect=proto_cls, only=only)
    return cast(AnyNodeData, node_data)


@tracer.start_as_current_span("graph.edit_graph")
def edit_graph(
    graph: NodeGraph,
    supergraph: "NodeSuperGraph",
    edits: Collection[EditData],
    options: "ReadOptions | None",
    *,
    track: bool,
    validate: bool,
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
            node = wiring.unpack_object(
                new_node_data, graph=graph, supergraph=supergraph, expect=Node
            )
            if edit_type == EditType.CREATE or node.id not in graph:
                graph.add(node)
            else:
                graph.update(node)
        elif edit_type == EditType.ERASE or (
            not options.include_hidden and edit_type in (EditType.ARCHIVE, EditType.DELETE)
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
                new_value = wiring.unpack_object_prop(prop, new_value_data, supergraph=supergraph)
                node._do_set(prop.name, new_value, track=track)
            # implicit metadata
            node.updated_at = edit.edited_at
            if "updated_epoch" in node.__properties__:
                node._do_set("updated_epoch", edit.epoch, track=track)
            node._do_set("updated_by_ptr", edit.subject_ptr, track=track)
            node._do_set("revision", edit.revision, track=track)
            if edit_type == EditType.ARCHIVE:
                node._do_set("archived_at", edit.edited_at, track=track)
            elif edit_type == EditType.UNARCHIVE:
                node._do_set("archived_at", None, track=track)
            elif edit_type == EditType.DELETE:
                node._do_set("deleted_at", edit.edited_at, track=track)
            elif edit_type == EditType.RESTORE:
                node._do_set("deleted_at", None, track=track)
            elif edit_type == EditType.MOVE:
                if new_node_data.parent_ptr is not None:
                    new_parent = graph.get(UUID(new_node_data.parent_ptr.id))
                else:
                    new_parent = None
                node._do_set("parent", new_parent, track=track, validate=validate)
            graph.update(node)


@tracer.start_as_current_span("graph.edit_data_graph")
def edit_data_graph(
    graph: NodeDataGraph,
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
            edit_type == EditType.ERASE
            or (not options.include_hidden and edit_type in (EditType.ARCHIVE, EditType.DELETE))
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
                            old_value, prop.type_info, wrap_primitive=False
                        )
                    new_value_data = getattr(new_node_data, prop.name)
                    setattr(updated_node_data, prop.name, new_value_data)

            # prepass: 'reset' externally provided data to known ground truth (from graph)
            if is_prepass:
                if edit_type in (EditType.UPDATE, EditType.MOVE):
                    # reset only partial old data
                    edit.old_node_packed = wiring.pack_proto_json(old_node_data)
                elif edit_type in (EditType.ARCHIVE, EditType.DELETE, EditType.ERASE):
                    # reset full node data as 'old'
                    edit.old_node_packed = pack_node_delta(updated_node_data)
                elif edit_type in (EditType.UNARCHIVE, EditType.RESTORE):
                    # reset full node data as 'new'
                    edit.new_node_packed = pack_node_delta(updated_node_data)

            # implicit metadata
            updated_node_data.updated_at = edit.edited_at
            if "updated_epoch" in node_cls.__properties__:
                setattr(updated_node_data, "updated_epoch", edit.epoch)
            setattr(updated_node_data, "updated_by_ptr", edit.subject_ptr)
            # Edit.revision may be unset when editing before flushing for validation
            updated_node_data.revision = edit.revision if edit.revision is not None else -1
            if edit_type == EditType.ARCHIVE:
                updated_node_data.archived_at = edit.edited_at
            elif edit_type == EditType.UNARCHIVE:
                updated_node_data.archived_at = None
            elif edit_type == EditType.DELETE:
                updated_node_data.deleted_at = edit.edited_at
            elif edit_type == EditType.RESTORE:
                updated_node_data.deleted_at = None

            graph.update(updated_node_data)
