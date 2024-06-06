import asyncio
from collections import deque
from dataclasses import dataclass, field
from datetime import datetime
from typing import AsyncIterator, Mapping, NamedTuple, cast, final, override
from uuid import UUID

import betterproto
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import C, Expression, NodeReference, ReadOptions, Session, Subject
from bench.language.access import (
    AccessError,
    adapt_read_options,
    evaluate_and_adapt_read,
    evaluate_edit,
    generate_access_matrix,
)
from bench.language.connection import ConnectionFailedError, FetchOptions, StoreEngine
from bench.language.const import (
    BASED_NODE_TYPES,
    ConditionalOp,
    EditType,
    NodeType,
    PolicyEffect,
)
from bench.language.graph import NodeDataGraph, NodeDict, NodeGraphLike
from bench.language.node import (
    EDIT_SUBJECT_TYPES,
    HasBaseNode,
    Node,
    is_implicit_node_property,
)
from bench.language.property import Property
from bench.language.query import QueryBuilder
from bench.language.session import SessionContext
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.language.transaction import edit_data_graph, unpack_node_delta
from bench.language.validation import ValidationError, on_invalid_raise
from bench.proto import wiring
from bench.proto.services import ServiceBase
from bench.proto.wire import (
    AccessMatrixData,
    AggregateNodesRequest,
    AggregateNodesResponse,
    AnyNodeData,
    CancelTransactionRequest,
    CancelTransactionResponse,
    CommitTransactionRequest,
    CommitTransactionResponse,
    CompleteTransactionRequest,
    CompleteTransactionResponse,
    EditData,
    FlushTransactionRequest,
    FlushTransactionResponse,
    GetNodesRequest,
    GetNodesResponse,
    GraphIoBase,
    GraphScope,
    NodeReferenceData,
    SearchNodesRequest,
    SearchNodesResponse,
    WatchEditsRequest,
    WatchEditsResponse,
)
from bench.utils.dt import utcnow
from bench.utils.func import CriticalLock, bittuple, group_by, to_uuid, uuid_to_str
from bench.utils.utils import get_from_env

TRANSACTION_BUFFER_SIZE = get_from_env("TRANSACTION_BUFFER_SIZE", typ=int, default=1000)
MAX_TIME_DRIFT_SECONDS = get_from_env("MAX_TIME_DRIFT_SECONDS", typ=int, default=60)


class _Commit(NamedTuple):
    """A commit of multiple edits (with their own epochs)"""

    epoch: int
    edits: list[EditData]
    cascaded_edits: list[EditData]


@dataclass(slots=True)
class EditWatcher:
    """An active subscriber to the watch_edits server stream."""

    subject: Subject
    node_types: bittuple[NodeType]
    filters: Mapping[NodeType, Expression]
    sink: asyncio.Queue[_Commit] = field(default_factory=asyncio.Queue)

    def __str__(self):
        return f"{self.subject}: {'|'.join(n.bench_name for n in self.node_types.tuple)} [{self.filters}]"


