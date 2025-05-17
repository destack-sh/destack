import dataclasses
from collections import defaultdict
from datetime import datetime
from typing import TYPE_CHECKING, Any, Callable, Collection, Optional, Sequence, assert_never, cast
from uuid import UUID

import pytz
import structlog
from google.protobuf.timestamp_pb2 import Timestamp
from opentelemetry import trace

from bench import pb2
from bench.language.connection import Connector, WritableConnector
from bench.language.core import (
    EditOperationType,
    EditType,
    Node,
    NodeDataGraph,
    NodeGraph,
    NodeReference,
    NodeType,
    Scope,
    Struct,
    StructType,
    Subject,
    p_regular,
    p_system,
    pack_value_data,
    struct_,
)
from bench.language.registry import NODE_CLASS_BY_TYPE
from bench.pb2 import (
    AnyNodeData,
    ChangeVignetteData,
    ClientOriginData,
    EditContextData,
    EditData,
    EditOperationData,
    GraphScopeData,
)
from bench.utils.func import partition
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import (
        ChangeCategory,
        ClientOrigin,
        Icon,
        NodeSuperGraph,
        Run,
        Session,
    )

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def new_edit_id() -> str:
    return str(UUIDT())


@struct_(StructType.CHANGE_VIGNETTE)
class ChangeVignette(Struct):
    """
    A short non-binding summary of key properties at the time just before the edit.
    (so if you rename Block 'A' to 'B', the vignette will say 'A').
    """

    name: str | None = p_system(30, description="Name of the object.")
    title: str | None = p_system(31, description="Title of the object.")
    subtype: int | None = p_system(32, description="Subtype of the object.")
    icon: Optional["Icon"] = p_system(35, description="Icon of the object.")


@struct_(StructType.EDIT_OPERATION)
class EditOperation(Struct):
    """An edit operation."""

    type: EditOperationType = p_system(30, default=EditOperationType.SET)
    key: str = p_system(31)

    new_value_packed: Any | None = p_regular(40)


@struct_(StructType.EDIT)
class Edit(Struct):
    """
    An Edit to a Node.
    """

    # core
    id: str = p_system(
        2,
        default_factory=new_edit_id,
        description="Unique identifier for the Edit within a Session.",
    )
    type: EditType = p_system(30, description="Type of Edit.")
    node: Node = p_system(31, description="Which Node.")
    edited_at: datetime = p_system(33, description="When the Edit was made.")
    old_edited_at: Optional[datetime] = p_system(
        34,
        default=None,
        description="The timestamp of the Edit being undone with this Edit.",
    )

    # content
    node_data: AnyNodeData | None = p_system(40, primitive_type=None, is_node_data=True)
    operations: list[EditOperation] = p_system(
        41,
        description="The operations to perform on the Node",
    )

    # meta
    scope: Scope = p_system(60, description="Enclosing scope of the Edit.")
    change_key: UUID | None = p_system(61, description="The Change that this Edit is part of.")
    category: "ChangeCategory | None" = p_system(
        62, description="Optional classification for the Edit."
    )
    subject: Optional[Subject] = p_system(63, description="Who made the Edit.")
    origin: "ClientOrigin | None" = p_system(64, description="Where the Edit came from.")


