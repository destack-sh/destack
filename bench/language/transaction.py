import dataclasses
from collections import defaultdict
from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, Optional, assert_never, cast
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.connection import Channel, WritableChannel
from bench.language.const import (
    NODE_TYPES,
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
    NODE_SUBSUBTYPE_PROPERTY_BY_TYPE,
    NODE_SUBTYPE_PROPERTY_BY_TYPE,
    ClientOrigin,
    EditSubject,
    GraphScope,
    Node,
    Property,
    Struct,
    struct_,
)
from bench.language.property import p_internal, p_system
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.proto.wire import (
    AnyNodeData,
    ChangeVignetteData,
    ClientOriginData,
    EditContextData,
    EditData,
    GraphScopeData,
    NodeReferenceData,
)
from bench.utils.func import IdEnum
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import (
        Code,
        EditContext,
        Icon,
        Log,
        NodeSuperGraph,
        QueryInfo,
        ReadOptions,
        Session,
    )

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def new_edit_id() -> str:
    return str(UUIDT())


@enum_(EnumType.CHANGE_CATEGORY)
class ChangeCategory(IdEnum):
    """Optional classification for edits."""

    SPACE = 10
    SESSION = 20


@struct_(StructType.CHANGE_VIGNETTE)
class ChangeVignette(Struct):
    """
    A short non-binding summary of key properties at the time just before the edit.
    (so if you rename Block 'A' to 'B', the vignette will say 'A').
    """

    name: str | None = p_system(30, require=False, description="Name of the object.")
    title: str | None = p_system(31, require=False, description="Title of the object.")
    subtype: int | None = p_system(32, require=False, description="Subtype of the object.")
    subsubtype: int | None = p_system(33, require=False, description="Subsubtype of the object.")
    icon: Optional["Icon"] = p_system(
        35, require=False, struct=StructType.ICON, description="Icon of the object."
    )


