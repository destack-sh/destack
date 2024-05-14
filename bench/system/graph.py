import asyncio
from collections import deque
from contextlib import asynccontextmanager
from dataclasses import dataclass, field
from itertools import chain
from typing import AsyncIterator, Mapping, NamedTuple, cast, final
from uuid import UUID

import betterproto
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import C, Expression, NodeReference, ReadOptions, Session, Subject
from bench.language.access import (
    AccessError,
    Request,
    adapt_read_options,
    evaluate_and_adapt_read,
    evaluate_edit,
    generate_access_matrix,
)
from bench.language.connection import FetchOptions, StoreEngine
from bench.language.const import AccessKind, ConditionalOp, EditType, NodeType, PolicyEffect
from bench.language.graph import NodeDataGraph, edit_data_graph
from bench.language.node import Node
from bench.language.query import QueryBuilder
from bench.language.setup import NODE_CLASS_BY_TYPE
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
from bench.utils.func import bytetuple, group_by, partition, to_uuid

logger = structlog.get_logger(__name__)

EPOCH_BUFFER_SIZE = 1000  # every epoch is a set of edits


class Epoch(NamedTuple):
    epoch: int
    edits: list[EditData]


@dataclass(slots=True)
class EditWatcher:
    """An active subscriber to the watch_edits server stream."""

    subject: Subject
    node_types: bytetuple[NodeType]
    filters: Mapping[NodeType, Expression]
    sink: asyncio.Queue[Epoch] = field(default_factory=asyncio.Queue)

    def __str__(self):
        return f"{self.subject}: {'|'.join(n.bench_name for n in self.node_types.tuple)} [{self.filters}]"


def _check_nodes_in_same_store(
    roots: tuple[NodeType, ...] | tuple[NodeReference, ...] | tuple[NodeReferenceData],
    options: ReadOptions,
):
    """
    Check that all node types belong in the same store.
    TODO :Robustness: assign & check nodes/node types to 'stores' more explicitly
    """

    if roots and isinstance(roots[0], (NodeReference, NodeReferenceData)):
        roots_types = tuple((cast(NodeReference, r)).type for r in roots)
    else:
        roots_types = cast(list[NodeType], roots)

    has_global = False
    has_local = False

    for node_type in chain(roots_types, options.ancestor_types, options.descendant_types):
        if NODE_CLASS_BY_TYPE[node_type].__is_local__:
            has_local = True
        else:
            has_global = True

    if has_global and has_local:
        global_types, local_types = partition(
            lambda t: NODE_CLASS_BY_TYPE[t].__is_local__,
            chain(roots, options.ancestor_types, options.descendant_types),
        )
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT,
            f"can't mix global and local node types: {global_types} vs {local_types}",
        )