@dataclasses.dataclass(slots=True)
class EditEvent:
    """
    A compact representation of an Edit that we summarize into actual Edits on flush.
    """

    node: Node
    type: EditType
    subject_ptr: NodeReference | None
    origin: ClientOriginData | None
    run: "Run | None"
    now: datetime
    scope: GraphScopeData
    node_data: AnyNodeData | None = None
    operations: list[EditOperationData] | None = None

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
    _split_read_connector: Optional[Connector] = dataclasses.field(default=None)
    _connectors: list[Connector] = dataclasses.field(default_factory=list)

    """All edits from this transaction (since the previous commit)."""
    _edits: list[EditData] = dataclasses.field(default_factory=list)
    _cascaded_edits: list[EditData] = dataclasses.field(default_factory=list)
    _touched_engine_ids: set[Any] = dataclasses.field(default_factory=set)

    """Pending (unflushed) edits."""
    # individual edit events, merged into edits on flush
    _pending_edit_events: list[EditEvent] = dataclasses.field(default_factory=list)
    # manual edits (or pre-accumulated edits from edit events)
    _pending_edits: list[EditData] = dataclasses.field(default_factory=list)

    def __str__(self):
        return f"[id={self.id}] ({len(self._edits)} edits, {len(self._cascaded_edits)} cascaded, {len(self._pending_edit_events) + len(self._pending_edits)} pending)"

    def __repr__(self):
        return f"<Transaction {self}>"

    @property
    def has_edits(self) -> bool:
        return len(self._edits) > 0 or self.has_pending_edits

    @property
    def has_pending_edits(self) -> bool:
        return len(self._pending_edit_events) > 0 or len(self._pending_edits) > 0

    #
    # Transaction management
    #

    def record_edit_event(
        self,
        type: EditType,
        node: Node,
        *,
        now: datetime | None = None,
        operation: EditOperationData | None = None,
    ):
        """
        Records an edit event (which are later summed into actual edits).
        We try to be efficient and record minimal information quickly and only as needed.
        """

        # peephole optimization for successive updates to same node:
        #  if the last edit was also an update to the same node, merge immediately
        if (
            (type == EditType.UPDATE or type == EditType.MOVE)
            and len(self._pending_edit_events) > 0
            and self._pending_edit_events[-1].node.id == node.id
            and (
                self._pending_edit_events[-1].type == EditType.UPDATE
                or self._pending_edit_events[-1].type == EditType.MOVE
            )
        ):
            # merge operation
            prev_edit = self._pending_edit_events[-1]
            assert prev_edit.operations is not None, f"missing operations for {prev_edit!r}"
            assert operation is not None, f"missing operation for {prev_edit!r}"
            prev_edit.operations.append(operation)
            # fire subscriptions
            subs = self.session._on_edit_subs.get(node.id)
            if subs is not None:
                for sub in subs:
                    sub(node)
            return  # already handled

        # context
        session = self.session
        if now is None:
            now = session._oracle.utc()
        run = session._runtime.active_run if session._runtime is not None else None
        if run is not None and run.agent_ptr is not None:
            subject_ptr = run.agent_ptr
        elif session._subject is not None:
            subject_ptr = session._subject.to_ref()
        else:
            subject_ptr = None

        # make edit event
        scope = session._get_scope_for_node(node)
        edit_event = EditEvent(
            node=node,
            type=type,
            subject_ptr=subject_ptr,
            origin=session._origin,
            run=run,
            now=now,
            scope=scope,
        )
        if operation is not None:
            edit_event.operations = [operation]
        if type in (
            EditType.ARCHIVE,
            EditType.UNARCHIVE,
            EditType.DELETE,
            EditType.RESTORE,
            EditType.ERASE,
        ):
            node_data = node._to_data()
            if node._is_new:
                # find previous create event and set node_data now to 'fresh' node
                for e in self._pending_edit_events:
                    if e.node == node and e.type == EditType.CREATE:
                        edit_event.node_data = node_data
                        break
            edit_event.node_data = node_data
            node._is_new = False
        self._pending_edit_events.append(edit_event)

        # fire subscriptions
        subs = self.session._on_edit_subs.get(node.id)
        if subs is not None:
            for sub in subs:
                sub(node)

    def _add_pending_edits(self, edits: Sequence[EditData]):
        """Adds full edits to the transaction directly."""
        self._pending_edits.extend(edits)

    def _track_edits(self, edits: Sequence[EditData]):
        """Tracks edits in our logical clock (local epoch)."""
        # assign local epoch if we have one
        epoch = self.session._local_epoch
        assert epoch is not None, f"no local epoch for {self.session!r}"
        for edit in edits:
            edit.epoch = epoch
            epoch += 1
        self.session._local_epoch = epoch

    @tracer.start_as_current_span("transaction.accumulate_edits")
    def _accumulate_edits(
        self, edit_events: list[EditEvent], *, filter: Callable[[EditEvent], bool] | None = None
    ) -> tuple[list[EditEvent], list[EditData]]:
        """
        Turns a series of mini edits into real edits, attempting to coalesce them.
        We only merge sequential updates/moves to the same node (coalescing into moves).
        """
        from bench.proto import wiring

        span = trace.get_current_span()
        span.set_attribute("edit_events", len(edit_events))
        span.set_attribute("filter", filter is not None)

        edits: list[EditData] = []
        batch: list[EditEvent] = []

        # filter
        if filter is not None:
            unconsumed_edit_events, edit_events = partition(filter, edit_events)
            span.set_attribute("filtered_edit_events", len(edit_events))
        else:
            unconsumed_edit_events = []

        for i, edit_event in enumerate(edit_events):
            edit_type = edit_event.type
            node: Node[AnyNodeData] = edit_event.node
            next_edit_event = edit_events[i + 1] if i + 1 < len(edit_events) else None
            batch.append(edit_event)

            # coalesce update/move edits
            if (
                next_edit_event is not None
                and edit_event.node == next_edit_event.node
                and edit_type in (EditType.UPDATE, EditType.MOVE)
                and next_edit_event.type in (EditType.UPDATE, EditType.MOVE)
            ):
                continue

            # edit data
            node_data: AnyNodeData | None = None
            old_edited_at: Timestamp | None = None
            operations: list[EditOperationData] | None = None
            if edit_type in (EditType.UPDATE, EditType.MOVE):
                # coalesce any move/update sequence into move
                if any(e.type == EditType.MOVE for e in batch):
                    edit_type = EditType.MOVE
                # accumulate old values (keep oldest)
                operations = edit_event.operations
                assert operations, f"missing operations for {edit_event!r}"
                for e in batch[1:]:
                    assert e.operations, f"missing operations for {e!r}"
                    operations.extend(e.operations)
            elif edit_type in (EditType.CREATE, EditType.UPSERT):
                node_data = edit_event.node_data or edit_event.node._to_data()
            elif edit_type in (EditType.ARCHIVE, EditType.UNARCHIVE):
                assert edit_event.node_data is not None, f"missing node data for {edit_event!r}"
                old_edited_at = edit_event.node_data.archived_at
                node_data = edit_event.node_data
            elif edit_type in (
                EditType.DELETE,
                EditType.ERASE,
                EditType.RESTORE,
            ):
                assert edit_event.node_data is not None, f"missing node data for {edit_event!r}"
                old_edited_at = edit_event.node_data.deleted_at
                node_data = edit_event.node_data
            else:
                assert_never(edit_type)

            # context
            if edit_event.run is not None:
                edit_context = EditContextData(metatype=pb2.OBJECT_TYPE_EDIT_CONTEXT)
                run = edit_event.run
                edit_context.run_ptr.CopyFrom(run._to_ref_data())
                if run.root_ptr is not None:
                    edit_context.run_root_ptr.CopyFrom(run.root_ptr._to_data())
                if run.flow_ptr is not None:
                    edit_context.flow_ptr.CopyFrom(run.flow_ptr._to_data())
                if run.action_ptr is not None:
                    edit_context.action_ptr.CopyFrom(run.action_ptr._to_data())
            else:
                edit_context = None
            if edit_event.subject_ptr is not None:
                subject_ptr = edit_event.subject_ptr._to_data()
            else:
                subject_ptr = None

            # make edit
            edited_at = Timestamp()
            edited_at.FromDatetime(edit_event.now)
            edit = EditData(
                metatype=pb2.ObjectType.OBJECT_TYPE_EDIT,
                id=new_edit_id(),
                type=wiring.pack_enum(EditType, edit_type),
                node_ptr=node._to_ref_data(),
                scope=edit_event.scope,
                operations=operations,
                origin=edit_event.origin,
                context=edit_context,
                edited_at=edited_at,
                old_edited_at=old_edited_at,
            )
            if subject_ptr is not None:
                edit.subject_ptr.CopyFrom(subject_ptr)
            if node_data is not None:
                edit.node_data.CopyFrom(wiring.wrap_some_node(node_data))
            edits.append(edit)
            batch.clear()  # reset

        span.set_attribute("edits", len(edits))
        span.set_attribute("unconsumed_edit_events", len(unconsumed_edit_events))

        return unconsumed_edit_events, edits

    def preflush(self, filter: Callable[[EditEvent], bool] | None = None) -> list[EditData]:
        """Accumulates edit events into edits (without flushing)."""
        self._pending_edit_events, accumulated_edits = self._accumulate_edits(
            self._pending_edit_events, filter=filter
        )
        if self.session._local_epoch is not None:
            self._track_edits(accumulated_edits)
        self._pending_edits.extend(accumulated_edits)
        return accumulated_edits

    async def _do_flush(
        self, *, is_commit: bool, filter: Callable[[EditData], bool] | None = None
    ) -> tuple[list[EditData], list[EditData]]:
        """Flush any pending edits."""
        assert self.session is not None, f"no session for {self!r}"

        # turn pending mini edits into real edits
        _, accumulated_edits = self._accumulate_edits(self._pending_edit_events)
        if self.session._local_epoch is not None:
            self._track_edits(accumulated_edits)  # track new edits
        self._pending_edit_events = []
        edits = [*accumulated_edits]
        if filter is not None:
            # add existing pending edits
            edits.extend(e for e in self._pending_edits if filter(e))
            self._pending_edits = [e for e in self._pending_edits if not filter(e)]
        else:
            edits.extend(self._pending_edits)
            self._pending_edits = []
        self._edits.extend(edits)
        log = logger.bind(transaction=self, edits=len(edits))

        # assign edits to engines
        edits_by_engine_id: dict[Any, list[EditData]] = defaultdict(list)
        for edit in edits:
            engine = self.session._get_engine(
                edit.scope,
                NodeType(edit.node_ptr.node_type),
                include_removed=False,
                is_readonly=False,
                include_memory=False,
            )
            edits_by_engine_id[engine.id].append(edit)

        # flush to engines
        cascaded_edits: list[EditData] = []
        for engine in self.session._engines:
            # prepare edits & connector
            engine_edits = edits_by_engine_id.get(engine.id, [])
            if not (engine_edits or (is_commit and engine.id in self._touched_engine_ids)):
                continue  # nothing to do
            connector = await self.session._get_connector(engine)
            assert isinstance(
                connector, WritableConnector
            ), f"read-only {connector!r} for {engine!r}"

            # flush/commit
            message = "transaction.commit.engine" if is_commit else "transaction.flush.engine"
            with tracer.start_as_current_span(
                message, attributes={"engine": engine.__class__.__name__}
            ):
                if is_commit:
                    flush = await connector.commit(engine_edits)
                else:
                    flush = await connector.flush(engine_edits)
                log.trace(message, engine=engine, edits=len(engine_edits))
            cascaded_edits.extend(flush.cascaded_edits)
            self._cascaded_edits.extend(flush.cascaded_edits)
            self._touched_engine_ids.add(engine.id)

        log.trace("transaction.commit" if is_commit else "transaction.flush")
        return edits, cascaded_edits

    @tracer.start_as_current_span("transaction.flush")
    async def flush(
        self, filter: Callable[[EditData], bool] | None = None
    ) -> tuple[list[EditData], list[EditData]]:
        """Flushes any pending edits (without committing)."""
        return await self._do_flush(is_commit=False, filter=filter)

    @tracer.start_as_current_span("transaction.commit")
    async def commit(self) -> tuple[list[EditData], list[EditData]]:
        """Commits the transaction (flushing any pending edits)."""
        await self._do_flush(is_commit=True)
        edits, cascaded_edits = self._edits, self._cascaded_edits
        self._edits = []
        self._cascaded_edits = []
        # keep pending edit events (haven't been consumed yet)
        self._touched_engine_ids.clear()
        return edits, cascaded_edits

    def reset(self):
        """Resets the transaction, any edits and connectors (without closing)."""
        self._edits = []
        self._cascaded_edits = []
        self._pending_edit_events = []
        self._touched_engine_ids.clear()