class GraphIoServiceBase(ServiceBase, GraphIoBase):
    """Common base for global & Bench-local graph I/O operations."""

    def __init__(
        self,
        *,
        bench_id: UUID | None,
        node_types: bittuple[NodeType],
        logger: structlog.BoundLogger,
        tracer: trace.Tracer,
    ):
        super().__init__(logger=logger, tracer=tracer)
        self.epoch: int = 0
        self.recent_transactions: deque[_Commit] = deque(maxlen=TRANSACTION_BUFFER_SIZE)
        self.bench_id: UUID | None = bench_id
        self.scope = GraphScope(bench_id=uuid_to_str(bench_id))
        self.node_types: bittuple[NodeType] = node_types
        self.watchers: list[EditWatcher] = []
        self.tx_lock: asyncio.Lock = CriticalLock(
            name=f"{self.__class__.__name__}_{bench_id or ''}"
        )

    def get_engines(self) -> tuple[StoreEngine, ...]:
        """Gets the store engines available to this subgraph. Implemented in the actual service."""
        raise NotImplementedError

    def _validate_request(self, request: betterproto.Message) -> None:
        """Validate a request message for this service."""
        scope: GraphScope = getattr(request, "scope", GraphScope())
        if to_uuid(scope.bench_id) != self.bench_id:
            raise GRPCError(
                GRPCStatus.INVALID_ARGUMENT, f"scope mismatch: {scope.bench_id} != {self.bench_id}"
            )

    def request_session(
        self,
        *,
        engines: tuple[StoreEngine, ...] | None = None,
        readonly: bool = True,
        system_commit: bool = True,
    ):
        """Gets a new session for processing a single request."""
        return Session(
            parent=None,
            _is_readonly=readonly,
            _default_scope=self.scope,
            _engines=engines if engines is not None else self.get_engines(),
            _epoch=self.epoch,
            _custom_commit=self._commit_system_session if system_commit else None,
        )

    @override
    async def get_nodes(self, subject: Subject, request: "GetNodesRequest") -> "GetNodesResponse":
        # parse request
        roots: tuple[NodeReference, ...] = tuple(
            wiring.unpack_object_interp(r, expect=NodeReference) for r in request.roots
        )
        if not roots:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no roots provided")
        if any(not r.id for r in roots):
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "root nodes must have an id")
        options: ReadOptions = (
            wiring.unpack_object_interp_maybe(request.options, expect=ReadOptions)
            or ReadOptions.default()
        )

        # fetch
        roots_by_type: dict[NodeType, list[NodeReference]] = group_by(roots, lambda r: r.type)
        graph = NodeDataGraph()
        async with self.request_session() as session:
            for root_node_type, root_node_references in roots_by_type.items():
                root_ids = tuple(r.id for r in root_node_references)
                query = QueryBuilder(
                    node_type=wiring.unpack_enum(NodeType, root_node_type),
                    filter=C(ConditionalOp.IN, property=Node.id, value=root_ids),
                    options=adapt_read_options(subject, root_node_type, options),
                )
                result = await session.tx._read_connection.fetch(query, FetchOptions(count=False))
                graph.extend(result.nodes)
        if any(cast(str, root.id) not in graph for root in request.roots):
            missing_roots = tuple(root for root in roots if str(root.id) not in graph)
            raise GRPCError(GRPCStatus.NOT_FOUND, f"roots not found: {missing_roots}")

        # check access
        with self.tracer.start_as_current_span("graph.get.check_access"):
            matrix = generate_access_matrix(subject, graph)
            decision, accesses, adapted_nodes = evaluate_and_adapt_read(
                matrix, graph, required_nodes=request.roots
            )
            if decision != PolicyEffect.ALLOW:
                raise AccessError(accesses)

        self.logger.info(
            "graph.get", subject=subject, graph=graph, epoch=self.epoch, span="current"
        )
        return GetNodesResponse(
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            access=cast(AccessMatrixData, matrix._to_data()),
            epoch=self.epoch,
        )

    @override
    async def search_nodes(
        self, subject: Subject, request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
        # parse request
        node_type: NodeType = wiring.unpack_enum(NodeType, request.node_type)
        filter: Expression | None = wiring.unpack_object_interp_maybe(
            request.filter, expect=Expression
        )
        sort: list[Expression] = [
            wiring.unpack_object_interp(s, expect=Expression) for s in request.sort
        ] or []
        options: ReadOptions = (
            wiring.unpack_object_interp_maybe(request.options, expect=ReadOptions)
            or ReadOptions.default()
        )

        # fetch
        adapted_options = adapt_read_options(subject, node_type, options)
        roots: list[NodeReferenceData] = []
        async with self.request_session() as session:
            query = QueryBuilder(
                node_type=node_type, filter=filter, options=adapted_options, sort=sort
            )
            result = await session.tx._read_connection.fetch(
                query, FetchOptions(count=request.count or False)
            )
            roots.extend(result.roots)
            graph = NodeDataGraph(result.nodes)

        # check access
        with self.tracer.start_as_current_span("graph.search.check_access"):
            matrix = generate_access_matrix(subject, graph)
            decision, accesses, adapted_nodes = evaluate_and_adapt_read(
                matrix, graph, required_nodes=request.bases
            )
            if decision != PolicyEffect.ALLOW:
                raise AccessError(accesses)

        self.logger.info(
            "graph.search", subject=subject, graph=graph, epoch=self.epoch, span="current"
        )
        return SearchNodesResponse(
            roots=roots,
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            cursors=list(result.cursors),
            start_cursor=result.start_cursor,
            total=result.total,
            access=cast(AccessMatrixData, matrix._to_data()),
            epoch=self.epoch,
        )

    @override
    async def aggregate_nodes(
        self, subject: Subject, request: "AggregateNodesRequest"
    ) -> "AggregateNodesResponse":
        # parse request
        if request.bases:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO has no bases")
        node_type: NodeType = wiring.unpack_enum(NodeType, request.node_type)
        filter: Expression | None = wiring.unpack_object_interp_maybe(request.filter)
        aggregation: Expression = wiring.unpack_object_interp(request.aggregation)

        # fetch
        adapted_options = adapt_read_options(subject, node_type, ReadOptions())
        async with self.request_session() as session:
            query = QueryBuilder(
                node_type=node_type, filter=filter, options=adapted_options, aggregation=aggregation
            )
            result = await session.tx._read_connection.aggregate(query)

        # TODO :Security!: check aggregation access

        self.logger.debug("graph.aggregate", subject=subject, epoch=self.epoch, span="current")
        return AggregateNodesResponse(aggregation=result.aggregation, epoch=self.epoch)

    def _prepare_commit(
        self, subject: Subject, context: SessionContext, edits: list[EditData]
    ) -> tuple["CommitScope", int]:
        """Prepares and validates the edits for a commit."""
        scope = parse_commit_scope(edits, base_graph=None)
        now = utcnow()
        epoch = self.epoch
        for edit in edits:
            validate_edit(edit, subject, now)
            epoch += 1
            edit.epoch = epoch
        return scope, epoch

    @override
    async def watch_edits(
        self, subject: Subject, request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        if not request.node_types:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no node types provided")
        node_types = bittuple(*tuple(wiring.unpack_enum(NodeType, t) for t in request.node_types))
        filters: dict[NodeType, Expression] = {
            wiring.unpack_enum(NodeType, k): cast(Expression, wiring.unpack_object_interp(v))
            for k, v in request.filters.items()
        }
        watcher = EditWatcher(subject=subject, node_types=node_types, filters=filters)
        self.watchers.append(watcher)

        try:
            # replay recent epochs
            if request.since_epoch is not None and request.since_epoch > self.epoch:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "can't watch from the future")
            if request.since_epoch is not None:
                num_epochs_to_replay = self.epoch - request.since_epoch
                if num_epochs_to_replay > TRANSACTION_BUFFER_SIZE:
                    raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "too much to replay")
                commits_to_replay = []
                for epoch, edits, cascaded_edits in reversed(self.recent_transactions):
                    if epoch <= request.since_epoch:
                        break
                    edits = self._filter_and_adapt_edits(watcher, edits)
                    cascaded_edits = self._filter_and_adapt_edits(watcher, cascaded_edits)
                    commits_to_replay.append((epoch, edits, cascaded_edits))
                if commits_to_replay:
                    self.logger.info(
                        "graph.watch.replay", watcher=watcher, commits=commits_to_replay
                    )
                    for epoch, edits, cascaded_edits in commits_to_replay:
                        yield WatchEditsResponse(
                            edits=edits, cascaded_edits=cascaded_edits, epoch=epoch
                        )

            # listen for new epochs
            self.logger.info("graph.watch", watcher=watcher, span="current")
            while True:
                epoch = await watcher.sink.get()
                yield WatchEditsResponse(edits=epoch.edits, epoch=epoch.epoch)
        finally:
            self.watchers.remove(watcher)

    @override
    async def commit_transaction(
        self, subject: Subject, request: "CommitTransactionRequest"
    ) -> "CommitTransactionResponse":
        assert subject.client is not None, f"no client for {subject!r}"

        # figure out context
        context = wiring.unpack_object_interp_maybe(request.context, expect=SessionContext)
        if context is None:
            context = SessionContext(
                client=subject.client, server=subject.server, user=subject.user
            )

        # NOTE :Performance: obviously, putting a big lock around commit is not ideal,
        #  but we have to guarantee absolute order + integrity of any loaded graphs (in Host).
        # We can probably optimize this by only locking some tighter critical sections
        #  if we rollback somehow if an optimistic commit (outside the lock) fails... somehow.
        async with self.tx_lock:
            # pre-validate/prepare edits
            scope, epoch = self._prepare_commit(subject, context, request.edits)

            async with self.request_session(readonly=False, system_commit=False) as session:
                # read the affected nodes into a single graph for evaluation
                data_graph = NodeDataGraph()
                with self.tracer.start_as_current_span("graph.commit.read"):
                    for node_type, node_references in scope.scopes_by_type.items():
                        node_type = wiring.unpack_enum(NodeType, node_type)
                        # NOTE :Performance: select only properties required to evaluate edit (id/policies/...?)
                        options = adapt_read_options(subject, node_type, ReadOptions.all())
                        node_ids = tuple(r.id for r in node_references)
                        query = QueryBuilder(
                            node_type=node_type,
                            filter=C(ConditionalOp.IN, property=Node.id, value=node_ids),
                            options=options,
                        )
                        result = await session.tx._read_connection.fetch(
                            query, FetchOptions(count=False)
                        )
                        # merge result into data_graph (there may be duplicates)
                        for node_data in result.nodes:
                            if node_data.id not in data_graph:
                                data_graph.add(node_data)
                    self.logger.trace("graph.commit.read", graph=data_graph)

                # check access
                with self.tracer.start_as_current_span("graph.commit.check_access"):
                    matrix = generate_access_matrix(subject, data_graph)
                    decision, accesses = evaluate_edit(matrix, data_graph, request.edits)
                    if decision != PolicyEffect.ALLOW:
                        raise AccessError(accesses)

                # apply edits in copy (to validate and get current 'old' values)
                # TODO :Robustness!: prevent circular parent/child references
                edit_data_graph(
                    graph=data_graph,
                    edits=request.edits,
                    options=ReadOptions.all(),
                    is_prepass=True,
                )
                unpacked_graph = wiring.unpack_node_graph(data_graph, parent=None, session=session)
                for node_id in scope.edited_node_ids:
                    node_ = unpacked_graph.get(UUID(node_id))
                    if node_ is None:
                        raise GRPCError(GRPCStatus.NOT_FOUND, f"{node_id} not found")
                    node_._validate_self(properties=(), invalid=on_invalid_raise)

                # flush edits to get cascaded edits for extend
                assert len(session.tx.edits) == 0, f"unexpected edits in {session.tx!r}"
                session.tx._add_pending_edits(request.edits)
                _, cascaded_edits = await session.flush()

                # extend commit
                new_edits = await self.extend_commit(
                    session=session,
                    context=context,
                    graph=unpacked_graph,
                    edits=request.edits,
                    cascaded_edits=cascaded_edits,
                )

                # actually commit (with new edits)
                edits, cascaded_edits = await session.commit()
            session.untrack_many(*unpacked_graph.nodes)

            # handle on commit
            self.epoch = epoch
            await self.on_commit(
                graph=unpacked_graph, edits=edits, cascaded_edits=cascaded_edits, epoch=self.epoch
            )

        self.logger.info(
            "graph.commit",
            subject=subject,
            request=request,
            request_edits=request.edits,
            extended_edits=new_edits,
            cascaded_edits=len(cascaded_edits),
            epoch=self.epoch,
            span="current",
        )
        accepted_revisions = [cast(int, e.revision) for e in request.edits]
        return CommitTransactionResponse(
            revisions=accepted_revisions, cascaded_edits=cascaded_edits, epoch=self.epoch
        )

    async def _commit_system_session(
        self, session: Session
    ) -> tuple[list[EditData], list[EditData]]:
        """
        Commits a system session (outside a request context).
        This is like GraphIo.commit_transaction but without validation.
        """
        assert session._tx is not None, f"no active tx in {session!r}"
        edit_graph = NodeDict(session._edited_nodes_by_id)

        try:
            # flush edits to get cascaded edits
            edits, cascaded_edits = await session._tx.flush()

            # extend commit
            await self.extend_commit(
                session=session,
                context=None,
                graph=edit_graph,
                edits=edits,
                cascaded_edits=cascaded_edits,
            )

            # commit
            edits, cascaded_edits = await session._tx.commit()
        except ConnectionFailedError as e:
            self.logger.error("graph.commit.error", session=session, error=e)
            await session._tx.reset()
            raise

        # handle on commit
        self.epoch = session.epoch
        await self.on_commit(
            edit_graph, edits=edits, cascaded_edits=cascaded_edits, epoch=self.epoch
        )
        return edits, cascaded_edits

    @override
    async def flush_transaction(
        self, subject: "Subject", request: "FlushTransactionRequest"
    ) -> "FlushTransactionResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)  # :2PC

    @override
    async def complete_transaction(
        self, subject: Subject, request: "CompleteTransactionRequest"
    ) -> "CompleteTransactionResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)  # :2PC

    @override
    async def cancel_transaction(
        self, subject: Subject, request: "CancelTransactionRequest"
    ) -> "CancelTransactionResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)  # :2PC

    async def extend_commit(
        self,
        session: Session,
        context: SessionContext | None,
        graph: NodeGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
    ) -> list[EditData]:
        """Extend a commit in a request session. Returns any new edits, but must add them to session."""
        return []  # do nothing by default

    @final
    async def on_commit(
        self,
        graph: NodeGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        """Handle a commit in the request session."""
        self.recent_transactions.append(_Commit(self.epoch, edits, cascaded_edits))

        # notify watchers
        for watcher in self.watchers:
            adapted_edits = self._filter_and_adapt_edits(watcher, edits)
            adapted_cascaded_edits = self._filter_and_adapt_edits(watcher, cascaded_edits)
            if adapted_edits:
                watcher.sink.put_nowait(_Commit(self.epoch, adapted_edits, adapted_cascaded_edits))

        await self._on_commit(graph=graph, edits=edits, cascaded_edits=cascaded_edits)

    async def _on_commit(
        self, graph: NodeGraphLike, edits: list[EditData], cascaded_edits: list[EditData]
    ):
        """Handle a commit in the request session."""
        pass  # do nothing by default

    @final
    def _filter_and_adapt_edits(
        self, watcher: EditWatcher, edits: list[EditData]
    ) -> list[EditData]:
        # TODO :Broken :Security!: adapt graph edits to watcher's access
        adapted_edits = []
        for edit in edits:
            node_type = wiring.unpack_enum(NodeType, edit.node_ptr.type)
            if node_type in watcher.node_types:
                adapted_edits.append(edit)
        return adapted_edits


class CommitScope(NamedTuple):
    """The scope of relevant nodes for a transaction."""

    edited_node_ids: set[str]
    scopes_by_type: dict[NodeType, list[NodeReference]]
    graph_scopes: tuple[GraphScope, ...]


def parse_commit_scope(
    edits: list[EditData], base_graph: NodeDataGraph[AnyNodeData] | None
) -> CommitScope:
    """
    Gets the specific nodes (scopes) and related nodes that are edited.
    :NodeEditScope
    """
    from bench.proto import wiring

    edited_node_ids: set[str] = set()
    node_scopes_by_id: dict[str, NodeReferenceData] = {}
    graph_scopes: dict[int, GraphScope] = {}
    in_tx_created_nodes_ids: set[str] = set()
    for edit in edits:
        node_type = NodeType(edit.node_ptr.type)
        node_id = cast(str, edit.node_ptr.id)
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        edited_node_ids.add(node_id)
        if edit.type == EditType.CREATE or edit.type == EditType.UPSERT:
            assert edit.new_node_packed
            new_node = unpack_node_delta(
                edit.new_node_packed, node_type=node_type, only=(node_cls.__parent_property__,)
            )
            # node scope is parent since we don't have this node yet
            if new_node.parent_ptr is None:
                raise ValidationError(new_node, "can't create orphan")
            elif new_node.parent_ptr.id not in node_scopes_by_id:
                node_scope = new_node.parent_ptr
            else:
                node_scope = node_scopes_by_id[new_node.parent_ptr.id]
            in_tx_created_nodes_ids.add(node_id)
        else:
            # node scope is the edited node itself
            if node_id in in_tx_created_nodes_ids:
                continue  # skip just created nodes
            node_scope = edit.node_ptr
            if edit.type == EditType.MOVE:
                # also add new parent to scope
                assert edit.new_node_packed, f"missing new node for {edit!r}"
                new_node = unpack_node_delta(
                    edit.new_node_packed, node_type=node_type, only=(node_cls.__parent_property__,)
                )
                assert new_node.parent_ptr is not None, f"missing parent for {new_node}"
                node_scopes_by_id[cast(str, new_node.parent_ptr.id)] = new_node.parent_ptr
        if node_type in BASED_NODE_TYPES and edit.node_ptr.base_ck is not None:
            # also add base as node scope
            assert base_graph is not None, f"missing base graph for {edit!r}"
            node_cls = cast(type[HasBaseNode], NODE_CLASS_BY_TYPE[node_type])
            # add current base (base is immutable)
            old_base_node = base_graph.get(edit.node_ptr.base_ck)
            if old_base_node is not None:
                old_base_ptr = NodeReference.from_node_data(old_base_node)
                node_scopes_by_id[cast(str, old_base_ptr.id)] = old_base_ptr
        node_scopes_by_id[node_id] = node_scope

        # graph scope
        graph_scope = edit.scope
        graph_scope_hash = hash((graph_scope.bench_id, graph_scope.package_id))
        if graph_scope_hash not in graph_scopes:
            graph_scopes[graph_scope_hash] = graph_scope

    node_scopes: dict[UUID, NodeReference] = {
        UUID(k): wiring.unpack_object(v, expect=NodeReference) for k, v in node_scopes_by_id.items()
    }
    node_scopes_by_type = group_by(node_scopes.values(), lambda n: n.type)
    return CommitScope(
        edited_node_ids=edited_node_ids,
        scopes_by_type=node_scopes_by_type,
        graph_scopes=tuple(graph_scopes.values()),
    )


def _is_allowable_drift(dt: datetime, now: datetime) -> bool:
    """Check if the given datetime is within the allowed time drift."""
    return abs((now - dt).total_seconds()) <= MAX_TIME_DRIFT_SECONDS


def validate_edit(edit: EditData, subject: Subject, now: datetime) -> None:
    """Checks the given edit for basic validity in isolation."""
    assert subject.client, f"{subject!r} has no client"
    node_cls = NODE_CLASS_BY_TYPE[cast(NodeType, edit.node_ptr.type)]

    # subject
    if subject.client.parent_type == NodeType.USER:
        user_id = str(subject.user.id) if subject.user else None
        # subject must match user
        if not edit.subject_ptr or edit.subject_ptr.id != user_id:
            raise GRPCError(
                GRPCStatus.PERMISSION_DENIED,
                f"subject mismatch in {edit!r}: {edit.subject_ptr} != {user_id}",
            )
    else:
        # subject must be a Run/Server
        if not edit.subject_ptr or edit.subject_ptr.type not in EDIT_SUBJECT_TYPES:
            raise GRPCError(
                GRPCStatus.PERMISSION_DENIED,
                f"bad created_by in {edit!r}: {edit.subject_ptr!r}",
            )
    # origin
    if not edit.origin or UUID(edit.origin.id) != subject.client.id:
        raise GRPCError(
            GRPCStatus.PERMISSION_DENIED,
            f"origin mismatch in {edit!r}: {edit.origin!r} != {subject.client!r}",
        )

    # time
    if not edit.edited_at or not _is_allowable_drift(edit.edited_at, now):
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT,
            f"bad edited_at in {edit!r}: {edit.edited_at} != {now}",
        )

    # old/new node packed
    should_set_new = edit.type in (
        EditType.CREATE,
        EditType.UPSERT,
        EditType.UPDATE,
        EditType.MOVE,
        EditType.UNARCHIVE,
        EditType.RESTORE,
    )
    should_set_old = edit.type in (
        EditType.UPDATE,
        EditType.MOVE,
        EditType.DELETE,
        EditType.ARCHIVE,
        EditType.ERASE,
    )
    if should_set_new != (edit.new_node_packed is not None):
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT, f"bad new_node_packed in {edit!r}: {edit.new_node_packed}"
        )
    if should_set_old != (edit.old_node_packed is not None):
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT, f"bad old_node_packed in {edit!r}: {edit.old_node_packed}"
        )

    # properties
    if edit.type in (EditType.UPDATE, EditType.MOVE):
        # check that properties are in both old and new
        assert edit.old_node_packed and edit.new_node_packed
        old_node_packed = edit.old_node_packed.to_dict()
        new_node_packed = edit.new_node_packed.to_dict()
        for p in edit.properties:
            if str(p) not in old_node_packed:
                raise GRPCError(
                    GRPCStatus.INVALID_ARGUMENT,
                    f"missing property in {edit!r}: {p} not in {tuple(old_node_packed.keys())}",
                )
            if str(p) not in new_node_packed:
                raise GRPCError(
                    GRPCStatus.INVALID_ARGUMENT,
                    f"missing property in {edit!r}: {p} not in {tuple(new_node_packed.keys())}",
                )
        # no forbidden properties
        if any(is_implicit_node_property(p) for p in edit.properties):
            bad_properties = [
                node_cls.__properties_by_id__[p]
                for p in edit.properties
                if is_implicit_node_property(p)
            ]
            raise GRPCError(
                GRPCStatus.PERMISSION_DENIED,
                f"cannot explicitly set implicit properties in {edit!r}: {bad_properties!r}",
            )
        has_parent = cast(Property, Node.parent).id in edit.properties
        if (edit.type == EditType.MOVE) != has_parent:
            raise GRPCError(
                GRPCStatus.INVALID_ARGUMENT,
                f"only move can set parent property in {edit!r}: {edit.properties}",
            )
    elif edit.properties:
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT, f"cannot set properties in {edit!r}: {edit.properties}"
        )