class GraphIoServiceBase(BenchServiceBase, GraphIoBase):
    """Common base for global & Bench-local graph I/O operations."""

    def __init__(self, *, bench_id: UUID | None, node_types: bytetuple[NodeType]):
        super().__init__()
        self.epoch: int = 0
        self.recent_epochs: deque[Epoch] = deque(maxlen=EPOCH_BUFFER_SIZE)
        self.bench_id: UUID | None = bench_id
        self.node_types: bytetuple[NodeType] = node_types
        self.watchers: list[EditWatcher] = []

    @property
    def engines(self) -> tuple[StoreEngine, ...]:
        """Gets the store engines available to this subgraph. Implemented in the actual service."""
        raise NotImplementedError

    def _validate_request_self(self, subject: Subject, request: betterproto.Message) -> None:
        """Validate a request message for this service."""
        scope: GraphScope = getattr(request, "scope", GraphScope())
        if to_uuid(scope.bench_id) != self.bench_id:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "service scope mismatch")

    @asynccontextmanager
    async def session(self):
        async with Session(parent=None, _engines=self.engines) as session:
            yield session

    async def check_and_log_request(self, request: Request):
        logger.debug(f"request.{request.decision.name.lower()}", request=request)
        if request.decision == PolicyEffect.DENY:
            raise AccessError(request)

    async def get_nodes(self, subject: Subject, request: "GetNodesRequest") -> "GetNodesResponse":
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
        _check_nodes_in_same_store(roots, options)

        roots_by_type: dict[NodeType, list[NodeReference]] = group_by(roots, lambda r: r.type)
        graph = NodeDataGraph()
        async with self.session() as session:
            for root_node_type, root_node_references in roots_by_type.items():
                adapted_options = adapt_read_options(subject, root_node_type, options)
                node_type = wiring.unpack_enum(NodeType, root_node_type)
                query = QueryBuilder(
                    node_type=node_type,
                    filter=C(
                        ConditionalOp.IN,
                        property=Node.id,
                        value=tuple(r.id for r in root_node_references),
                    ),
                    options=adapted_options,
                )
                connection = await session.tx.connect_store(
                    request.scope, node_type, AccessKind.READ
                )
                result = await connection.fetch(query, FetchOptions(count=False))
                graph.extend(result.nodes)
        if any(cast(str, root.id) not in graph for root in request.roots):
            missing_roots = tuple(root for root in roots if str(root.id) not in graph)
            raise GRPCError(GRPCStatus.NOT_FOUND, f"roots not found: {missing_roots}")

        access = generate_access_matrix(subject, graph)
        evaluated_request, adapted_nodes = evaluate_and_adapt_read(
            access, graph, required_nodes=request.roots, adapt_nodes_in_place=True
        )
        await self.check_and_log_request(evaluated_request)

        logger.info("graph.get", subject=subject, request=request, epoch=self.epoch)
        return GetNodesResponse(
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            access=cast(AccessMatrixData, access._to_data()),
            epoch=self.epoch,
        )

    async def search_nodes(
        self, subject: Subject, request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
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
        _check_nodes_in_same_store((node_type,), options)

        adapted_options = adapt_read_options(subject, node_type, options)
        async with self.session() as session:
            query = QueryBuilder(
                node_type=node_type, filter=filter, options=adapted_options, sort=sort
            )
            connection = await session.tx.connect_store(request.scope, node_type, AccessKind.READ)
            result = await connection.fetch(query, FetchOptions(count=request.count or False))
            graph = NodeDataGraph(result.nodes)
        access = generate_access_matrix(subject, graph)
        evaluated_request, adapted_nodes = evaluate_and_adapt_read(
            access, graph, adapt_nodes_in_place=True, required_nodes=request.bases
        )
        await self.check_and_log_request(evaluated_request)

        logger.info("graph.search", subject=subject, request=request, epoch=self.epoch)
        return SearchNodesResponse(
            roots=[NodeReference.from_node_data(r) for r in result.nodes],
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            cursors=list(result.cursors),
            start_cursor=result.start_cursor,
            total=result.total,
            access=cast(AccessMatrixData, access._to_data()),
            epoch=self.epoch,
        )

    async def aggregate_nodes(
        self, subject: Subject, request: "AggregateNodesRequest"
    ) -> "AggregateNodesResponse":
        if request.bases:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO has no bases")
        node_type: NodeType = wiring.unpack_enum(NodeType, request.node_type)
        filter: Expression | None = wiring.unpack_struct_interp_maybe(request.filter)
        aggregation: Expression = wiring.unpack_struct_interp(request.aggregation)

        adapted_options = adapt_read_options(subject, node_type, ReadOptions())
        async with self.session() as session:
            query = QueryBuilder(
                node_type=node_type, filter=filter, options=adapted_options, aggregation=aggregation
            )
            connection = await session.tx.connect_store(request.scope, node_type, AccessKind.READ)
            result = await connection.aggregate(query)

        logger.debug("graph.aggregate", subject=subject, request=request, epoch=self.epoch)
        return AggregateNodesResponse(aggregation=result.aggregation, epoch=self.epoch)

    async def commit_transaction(
        self, subject: Subject, request: "CommitTransactionRequest"
    ) -> "CommitTransactionResponse":
        # figure out the node (scopes) we need to evaluate the edit
        edited_scopes = get_validated_edited_scopes(request.edits)
        start = asyncio.get_event_loop().time()
        async with self.session() as session:
            # read the required nodes into a single graph for evaluation
            data_graph = NodeDataGraph()
            for node_type, node_references in edited_scopes.node_scopes_by_type.items():
                node_type = wiring.unpack_enum(NodeType, node_type)
                # TODO :Performance: select only properties required to evaluate edit (id/policies/...?)
                options = adapt_read_options(subject, node_type, ReadOptions.default())
                query = QueryBuilder(
                    node_type=node_type,
                    filter=C(
                        ConditionalOp.IN,
                        property=Node.id,
                        value=tuple(r.id for r in node_references),
                    ),
                    options=options,
                )
                connection = await session.tx.connect_store(
                    request.scope, node_type, AccessKind.EDIT
                )
                result = await connection.fetch(query, FetchOptions(count=False))

                # merge result into data_graph (there may be duplicates)
                for node in result.nodes:
                    if node.id not in data_graph:
                        data_graph.add(node)

                # all requested nodes must be present
                if any(str(r.id) not in data_graph for r in node_references):
                    missing = tuple(r for r in node_references if str(r.id) not in data_graph)
                    raise GRPCError(GRPCStatus.NOT_FOUND, f"edited scopes not found: {missing}")

            # check access
            matrix = generate_access_matrix(subject, data_graph)
            evaluated_request = evaluate_edit(matrix, data_graph, request.edits)
            await self.check_and_log_request(evaluated_request)

            # validate edits
            edit_data_graph(  # apply in copy & validate
                graph=data_graph,
                options=ReadOptions.all(),
                edits=request.edits,
                update_nodes_in_place=False,
            )
            unpacked_graph = wiring.unpack_node_graph(data_graph, parent=None, session=session)
            for node_id in edited_scopes.node_scopes_by_id:
                node = unpacked_graph.get(node_id)
                if node is None:
                    # this is an internal error (all edited nodes (incl. new) should be here)
                    raise RuntimeError(f"node {node_id} not found in unpacked {unpacked_graph!r}")
                node._validate_self(properties=(), invalid=on_invalid_raise)
            # TODO :Robustness! :Test: prevent circular parent/child references

            # apply edits
            session.tx._add_pending_edits(request.edits)
            await session.commit()
            logger.info(
                "graph.commit",
                subject=subject,
                request=request,
                edits=session.tx.edits,
                epoch=self.epoch,
                duration=asyncio.get_event_loop().time() - start,
            )
            self.on_graph_edited(edited_scopes.graph_scopes, request.edits)

        accepted_revisions = [cast(int, e.revision) for e in request.edits]
        return CommitTransactionResponse(revisions=accepted_revisions, epoch=self.epoch)

    async def flush_transaction(
        self, subject: "Subject", request: "FlushTransactionRequest"
    ) -> "FlushTransactionResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)  # :2PC

    async def complete_transaction(
        self, subject: Subject, request: "CompleteTransactionRequest"
    ) -> "CompleteTransactionResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)  # :2PC

    async def cancel_transaction(
        self, subject: Subject, request: "CancelTransactionRequest"
    ) -> "CancelTransactionResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)  # :2PC

    @final
    def on_graph_edited(self, scopes: tuple[GraphScope, ...], edits: list[EditData]):
        self.epoch += 1
        self.recent_epochs.append(Epoch(self.epoch, edits))

        # notify watchers
        for watcher in self.watchers:
            adapted_edits = self._filter_and_adapt_edits(watcher, edits)
            if adapted_edits:
                watcher.sink.put_nowait(Epoch(self.epoch, adapted_edits))

        self._on_graph_edited(scopes=scopes, edits=edits)

    def _on_graph_edited(self, scopes: tuple[GraphScope, ...], edits: list[EditData]):
        pass

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

    async def watch_edits(
        self, subject: Subject, request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        node_types = bytetuple(*tuple(wiring.unpack_enum(NodeType, t) for t in request.node_types))
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
                if num_epochs_to_replay > EPOCH_BUFFER_SIZE:
                    raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "too much to replay")
                epochs_to_replay = []
                for epoch, edits in reversed(self.recent_epochs):
                    if epoch <= request.since_epoch:
                        break
                    edits = self._filter_and_adapt_edits(watcher, edits)
                    epochs_to_replay.append((epoch, edits))
                if epochs_to_replay:
                    logger.info("graph.watch.replay", watcher=watcher, epochs=epochs_to_replay)
                    for epoch, edits in epochs_to_replay:
                        yield WatchEditsResponse(edits=edits, epoch=epoch)

            # listen for new epochs
            logger.info("graph.watch", watcher=watcher)
            while True:
                epoch = await watcher.sink.get()
                yield WatchEditsResponse(edits=epoch.edits, epoch=epoch.epoch)
        finally:
            self.watchers.remove(watcher)


class _EditScopes(NamedTuple):
    node_scopes_by_type: dict[NodeType, list[NodeReference]]
    node_scopes_by_id: dict[UUID, NodeReference]
    graph_scopes: tuple[GraphScope, ...]


def validate_node_scope(node_data: AnyNodeData, graph_scope: GraphScope):
    bench_ptr = getattr(node_data, "bench_ptr", None)
    if bench_ptr and bench_ptr.id != graph_scope.bench_id:
        raise ValidationError(
            node_data,
            f"node {node_data} has bench_id: {bench_ptr.id} != {graph_scope.bench_id}",
        )


def get_validated_edited_scopes(edits: list[EditData]) -> _EditScopes:
    """
    Gets the specific nodes (scopes) and broader graph scopes that are edited.
    Also verifies that the edited scopes match the nodes data.
    """
    from bench.proto import wiring

    node_scopes_by_id: dict[str, "NodeReferenceData"] = {}
    graph_scopes: dict[int, "GraphScope"] = {}
    just_created_nodes_id: set[str] = set()
    for edit in edits:
        # node scope :NodeEditScope
        node_data = wiring.unwrap_some_node(edit.node)
        if edit.type == EditType.CREATE or edit.type == EditType.UPSERT:
            # node scope is parent since we don't know this node yet
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
            else:
                node_scope = NodeReference.from_node_data(node_data)
        node_scopes_by_id[node_data.id] = node_scope

        # graph scope
        graph_scope = edit.scope
        graph_scope_hash = hash((graph_scope.bench_id, graph_scope.package_id))
        if graph_scope_hash not in graph_scopes:
            graph_scopes[graph_scope_hash] = graph_scope

        # validate node scope with data
        # NOTE :Cleanup: not sure where to validate node *data* scopes
        #  e.g., we want to check that bench_ptr and package_ptr are correct
        #   but they are only present in NodeData, not in Nodes (where they are computed).
        validate_node_scope(node_data, graph_scope)

    node_scopes: dict[UUID, NodeReference] = {
        UUID(k): cast(NodeReference, wiring.unpack_struct(v)) for k, v in node_scopes_by_id.items()
    }
    node_scopes_by_type = group_by(node_scopes.values(), lambda n: n.type)
    return _EditScopes(node_scopes_by_type, node_scopes, tuple(graph_scopes.values()))