@tracer.start_as_current_span("graph.edit_graph")
def edit_graph(
    graph: NodeGraph,
    supergraph: "NodeSuperGraph",
    edits: Collection[EditData],
    *,
    include_removed: bool,
    validate: bool,
    ignore_missing: bool = False,
) -> None:
    """Applies the edits to the graph (in place!)."""
    trace.get_current_span().set_attribute("edits", len(edits))

    from bench.proto import wiring

    for edit in edits:
        edit_type = cast(EditType, edit.type)
        node_id = UUID(edit.node_ptr.id)

        if edit_type in (EditType.CREATE, EditType.UPSERT) or (
            not include_removed
            and (edit_type == EditType.UNARCHIVE or edit_type == EditType.RESTORE)
        ):
            assert edit.HasField("node_data"), f"missing node_data for {edit!r}"
            node_data = wiring.unwrap_some_node(edit.node_data)
            # inline implicit metadata
            if edit_type == EditType.CREATE or edit_type == EditType.UPSERT:
                node_data.created_at.CopyFrom(edit.edited_at)
            node_data.updated_at.CopyFrom(edit.edited_at)
            node_data.ClearField("deleted_at")
            if edit.subject_ptr.metatype != 0:
                if edit_type == EditType.CREATE or edit_type == EditType.UPSERT:
                    node_data.created_by_ptr.CopyFrom(edit.subject_ptr)
                node_data.updated_by_ptr.CopyFrom(edit.subject_ptr)
            # unpack
            node = wiring.unpack_builtin_object(
                node_data, graph=graph, supergraph=supergraph, expect=Node
            )
            # NOTE :Robustness: should edit_graph ignore new nodes without parents? :RichGraph (like in bench-web)
            if edit_type == EditType.CREATE or node.id not in graph:
                graph.add(node)
            else:
                graph.update(node)
        elif edit_type == EditType.ERASE or (
            not include_removed and (edit_type == EditType.ARCHIVE or edit_type == EditType.DELETE)
        ):
            node = graph.get(node_id)
            if node is None:
                if not ignore_missing:
                    raise LookupError(
                        f"missing node {wiring.describe_node_ptr(edit.node_ptr)} for remove: {wiring.describe_edit(edit)}"
                    )
                continue
            graph.remove(node)
        else:
            # some update
            node = graph.get(node_id)
            if node is None:
                if not ignore_missing:
                    raise LookupError(
                        f"missing node {wiring.describe_node_ptr(edit.node_ptr)} for update: {wiring.describe_edit(edit)}"
                    )
                continue

            # apply edit operations
            for op in edit.operations:
                apply_edit_operation(node, op, validate=validate)

            # implicit metadata
            node.updated_at = edit.edited_at.ToDatetime(tzinfo=pytz.utc)
            updated_by_ptr = (
                wiring.unpack_builtin_object_prop(
                    Node.get_property("updated_by"), edit.subject_ptr, supergraph=supergraph
                )
                if edit.subject_ptr.metatype != 0
                else None
            )
            node._do_set("updated_by_ptr", updated_by_ptr, track=False)
            if edit_type == EditType.ARCHIVE:
                node._do_set("archived_at", edit.edited_at, track=False)
            elif edit_type == EditType.UNARCHIVE:
                node._do_set("archived_at", None, track=False)
            elif edit_type == EditType.DELETE:
                node._do_set("deleted_at", edit.edited_at, track=False)
            elif edit_type == EditType.RESTORE:
                node._do_set("deleted_at", None, track=False)
            graph.update(node)