@struct_(StructType.EDIT)
class Edit(Struct):
    """
    An edit to a Node. Currently, edits are always on the property level (no sub-properties or values).

    For updates/moves, the old/new node values are just the edited properties.
    For archive/delete/erase, the old node is the full node.
      (technically we don't *need* the old node if it's not an erase, but it's very convenient)
    Similarly, for create/upsert/unarchive/restore, the new node is the full node.
    """

    # NOTE: Edit.old_node_partial/new_node_partial :EditData are populated as follows:
    #  (default is old_node_partial=None, new_node_partial=None)
    # EditType.CREATE: new_node_partial = full new node
    # EditType.UPSERT: new_node_partial = full new node
    # EditType.UPDATE: new_node_partial = partial new node, old_node_partial = partial old node
    # EditType.MOVE: new_node_partial = partial new node, old_node_partial = partial old node
    # EditType.ARCHIVE: old_node_partial = full old node (archived_at=None)
    # EditType.UNARCHIVE: old_node_partial = full old node (archived_at=...)
    # EditType.DELETE: old_node_partial = full old node (deleted_at=None)
    # EditType.RESTORE: old_node_partial = full old node (deleted_at=...)
    # EditType.ERASE: old_node_partial = full old node

    # core
    id: UUID = p_system(
        2,
        default_factory=UUIDT,
        require=True,
        description="Unique identifier for the edit within a session.",
    )
    type: EditType = p_system(30, require=True, description="Type of edit.")
    node: Node = p_system(31, require=True, references=NODE_TYPES.tuple, description="Which node.")
    vignette: ChangeVignette | None = p_system(
        32,
        require=False,
        struct=StructType.CHANGE_VIGNETTE,
        description="Summary of the node before the edit.",
    )

    # content
    properties: list[int] = p_system(
        40,
        array=True,
        description="Which non-tracking properties are edited.",
        primitive_type=PrimitiveType.INT16,
    )
    old_node_partial: AnyNodeData | None = p_system(
        41,
        primitive_type=None,
        is_node_data=True,
        description="The previous values for the edited properties (if any.)",
    )
    new_node_partial: AnyNodeData | None = p_system(
        42,
        primitive_type=None,
        is_node_data=True,
        description="The new values for the edited properties (if any.)",
    )

    # meta
    scope: GraphScope = p_system(
        60, require=True, struct=StructType.GRAPH_SCOPE, description="Enclosing scope of the edit."
    )
    change_key: UUID | None = p_system(
        61, require=False, description="The change that this edit is part of."
    )
    category: ChangeCategory | None = p_system(
        62, require=False, description="Optional classification for the edit."
    )
    subject: EditSubject | None = p_system(
        63, require=False, references=EDIT_SUBJECT_TYPES, description="Who made the edit."
    )
    origin: ClientOrigin | None = p_system(
        64, require=False, struct=StructType.CLIENT_ORIGIN, description="Where the edit came from."
    )
    context: "EditContext | None" = p_system(
        65,
        require=False,
        struct=StructType.EDIT_CONTEXT,
        description="Additional per edit context for servers.",
    )
    edited_at: datetime = p_system(66, require=True, description="When the edit was made.")
    revision: int | None = p_system(
        67,
        require=False,
        description="New revision of the edited node.",
        primitive_type=PrimitiveType.INT64,
    )
    epoch: int | None = p_system(
        68,
        require=False,
        description="Epoch at that edit (client if submitting, system if accepted).",
        primitive_type=PrimitiveType.INT64,
    )
    undo_of: Optional["Log"] = p_system(
        69,
        require=False,
        array=False,
        references=NodeType.LOG,
        description="The logged change that is being undon with this edit.",
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
    """
    A change is a sequence of related edits.
    Any edit not associated with a change is implicitly in its own change.
    The 'key' is the id the change will have in the Log.
    """

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
    logs_query: Optional["QueryInfo"] = p_internal(
        36, require=False, array=False, struct=StructType.QUERY_INFO, description="Query for logs."
    )


@dataclasses.dataclass(slots=True)
class EditEvent:
    """
    A tiny representation of an edit we summarize into actual Edits on flush.
    Mainly for 'debouncing' updates/moves into single Edits.
    """

    node: Node
    type: EditType
    subject: NodeReferenceData | None
    origin: ClientOriginData | None
    context: EditContextData | None
    now: datetime
    scope: GraphScopeData
    node_data: AnyNodeData | None = None
    # remember old values (since node is edited in place)
    old_values: dict[int, Any] | None = None

    def __str__(self):
        return f"{self.type.bench_name} {self.node!r}"

    def __repr__(self):
        return f"<EditEvent {self}>"


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
    _edits: list[EditData] = dataclasses.field(default_factory=list)
    _cascaded_edits: list[EditData] = dataclasses.field(default_factory=list)
    _touched_engine_ids: set[Any] = dataclasses.field(default_factory=set)

    """Pending (unflushed) edits."""
    _pending_edits: list[EditEvent] = dataclasses.field(default_factory=list)
    _pending_changed_nodes_by_id: dict[UUID, Node] = dataclasses.field(default_factory=dict)

    def __str__(self):
        return f"[id={self.id}] ({len(self._edits)} edits, {len(self._cascaded_edits)} cascaded, {len(self._pending_edits)} pending)"

    def __repr__(self):
        return f"<Transaction {self}>"

    @property
    def has_edits(self) -> bool:
        return len(self._edits) > 0 or len(self._pending_edits) > 0

    @property
    def has_pending_edits(self) -> bool:
        return len(self._pending_edits) > 0

    #
    # Edits
    #

    def _record_edit(
        self,
        edit_type: EditType,
        node: Node,
        *,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
        properties: Collection[Property] | None = None,
        old_values: dict[int, Any] | None = None,
    ):
        # peephole optimization for successive updates to same node:
        #  if the last edit was also an update to the same node, merge immediately
        if (
            edit_type == EditType.UPDATE
            and len(self._pending_edits) > 0
            and self._pending_edits[-1].node == node
            and self._pending_edits[-1].type == EditType.UPDATE
        ):
            prev_edit = self._pending_edits[-1]
            assert prev_edit.old_values is not None, f"missing old values for {prev_edit!r}"
            assert old_values is not None, f"missing old values for {edit_type} {node!r}"
            for prop_id, old_value in old_values.items():
                if prop_id not in prev_edit.old_values:
                    prev_edit.old_values[prop_id] = old_value
            return

        scope = self.session._get_scope_for_node(node)
        edit = EditEvent(
            node=node,
            type=edit_type,
            subject=subject,
            origin=origin,
            context=context,
            now=now,
            scope=scope,
            old_values=old_values,
        )
        if edit_type not in (EditType.UPDATE, EditType.MOVE):
            edit.node_data = node._to_data()
        self._pending_edits.append(edit)
        self._pending_changed_nodes_by_id[node.id] = node

    def create(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        self._record_edit(
            EditType.CREATE, node, subject=subject, origin=origin, context=context, now=now
        )

    def upsert(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        self._record_edit(
            EditType.UPSERT, node, subject=subject, origin=origin, context=context, now=now
        )

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
        self._record_edit(
            EditType.UPDATE,
            node,
            subject=subject,
            origin=origin,
            context=context,
            properties=properties,
            old_values=old_values,
            now=now,
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
        self._record_edit(
            EditType.MOVE,
            node,
            subject=subject,
            origin=origin,
            context=context,
            properties=properties,
            old_values=old_values,
            now=now,
        )

    def delete(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        self._record_edit(
            EditType.DELETE, node, subject=subject, origin=origin, context=context, now=now
        )

    def restore(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        self._record_edit(
            EditType.RESTORE, node, subject=subject, origin=origin, context=context, now=now
        )

    def archive(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        self._record_edit(
            EditType.ARCHIVE, node, subject=subject, origin=origin, context=context, now=now
        )

    def unarchive(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        self._record_edit(
            EditType.UNARCHIVE, node, subject=subject, origin=origin, context=context, now=now
        )

    def erase(
        self,
        node: Node,
        subject: NodeReferenceData | None,
        origin: ClientOriginData | None,
        context: EditContextData | None,
        now: datetime,
    ):
        self._record_edit(
            EditType.ERASE, node, subject=subject, origin=origin, context=context, now=now
        )

    #
    # Transaction management
    #

    async def open(self):
        pass  # nothing to do?

    @tracer.start_as_current_span("transaction.accumulate_edits")
    def _accumulate_edits(self, edit_events: list[EditEvent]) -> list[EditData]:
        """
        Turns a series of mini edits into real edits, attempting to coalesce them.
        Specifically, we coalesce sequential updates/moves to the same node (coalescing into moves),
         all other edit types are kept separate.
        NOTE :Performance :Architecture: revisit how we coalesce edits in sessions (updates/moves)
         Right now, something like:
          node1.a = 1
          node2.a = 2
          node1.a = 1
         will result in 3 edits, but depending on interpretation, it could be 2. Maybe.
        """
        from bench.proto import wire, wiring

        edits: list[EditData] = []
        batch: list[EditEvent] = []
        for i, edit_event in enumerate(edit_events):
            edit_type = edit_event.type
            node: Node[AnyNodeData] = edit_event.node
            next_edit_event = edit_events[i + 1] if i + 1 < len(edit_events) else None
            batch.append(edit_event)
            if (
                next_edit_event is not None
                and edit_event.node == next_edit_event.node
                and edit_type in (EditType.UPDATE, EditType.MOVE)
                and next_edit_event.type in (EditType.UPDATE, EditType.MOVE)
            ):
                continue  # coalesce

            # make edit
            old_node_partial: AnyNodeData | None = None
            new_node_partial: AnyNodeData | None = None
            properties: list[int] = []
            if edit_type in (EditType.UPDATE, EditType.MOVE):
                # coalesce any move/update sequence into move
                if any(e.type == EditType.MOVE for e in batch):
                    edit_type = EditType.MOVE
                # accumulate old values (keep oldest)
                old_values: dict[int, Any] = {}
                for e in batch:
                    assert e.old_values is not None, f"missing old values for {e!r}"
                    for prop_id, old_value in e.old_values.items():
                        if prop_id not in old_values:
                            old_values[prop_id] = old_value
                properties = list(old_values.keys())
                properties.sort()  # ascending
                # pack old/new
                proto_cls = cast(type[AnyNodeData], wiring.PROTO_CLASS_BY_TYPE[node.metatype])
                old_node_partial = proto_cls(metatype=wire.ObjectType(node.metatype))
                new_node_partial = proto_cls(metatype=wire.ObjectType(node.metatype))
                for prop_id, old_value in old_values.items():
                    prop = node.__properties_by_id__.get(prop_id)
                    assert prop is not None, f"no property {prop_id} for {node!r} in {batch!r}"
                    if prop.reference_wired_ptr is not None:
                        prop = prop.reference_wired_ptr
                    setattr(old_node_partial, prop.name, wiring.pack_object_prop(prop, old_value))
                    new_value = getattr(node, prop.name)
                    setattr(new_node_partial, prop.name, wiring.pack_object_prop(prop, new_value))
            else:
                # see :EditData for Edit.old_node_partial/new_node_partial
                assert edit_event.node_data is not None, f"missing node data for {edit_event!r}"
                if edit_type in (EditType.CREATE, EditType.UPSERT):
                    new_node_partial = edit_event.node_data
                elif edit_type in (EditType.ARCHIVE, EditType.DELETE, EditType.ERASE):
                    old_node_partial = wiring.copy_struct(edit_event.node_data)
                    if edit_type == EditType.ARCHIVE:
                        old_node_partial.archived_at = None
                    elif edit_type == EditType.DELETE:
                        old_node_partial.deleted_at = None
                elif edit_type == EditType.UNARCHIVE:
                    assert edit_event.node_data.archived_at, f"cannot unarchive {node!r}"
                    old_node_partial = edit_event.node_data
                elif edit_type == EditType.RESTORE:
                    assert edit_event.node_data.deleted_at, f"cannot restore {node!r}"
                    old_node_partial = edit_event.node_data
                else:
                    assert_never(edit_type)

            edit = EditData(
                metatype=wire.ObjectType.EDIT,
                id=new_edit_id(),
                type=wiring.pack_enum(EditType, edit_type),
                node_ptr=node._to_plain_ref_data(),
                properties=properties,
                new_node_partial=wiring.wrap_some_node_maybe(new_node_partial),
                old_node_partial=wiring.wrap_some_node_maybe(old_node_partial),
                scope=edit_event.scope,
                origin=edit_event.origin,
                subject_ptr=edit_event.subject,
                context=edit_event.context,
                edited_at=edit_event.now,
            )
            edits.append(edit)
            batch.clear()  # reset

        return edits

    async def _do_flush(self, *, is_commit: bool, _extra_edits: list[EditData] | None = None):
        """Flush any pending edit events."""
        assert self.session is not None, f"no session for {self!r}"
        log = logger.bind(
            edit_events=len(self._pending_edits),
            extra_edits=len(_extra_edits or ()),
            transaction=self,
        )

        # turn pending mini edits into real edits
        edits = self._accumulate_edits(self._pending_edits)
        if _extra_edits:
            edits.extend(_extra_edits)
        log = log.bind(edits=len(edits))
        self._edits.extend(edits)
        self._pending_edits = []

        # assign system epoch if we have one
        # NOTE :Cleanup: handling system epoch in transaction feels clumsy
        if self.session._system_epoch is not None:
            for edit in edits:
                edit.epoch = self.session._system_epoch
                self.session._system_epoch += 1

        # NOTE :Robustness: use :2PC in Transaction.commit? (if there are more than 2 channels)
        #  (in general, all bench source & logs is in the same DB, so inconsistent reads are ok)
        edits_by_engine_id: dict[Any, list[EditData]] = defaultdict(list)
        for edit in edits:
            engine = self.session._get_engine_for(
                edit.scope, NodeType(edit.node_ptr.type), include_hidden=False, is_readonly=False
            )
            edits_by_engine_id[engine.id].append(edit)
        for engine in self.session._engines:
            # prepare edits & channel
            engine_edits = edits_by_engine_id.get(engine.id, [])
            if not (engine_edits or (is_commit and engine.id in self._touched_engine_ids)):
                continue  # nothing to do
            channel = await self.session._get_channel(engine)
            assert isinstance(channel, WritableChannel), f"read-only {channel!r} for {engine!r}"

            # flush/commit
            message = "transaction.commit.engine" if is_commit else "transaction.flush.engine"
            with tracer.start_as_current_span(
                message, attributes={"engine": engine.__class__.__name__}
            ):
                if is_commit:
                    flush = await channel.commit(engine_edits)
                else:
                    flush = await channel.flush(engine_edits)
                log.trace(message, engine=engine, edits=len(engine_edits))
            assert len(flush.revisions or ()) == len(engine_edits), "revisions mismatch"
            for edit, new_revision in zip(engine_edits, cast(list[int], flush.revisions)):
                edit.revision = new_revision
            self._cascaded_edits.extend(flush.cascaded_edits)
            self._touched_engine_ids.add(engine.id)

        # notify nodes on flushed
        for n in self._pending_changed_nodes_by_id.values():
            n._flushed_self()
        self._pending_changed_nodes_by_id.clear()
        log.trace("transaction.commit" if is_commit else "transaction.flush")

    @tracer.start_as_current_span("transaction.flush")
    async def flush(
        self, _extra_edits: list[EditData] | None = None
    ) -> tuple[list[EditData], list[EditData]]:
        """Canonicalizes and flushes any pending edits (without committing)."""
        await self._do_flush(is_commit=False, _extra_edits=_extra_edits)
        return self._edits, self._cascaded_edits

    @tracer.start_as_current_span("transaction.commit")
    async def commit(
        self, _extra_edits: list[EditData] | None = None
    ) -> tuple[list[EditData], list[EditData]]:
        """Commits the transaction (also flushing any pending edits)."""
        await self._do_flush(is_commit=True, _extra_edits=_extra_edits)
        edits, cascaded_edits = self._edits, self._cascaded_edits
        self._edits = []
        self._cascaded_edits = []
        self._touched_engine_ids.clear()
        return edits, cascaded_edits

    async def reset(self):
        """Resets the transaction, any edits and channels (without closing)."""
        self._edits = []
        self._cascaded_edits = []
        self._pending_edits = []
        self._pending_changed_nodes_by_id.clear()
        self._touched_engine_ids.clear()


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
        node_id = UUID(edit.node_ptr.id)

        if edit_type in (EditType.CREATE, EditType.UPSERT) or (
            not options.include_hidden and edit_type in (EditType.UNARCHIVE, EditType.RESTORE)
        ):
            if edit_type in (EditType.CREATE, EditType.UPSERT):
                assert edit.new_node_partial, f"missing new node for {edit!r}"
                new_node_data = wiring.unwrap_some_node(edit.new_node_partial)
            else:
                assert edit.old_node_partial, f"missing old node for {edit!r}"
                new_node_data = wiring.unwrap_some_node(edit.old_node_partial)
                if edit_type == EditType.UNARCHIVE:
                    new_node_data.archived_at = None
                elif edit_type == EditType.RESTORE:
                    new_node_data.deleted_at = None
                else:
                    assert_never(edit_type)
            # inline implicit metadata
            new_node_data.created_at = new_node_data.updated_at = edit.edited_at
            if hasattr(new_node_data, "created_epoch"):
                setattr(new_node_data, "created_epoch", edit.epoch)
                setattr(new_node_data, "updated_epoch", edit.epoch)
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
            assert edit.new_node_partial, f"missing new node for {edit!r}"
            new_node_data = wiring.unwrap_some_node(edit.new_node_partial)
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
            node._do_set(
                "updated_by_ptr",
                wiring.unpack_object_prop(
                    Node.get_property("updated_by"), edit.subject_ptr, supergraph=supergraph
                ),
                track=track,
            )
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
    from bench.proto import wire, wiring

    if options is None:
        options = DEFAULT_READ_OPTIONS

    def _make_vignette(node: AnyNodeData) -> ChangeVignetteData:
        node_subtype = NODE_SUBTYPE_PROPERTY_BY_TYPE.get(cast(NodeType, node.metatype))
        node_subsubtype = NODE_SUBSUBTYPE_PROPERTY_BY_TYPE.get(cast(NodeType, node.metatype))
        vignette = ChangeVignetteData(
            metatype=wire.ObjectType.CHANGE_VIGNETTE,
            name=getattr(node, "name", None),
            title=getattr(node, "title", None),
            subtype=getattr(node, node_subtype, None) if node_subtype else None,
            subsubtype=getattr(node, node_subsubtype, None) if node_subsubtype else None,
            icon=getattr(node, "icon", None),
        )
        return vignette

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
            if edit_type in (EditType.CREATE, EditType.UPSERT):
                assert edit.new_node_partial, f"missing new node for {edit!r}"
                new_node_data = wiring.unwrap_some_node(edit.new_node_partial)
            else:
                assert edit.old_node_partial, f"missing old node for {edit!r}"
                new_node_data = wiring.unwrap_some_node(edit.old_node_partial)
                if edit_type == EditType.UNARCHIVE:
                    new_node_data.archived_at = None
                elif edit_type == EditType.RESTORE:
                    new_node_data.deleted_at = None
                else:
                    assert_never(edit_type)
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

            # prepass: update new_node_partial and make vignette with new data
            if is_prepass:
                edit.old_node_partial = None
                edit.new_node_partial = wiring.wrap_some_node(new_node_data)  # :EditData
                edit.vignette = _make_vignette(new_node_data)
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
            if edit.new_node_partial is not None:
                new_node_data = wiring.unwrap_some_node(edit.new_node_partial)
            else:
                new_node_data = None
            updated_node_data = graph.get(node_id)
            assert updated_node_data is not None, f"missing node {edit.node_ptr!r} for {edit!r}"
            updated_node_data = wiring.copy_struct(updated_node_data)

            # prepass: make vignette with old data
            if is_prepass:
                edit.vignette = _make_vignette(updated_node_data)

            # directly edited properties
            proto_cls = wiring.PROTO_CLASS_BY_TYPE[node_cls.metatype]
            old_node_partial = proto_cls(metatype=wire.ObjectType(node_cls.metatype))
            if edit_type in (EditType.UPDATE, EditType.MOVE):
                for prop_id in edit.properties:
                    prop = node_cls.__properties_by_id__.get(prop_id)
                    assert prop is not None, f"missing property {prop_id} for update: {edit!r}"
                    if prop.reference_wired_ptr is not None:
                        prop = prop.reference_wired_ptr
                    if is_prepass:
                        old_value = getattr(updated_node_data, prop.name)
                        setattr(old_node_partial, prop.name, old_value)
                    new_value_data = getattr(new_node_data, prop.name)
                    setattr(updated_node_data, prop.name, new_value_data)

            # prepass: 'reset' externally provided data to known ground truth (from graph)
            if is_prepass:  # :EditData
                if edit_type in (EditType.UPDATE, EditType.MOVE):
                    edit.old_node_partial = wiring.wrap_some_node(
                        cast(AnyNodeData, old_node_partial)
                    )
                elif edit_type in (EditType.ARCHIVE, EditType.DELETE, EditType.ERASE):
                    edit.old_node_partial = wiring.wrap_some_node(
                        wiring.copy_struct(updated_node_data)
                    )
                    if edit_type == EditType.ARCHIVE:
                        edit.old_node_partial.archived_at = None
                    elif edit_type == EditType.DELETE:
                        edit.old_node_partial.deleted_at = None
                elif edit_type in (EditType.UNARCHIVE, EditType.RESTORE):
                    edit.old_node_partial = wiring.wrap_some_node(
                        wiring.copy_struct(updated_node_data)  # keep archived_at/deleted_at
                    )

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
