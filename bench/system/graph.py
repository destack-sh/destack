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
from bench.language.connection import FetchOptions, StoreEngine
from bench.language.const import (
    BASED_NODE_TYPES,
    ConditionalOp,
    EditType,
    NodeType,
    PolicyEffect,
)
from bench.language.graph import NodeDataGraph, NodeGraphLike, edit_data_graph
from bench.language.node import EDIT_SUBJECT_TYPES, BasedNode, Node
from bench.language.query import QueryBuilder
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.language.transaction import ALL_IMPLICIT_PROPERTIES_IDS
from bench.language.validation import ValidationError, on_invalid_raise
from bench.proto import wiring
from bench.proto.services import BenchServiceBase
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

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

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


class GraphIoServiceBase(BenchServiceBase, GraphIoBase):
    """Common base for global & Bench-local graph I/O operations."""

    def __init__(self, *, bench_id: UUID | None, node_types: bittuple[NodeType]):
        super().__init__()
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
    ):
        """Gets a new session for processing a request."""
        return Session(
            parent=None,
            _is_readonly=readonly,
            _default_scope=self.scope,
            _engines=engines if engines is not None else self.get_engines(),
            _extend_commit_hook=self.extend_commit,
            _on_commit_hook=self.on_commit,
            _epoch=self.epoch,
        )

    @override
    async def get_nodes(self, subject: Subject, request: "GetNodesRequest") -> "GetNodesResponse":
        # parse request
        roots: tuple[NodeReference, ...] = tuple(
            wiring.unpack_struct_interp(r, expect=NodeReference) for r in request.roots
        )
        if not roots:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no roots provided")
        if any(not r.id for r in roots):
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "root nodes must have an id")
        options: ReadOptions = (
            wiring.unpack_struct_interp_maybe(request.options, expect=ReadOptions)
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
        matrix = generate_access_matrix(subject, graph)
        decision, accesses, adapted_nodes = evaluate_and_adapt_read(
            matrix, graph, required_nodes=request.roots, adapt_nodes_in_place=True
        )
        if decision != PolicyEffect.ALLOW:
            raise AccessError(accesses)

        logger.info("graph.get", subject=subject, graph=graph, epoch=self.epoch)
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
        filter: Expression | None = wiring.unpack_struct_interp_maybe(
            request.filter, expect=Expression
        )
        sort: list[Expression] = [
            wiring.unpack_struct_interp(s, expect=Expression) for s in request.sort
        ] or []
        options: ReadOptions = (
            wiring.unpack_struct_interp_maybe(request.options, expect=ReadOptions)
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
        matrix = generate_access_matrix(subject, graph)
        decision, accesses, adapted_nodes = evaluate_and_adapt_read(
            matrix, graph, adapt_nodes_in_place=True, required_nodes=request.bases
        )
        if decision != PolicyEffect.ALLOW:
            raise AccessError(accesses)

        logger.info("graph.search", subject=subject, graph=graph, epoch=self.epoch)
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
        filter: Expression | None = wiring.unpack_struct_interp_maybe(request.filter)
        aggregation: Expression = wiring.unpack_struct_interp(request.aggregation)

        # fetch
        adapted_options = adapt_read_options(subject, node_type, ReadOptions())
        async with self.request_session() as session:
            query = QueryBuilder(
                node_type=node_type, filter=filter, options=adapted_options, aggregation=aggregation
            )
            result = await session.tx._read_connection.aggregate(query)

        # TODO :Security!: check aggregation access

        logger.debug("graph.aggregate", subject=subject, epoch=self.epoch)
        return AggregateNodesResponse(aggregation=result.aggregation, epoch=self.epoch)

    @override
    async def commit_transaction(
        self, subject: Subject, request: "CommitTransactionRequest"
    ) -> "CommitTransactionResponse":
        # check/prepare edits
        assert subject.client, f"{subject!r} has no client"
        edit_scopes = parse_edit_scopes(request.edits)
        now = utcnow()
        epoch = self.epoch
        for edit in request.edits:
            _validate_edit(edit, subject, now)
            node_data = wiring.unwrap_some_node(edit.node)
            epoch += 1
            edit.epoch = epoch
            if edit.type in (EditType.CREATE, EditType.UPSERT):
                node_data.created_epoch = epoch
            node_data.updated_epoch = epoch

        # process edits
        async with self.tx_lock:
            async with self.request_session(readonly=False) as session:
                # read the affected nodes into a single graph
                data_graph = NodeDataGraph()
                with tracer.start_as_current_span("graph.commit.read") as span:
                    for node_type, node_references in edit_scopes.scopes_by_type.items():
                        node_type = wiring.unpack_enum(NodeType, node_type)
                        # NOTE :Performance: select only properties required to evaluate edit (id/policies/...?)
                        options = adapt_read_options(subject, node_type, ReadOptions.default())
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
                    logger.trace("graph.commit.read", graph=data_graph, span=span)

                # check access
                with tracer.start_as_current_span("graph.commit.access") as span:
                    matrix = generate_access_matrix(subject, data_graph)
                    decision, accesses = evaluate_edit(matrix, data_graph, request.edits)
                    if decision != PolicyEffect.ALLOW:
                        raise AccessError(accesses)

                # validate edits (in copy)
                # TODO :Robustness: prevent circular parent/child references
                edit_data_graph(
                    graph=data_graph,
                    edits=request.edits,
                    options=ReadOptions.all(),
                    keep_all=True,
                    update_nodes_in_place=False,
                )
                unpacked_graph = wiring.unpack_node_graph(data_graph, parent=None, session=session)
                for node_id in edit_scopes.edited_node_ids:
                    node_data = unpacked_graph.get(UUID(node_id))
                    if node_data is None:
                        raise GRPCError(GRPCStatus.NOT_FOUND, f"{node_id} not found")
                    node_data._validate_self(properties=(), invalid=on_invalid_raise)

                # flush edits to get cascaded edits
                session.tx._add_pending_edits(request.edits)
                _, cascaded_edits = await session.flush()

                # extend commit
                new_edits, epoch = await self.extend_commit(
                    session=session,
                    graph=unpacked_graph,
                    edits=request.edits,
                    epoch=epoch,
                    cascaded_edits=cascaded_edits,
                )
                session.tx._add_pending_edits(new_edits)

                # commit (suppress hooks because we're firing them manually above/below)
                edits, cascaded_edits = await session.commit(_suppress_hooks=True)
            session.untrack_many(*unpacked_graph.nodes)

            # handle on commit
            self.epoch = epoch
            await self.on_commit(
                graph=unpacked_graph, edits=edits, epoch=epoch, cascaded_edits=cascaded_edits
            )

        logger.info(
            "graph.commit",
            subject=subject,
            request=request,
            request_edits=request.edits,
            extended_edits=new_edits,
            cascaded_edits=len(cascaded_edits),
            epoch=self.epoch,
        )
        accepted_revisions = [cast(int, e.revision) for e in request.edits]
        return CommitTransactionResponse(
            revisions=accepted_revisions, cascaded_edits=cascaded_edits, epoch=self.epoch
        )

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

    @override
    async def watch_edits(
        self, subject: Subject, request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        if not request.node_types:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no node types provided")
        node_types = bittuple(*tuple(wiring.unpack_enum(NodeType, t) for t in request.node_types))
        filters: dict[NodeType, Expression] = {
            wiring.unpack_enum(NodeType, k): cast(Expression, wiring.unpack_struct_interp(v))
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
                epochs_to_replay = []
                for epoch, edits, cascaded_edits in reversed(self.recent_transactions):
                    if epoch <= request.since_epoch:
                        break
                    edits = self._filter_and_adapt_edits(watcher, edits)
                    cascaded_edits = self._filter_and_adapt_edits(watcher, cascaded_edits)
                    epochs_to_replay.append((epoch, edits, cascaded_edits))
                if epochs_to_replay:
                    logger.info("graph.watch.replay", watcher=watcher, epochs=epochs_to_replay)
                    for epoch, edits, cascaded_edits in epochs_to_replay:
                        yield WatchEditsResponse(
                            edits=edits, cascaded_edits=cascaded_edits, epoch=epoch
                        )

            # listen for new epochs
            logger.info("graph.watch", watcher=watcher)
            while True:
                epoch = await watcher.sink.get()
                yield WatchEditsResponse(edits=epoch.edits, epoch=epoch.epoch)
        finally:
            self.watchers.remove(watcher)

    async def extend_commit(
        self,
        session: Session,
        graph: NodeGraphLike,
        edits: list[EditData],
        epoch: int,
        cascaded_edits: list[EditData],
    ) -> tuple[list[EditData], int]:
        return [], epoch  # do nothing by default

    @final
    async def on_commit(
        self,
        graph: NodeGraphLike,
        edits: list[EditData],
        epoch: int,
        cascaded_edits: list[EditData],
    ):
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
        pass  # do nothing by default

    @final
    def _filter_and_adapt_edits(
        self, watcher: EditWatcher, edits: list[EditData]
    ) -> list[EditData]:
        # TODO :Broken :Security!: adapt graph edits to watcher's access
        adapted_edits = []
        for edit in edits:
            node_type = wiring.unpack_enum(NodeType, edit.node_type)
            if node_type in watcher.node_types:
                adapted_edits.append(edit)
        return adapted_edits


class _EditScopes(NamedTuple):
    edited_node_ids: set[str]
    scopes_by_type: dict[NodeType, list[NodeReference]]
    graph_scopes: tuple[GraphScope, ...]


def validate_node_scope(node_data: AnyNodeData, graph_scope: GraphScope):
    bench_ptr = getattr(node_data, "bench_ptr", None)
    if bench_ptr and bench_ptr.id != graph_scope.bench_id:
        raise ValidationError(
            node_data,
            f"node {node_data} has bench_id: {bench_ptr.id} != {graph_scope.bench_id}",
        )


def parse_edit_scopes(edits: list[EditData]) -> _EditScopes:
    """
    Gets the specific nodes (scopes) and broader graph scopes that are edited.
    Also verifies that the edited scopes match the nodes data.
    :NodeEditScope
    """
    from bench.proto import wiring

    edited_node_ids: set[str] = set()
    node_scopes_by_id: dict[str, NodeReferenceData] = {}
    graph_scopes: dict[int, GraphScope] = {}
    just_created_nodes_id: set[str] = set()
    for edit in edits:
        node_type = NodeType(edit.node_type)
        node_data = wiring.unwrap_some_node(edit.node)
        edited_node_ids.add(node_data.id)
        if edit.type == EditType.CREATE or edit.type == EditType.UPSERT:
            # node scope is parent since we don't have this node yet
            if node_data.parent_ptr is None:
                raise ValidationError(node_data, "can't create orphan")
            elif node_data.parent_ptr.id not in node_scopes_by_id:
                node_scope = node_data.parent_ptr
            else:
                node_scope = node_scopes_by_id[node_data.parent_ptr.id]
            just_created_nodes_id.add(node_data.id)
        else:
            # node scope is the edited node itself
            if node_data.id in just_created_nodes_id:
                continue  # skip just created nodes
            node_scope = NodeReference.from_node_data(node_data)
            if edit.type == EditType.MOVE:
                # also add new parent as node scope
                assert node_data.parent_ptr is not None, f"missing parent for {node_data}"
                node_scopes_by_id[cast(str, node_data.parent_ptr.id)] = node_data.parent_ptr
        if node_type in BASED_NODE_TYPES:
            # also add base as node scope
            node_cls = cast(type[BasedNode], NODE_CLASS_BY_TYPE[node_type])
            base_ptr = node_cls.get_base_from_data(node_data)
            if base_ptr is not None:
                node_scopes_by_id[cast(str, base_ptr.id)] = base_ptr
        node_scopes_by_id[node_data.id] = node_scope

        # graph scope
        graph_scope = edit.scope
        graph_scope_hash = hash((graph_scope.bench_id, graph_scope.package_id))
        if graph_scope_hash not in graph_scopes:
            graph_scopes[graph_scope_hash] = graph_scope

        # validate node scope with data
        # NOTE :Cleanup: not sure where to validate node *data* scopes
        #  e.g., we want to check that bench_ptr and package_ptr are correct
        #   but they are only present in NodeData, not in Nodes (where they are computed, not set).
        validate_node_scope(node_data, graph_scope)

    node_scopes: dict[UUID, NodeReference] = {
        UUID(k): cast(NodeReference, wiring.unpack_struct(v)) for k, v in node_scopes_by_id.items()
    }
    node_scopes_by_type = group_by(node_scopes.values(), lambda n: n.type)
    return _EditScopes(
        edited_node_ids=edited_node_ids,
        scopes_by_type=node_scopes_by_type,
        graph_scopes=tuple(graph_scopes.values()),
    )


def _is_allowable_drift(dt: datetime, now: datetime) -> bool:
    """Check if the given datetime is within the allowed time drift."""
    return abs((now - dt).total_seconds()) <= MAX_TIME_DRIFT_SECONDS


def _validate_edit(edit: EditData, subject: Subject, now: datetime) -> None:
    """Checks the given edit for basic validity."""
    assert subject.client, f"{subject!r} has no client"
    node_data = wiring.unwrap_some_node(edit.node)
    node_cls = NODE_CLASS_BY_TYPE[cast(NodeType, edit.node_type)]

    # subject/origin match
    user_id = str(subject.user.id) if subject.user else None
    if subject.client.parent_type == NodeType.USER:
        # subject must match user
        if edit.type in (EditType.CREATE, EditType.UPSERT) and (
            not node_data.created_by_ptr or node_data.created_by_ptr.id != user_id
        ):
            raise GRPCError(
                GRPCStatus.PERMISSION_DENIED,
                f"created_by mismatch in {edit!r}: {node_data.created_by_ptr} != {user_id}",
            )
        if not node_data.updated_by_ptr or node_data.updated_by_ptr.id != user_id:
            raise GRPCError(
                GRPCStatus.PERMISSION_DENIED,
                f"updated_by mismatch in {edit!r}: {node_data.updated_by_ptr} != {user_id}",
            )
    else:
        # subject must be a Run/Server
        if edit.type in (EditType.CREATE, EditType.UPSERT) and (
            not node_data.created_by_ptr or node_data.created_by_ptr.type not in EDIT_SUBJECT_TYPES
        ):
            raise GRPCError(
                GRPCStatus.PERMISSION_DENIED,
                f"bad created_by in {edit!r}: {node_data.created_by_ptr!r}",
            )
        if not node_data.updated_by_ptr or node_data.updated_by_ptr.type not in EDIT_SUBJECT_TYPES:
            raise GRPCError(
                GRPCStatus.PERMISSION_DENIED,
                f"bad updated_by in {edit!r}: {node_data.updated_by_ptr!r}",
            )
    if not edit.origin or UUID(edit.origin.id) != subject.client.id:
        raise GRPCError(
            GRPCStatus.PERMISSION_DENIED,
            f"origin mismatch in {edit!r}: {edit.origin!r} != {subject.client!r}",
        )

    # specific edit type constraints
    if any(p in edit.properties for p in ALL_IMPLICIT_PROPERTIES_IDS):
        bad_properties = [
            node_cls.__properties_by_id__[p]
            for p in ALL_IMPLICIT_PROPERTIES_IDS
            if p in edit.properties
        ]
        raise GRPCError(
            GRPCStatus.PERMISSION_DENIED,
            f"implicit properties set explicitly in {edit!r}: {bad_properties!r}",
        )
    if edit.type in (EditType.CREATE, EditType.UPSERT):
        if not node_data.created_at:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"missing created_at in {edit!r}")
    if not node_data.updated_at:
        raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"missing updated_at in {edit!r}")
    if not _is_allowable_drift(node_data.updated_at, now):
        raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"updated_at in {edit!r} has drifted too much")
    if edit.type == EditType.ARCHIVE:
        if not node_data.archived_at:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"missing archived_at in {edit!r}")
        if not _is_allowable_drift(node_data.archived_at, now):
            raise GRPCError(
                GRPCStatus.INVALID_ARGUMENT, f"archived_at in {edit!r} has drifted too much"
            )
    if edit.type == EditType.UNARCHIVE:
        if node_data.archived_at:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"set archived_at in {edit!r}")
    if edit.type == EditType.SOFT_DELETE:
        if not node_data.deleted_at:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"missing deleted_at in {edit!r}")
        if not _is_allowable_drift(node_data.deleted_at, now):
            raise GRPCError(
                GRPCStatus.INVALID_ARGUMENT, f"deleted_at in {edit!r} has drifted too much"
            )
    if edit.type == EditType.RESTORE:
        if node_data.deleted_at:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"set deleted_at in {edit!r}")
    if edit.type == EditType.DELETE:
        if not node_data.deleted_at:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"missing deleted_at in {edit!r}")
        if not _is_allowable_drift(node_data.deleted_at, now):
            raise GRPCError(
                GRPCStatus.INVALID_ARGUMENT, f"deleted_at in {edit!r} has drifted too much"
            )