@tracer.start_as_current_span("graph.edit_data_graph")
def edit_data_graph(
    graph: NodeDataGraph,
    edits: Collection[EditData],
    *,
    include_removed: bool,
    is_prepass: bool = False,
    ignore_missing: bool = False,
) -> list[EditData] | None:
    """
    Applies the edits to the data graph (edited nodes are copied before update).
    During prepass, we do some extra work (before applying the edits within in the system):
      1. Update edits (and edit operations) with the ground truth state.
      2. Simplify hierarchical edits into flat set/clear edits (for storage).
    """
    trace.get_current_span().set_attribute("edits", len(edits))

    from bench.proto import wiring

    def _make_vignette(node: AnyNodeData) -> ChangeVignetteData:
        vignette = ChangeVignetteData(metatype=pb2.OBJECT_TYPE_CHANGE_VIGNETTE)
        if name := getattr(node, "name", None):
            vignette.name = name
        if (subtype := getattr(node, "type", None)) is not None:
            vignette.subtype = subtype
        if (icon := getattr(node, "icon", None)) is not None:
            vignette.icon.CopyFrom(icon)
        return vignette

    flat_edits: list[EditData] = []  # for prepass
    for edit in edits:
        edit_type = cast(EditType, edit.type)
        node_type = NodeType(edit.node_ptr.node_type)
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        node_id = edit.node_ptr.id
        assert node_id is not None, f"missing node id for {edit!r}"
        subject_ptr = edit.subject_ptr if edit.subject_ptr.metatype != 0 else None

        if edit_type in (EditType.CREATE, EditType.UPSERT) or (
            not include_removed
            and (edit_type == EditType.UNARCHIVE or edit_type == EditType.RESTORE)
            and not is_prepass
        ):
            # add
            assert edit.HasField("node_data"), f"missing node_data for {edit!r}"
            node = wiring.unwrap_some_node(edit.node_data)
            # inline implicit metadata
            if edit_type == EditType.CREATE or edit_type == EditType.UPSERT:
                node.created_at.CopyFrom(edit.edited_at)
            node.updated_at.CopyFrom(edit.edited_at)
            node.ClearField("deleted_at")
            if subject_ptr is not None:
                if edit_type == EditType.CREATE or edit_type == EditType.UPSERT:
                    node.created_by_ptr.CopyFrom(subject_ptr)
                node.updated_by_ptr.CopyFrom(subject_ptr)
            else:
                node.ClearField("created_by_ptr")
                node.ClearField("updated_by_ptr")
            # NOTE :Robustness: should edit_graph ignore new nodes without parents? :RichGraph (like in bench-web)
            if edit_type == EditType.CREATE or node.id not in graph:
                graph.add(node)
            else:
                graph.update(node)

            # prepass: update new_node and make vignette with new data
            if is_prepass:
                edit.ClearField("node_data")
                edit.node_data.CopyFrom(wiring.wrap_some_node(node))  # :EditData
                edit.vignette.CopyFrom(_make_vignette(node))
                flat_edits.append(edit)
        elif (
            edit_type == EditType.ERASE
            or (
                not include_removed
                and (edit_type == EditType.ARCHIVE or edit_type == EditType.DELETE)
            )
        ) and not is_prepass:
            # remove
            node = graph.get(node_id)
            if node is None:
                if not ignore_missing:
                    raise LookupError(
                        f"missing node {wiring.describe_node_ptr(edit.node_ptr)} for {wiring.describe_edit(edit)}"
                    )
                continue
            graph.remove(node)
        else:
            # update
            node = graph.get(node_id)
            if node is None:
                if not ignore_missing:
                    raise LookupError(
                        f"missing node {wiring.describe_node_ptr(edit.node_ptr)} for {wiring.describe_edit(edit)}"
                    )
                continue
            node = wiring.copy_struct(node)

            # prepass: make vignette with old data
            if is_prepass:
                edit.vignette.CopyFrom(_make_vignette(node))
                if edit.type not in (EditType.UPDATE, EditType.MOVE):
                    edit.node_data.CopyFrom(wiring.wrap_some_node(node))

            # apply edit operations
            if edit_type in (EditType.UPDATE, EditType.MOVE):
                for op in edit.operations:
                    apply_edit_operation_data(node, op, is_prepass=is_prepass)

                # prepass: convert hierarchical edits into flat set/clear edits
                if is_prepass:
                    properties = {int(op.path[0]) for op in edit.operations}
                    flat_operations: list[EditOperationData] = []
                    for prop_id in properties:
                        prop = node_cls.__properties_by_id__.get(prop_id)
                        if prop is None:
                            continue
                        if prop.ptr_prop:
                            prop = prop.ptr_prop
                        if prop.is_optional_scalar and not node.HasField(prop.name):
                            new_value_packed = None
                            op_type = pb2.EDIT_OPERATION_TYPE_CLEAR
                        else:
                            new_value = getattr(node, prop.name)
                            new_value_packed = pack_value_data(new_value, prop.type_info)
                            op_type = pb2.EDIT_OPERATION_TYPE_SET
                        flat_op = EditOperationData(
                            metatype=pb2.OBJECT_TYPE_EDIT_OPERATION,
                            path=[str(prop_id)],
                            type=op_type,
                            new_value_packed=wiring.pack_proto_json(new_value_packed),
                        )
                        flat_operations.append(flat_op)
                    flat_edit = wiring.copy_struct(edit)
                    flat_edit.ClearField("operations")
                    flat_edit.operations.extend(flat_operations)
                    flat_edits.append(flat_edit)
            elif is_prepass:
                flat_edits.append(edit)

            # implicit metadata
            node.updated_at.CopyFrom(edit.edited_at)
            if edit.subject_ptr.metatype != 0:
                node.updated_by_ptr.CopyFrom(edit.subject_ptr)
            else:
                node.ClearField("updated_by_ptr")
            if edit_type == EditType.ARCHIVE:
                node.archived_at.CopyFrom(edit.edited_at)
            elif edit_type == EditType.UNARCHIVE:
                node.ClearField("archived_at")
            elif edit_type == EditType.DELETE:
                node.deleted_at.CopyFrom(edit.edited_at)
            elif edit_type == EditType.RESTORE:
                node.ClearField("deleted_at")

            graph.update(node)

    return flat_edits if is_prepass else None
