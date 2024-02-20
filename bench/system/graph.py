from collections import deque
from contextlib import asynccontextmanager
from typing import AsyncIterator, AsyncContextManager, Optional, TYPE_CHECKING
from uuid import UUID

import betterproto
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
import structlog

from bench.language import Subject, NodeReference, ReadOptions, Expression, Session, C
from bench.language.access import (
    adapt_read_options,
    generate_access_matrix,
    evaluate_and_adapt_read,
    get_edited_scopes,
    evaluate_edit,
    Request,
    AccessError,
)
from bench.language.const import (
    NodeType,
    ABOVE_SOURCE_NODE_TYPES,
    PolicyEffect,
    ConditionalOp,
)
from bench.language.graph import NodeDataGraph, NodeGraph
from bench.language.node import Node
from bench.language.query import StoreEngine, QueryBuilder, FetchOptions
from bench.proto import wiring
from bench.proto.services import BenchServiceBase
from bench.proto.wire import (
    EditData,
    GraphIoBase,
    GetNodesResponse,
    SearchNodesResponse,
    AggregateNodesResponse,
    CommitTransactionResponse,
    CompleteTransactionRequest,
    CompleteTransactionResponse,
    CancelTransactionRequest,
    CancelTransactionResponse,
    WatchEditsRequest,
    WatchEditsResponse,
    GetNodesRequest,
    SearchNodesRequest,
    CommitTransactionRequest,
    AggregateNodesRequest,
    FlushTransactionResponse,
    FlushTransactionRequest,
    GraphScope,
)
from bench.utils.func import group_by, bytetuple, to_uuid

logger = structlog.get_logger(__name__)

EPOCH_BUFFER_SIZE = 1000


class GraphIoService(GraphIoBase, BenchServiceBase if TYPE_CHECKING else object):
    """Common base for global & Bench-local graph I/O operations."""

    def __init__(self, *, bench_id: UUID | None, node_types: bytetuple[NodeType]):
        super().__init__()
        self.epoch: int = 0
        self.recent_epochs: deque[list[EditData]] = deque(maxlen=EPOCH_BUFFER_SIZE)
        self.bench_id: UUID | None = bench_id
        self.node_types: bytetuple[NodeType] = node_types

    def on_graph_edited(
        self, edits: list[EditData], source_graph: NodeDataGraph, graph: Optional[NodeGraph] = None
    ):
        # nocheckin: track, buffer and broadcast edits for recent epochs
        self.recent_epochs.appendleft(edits)
        # self.watchers....
        self.epoch += 1

    @property
    def engines(self) -> tuple[StoreEngine, ...]:
        """Gets the store engines available to this subgraph. Implemented in the actual service."""
        raise NotImplementedError

    def _validate_request_self(self, subject: Subject, request: betterproto.Message) -> None:
        """Validate a request message for this service."""
        scope: GraphScope | None = getattr(request, "scope", None)
        assert scope is not None, f"{request!r} is missing 'scope' property required for {self!r}"
        if to_uuid(scope.bench_id) != self.bench_id:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "service scope mismatch")

    @asynccontextmanager
    async def session(self) -> AsyncContextManager[Session]:
        async with Session(parent=None, _engines=self.engines) as session:
            yield session

    async def check_and_log_request(self, request: Request):
        logger.debug(f"request.{request.decision.name.lower()}", request=request)
        if request.decision == PolicyEffect.DENY:
            raise AccessError(request)

    async def get_nodes(self, subject: Subject, request: "GetNodesRequest") -> "GetNodesResponse":
        roots: tuple[NodeReference, ...] = tuple(wiring.unpack_struct(r) for r in request.roots)
        if not roots:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no roots provided")
        if any(root.type not in ABOVE_SOURCE_NODE_TYPES for root in roots):
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO can't read packages")
        options: ReadOptions = (
            wiring.unpack_struct_interp_maybe(request.options) or ReadOptions.default()
        )

        roots_by_type: dict[NodeType, list[NodeReference]] = group_by(roots, lambda r: r.type)
        graph = NodeDataGraph()
        async with self.session() as session:
            session: Session
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
                connection = await session.tx.connect_to_store_for(request.scope, node_type)
                result = await connection.fetch(query, FetchOptions(count=False))
                graph.extend(result.nodes)
        if any(root.id not in graph for root in request.roots):
            missing_roots = tuple(root for root in roots if str(root.id) not in graph)
            raise GRPCError(GRPCStatus.NOT_FOUND, f"roots not found: {missing_roots}")

        access = generate_access_matrix(subject, graph)
        evaluated_request, adapted_nodes = evaluate_and_adapt_read(
            access, graph, required_nodes=request.roots, adapt_nodes_in_place=True
        )
        await self.check_and_log_request(evaluated_request)

        return GetNodesResponse(
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            access=access._to_data(),
            epoch=self.epoch,
        )

    async def search_nodes(
        self, subject: Subject, request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
        if request.bases:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO has no bases")
        node_type: NodeType = wiring.unpack_enum(NodeType, request.node_type)
        if node_type not in ABOVE_SOURCE_NODE_TYPES:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO can't read packages")
        filter: Expression | None = wiring.unpack_struct_interp_maybe(request.filter)
        sort: list[Expression] = [wiring.unpack_struct_interp(s) for s in request.sort] or None
        options: ReadOptions = (
            wiring.unpack_struct_interp_maybe(request.options) or ReadOptions.default()
        )

        adapted_options = adapt_read_options(subject, node_type, options)
        async with self.session() as session:
            session: Session
            query = QueryBuilder(
                node_type=node_type, filter=filter, options=adapted_options, sort=sort
            )
            connection = await session.tx.connect_to_store_for(request.scope, node_type)
            result = await connection.fetch(query, FetchOptions(count=request.count))
            graph = NodeDataGraph(result.nodes)
        access = generate_access_matrix(subject, graph)
        evaluated_request, adapted_nodes = evaluate_and_adapt_read(
            access, graph, adapt_nodes_in_place=True, required_nodes=request.bases
        )
        await self.check_and_log_request(evaluated_request)

        return SearchNodesResponse(
            roots=[NodeReference.from_node_data(r) for r in result.nodes],
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            cursors=list(result.cursors),
            start_cursor=result.start_cursor,
            total=result.total,
            access=access._to_data(),
            epoch=self.epoch,
        )

    async def aggregate_nodes(
        self, subject: Subject, request: "AggregateNodesRequest"
    ) -> "AggregateNodesResponse":
        if request.bases:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO has no bases")
        node_type: NodeType = wiring.unpack_enum(NodeType, request.node_type)
        if node_type not in ABOVE_SOURCE_NODE_TYPES:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO can't read packages")
        filter: Expression | None = wiring.unpack_struct_interp_maybe(request.filter)
        aggregation: Expression = wiring.unpack_struct_interp(request.aggregation)

        adapted_options = adapt_read_options(subject, node_type, ReadOptions())
        async with self.session() as session:
            session: Session
            query = QueryBuilder(
                node_type=node_type, filter=filter, options=adapted_options, aggregation=aggregation
            )
            connection = await session.tx.connect_to_store_for(request.scope, node_type)
            result = await connection.aggregate(query)

        return AggregateNodesResponse(aggregation=result.aggregation, epoch=self.epoch)

    async def commit_transaction(
        self, subject: Subject, request: "CommitTransactionRequest"
    ) -> "CommitTransactionResponse":
        # figure out the node (scopes) we need to evaluate the edit
        edited_scopes_ptr: dict[UUID, NodeReference] = get_edited_scopes(request.edits)
        edited_scopes_by_type: dict[NodeType, list[NodeReference]] = group_by(
            edited_scopes_ptr.values(), lambda r: r.type
        )
        async with self.session() as session:
            session: Session
            # read the required nodes into a single graph for evaluation
            graph = NodeDataGraph()
            for node_type, node_references in edited_scopes_by_type.items():
                node_type = wiring.unpack_enum(NodeType, node_type)
                # TODO @Performance: select only require properties for edit eval (id/policies/...?)
                adapted_options = adapt_read_options(subject, node_type, ReadOptions.default())
                query = QueryBuilder(
                    node_type=node_type,
                    filter=C(
                        ConditionalOp.IN,
                        property=Node.id,
                        value=tuple(r.id for r in node_references),
                    ),
                    options=adapted_options,
                )
                connection = await session.tx.connect_to_store_for(request.scope, node_type)
                result = await connection.fetch(query, FetchOptions(count=False))
                graph.extend(result.nodes)
                if any(str(r.id) not in graph for r in node_references):
                    missing = tuple(r for r in node_references if str(r.id) not in graph)
                    raise GRPCError(GRPCStatus.NOT_FOUND, f"edited scopes not found: {missing}")

            # nocheckin: validate the edits in language (where? in tx when applying to source?)

            # evaluate edit access
            matrix = generate_access_matrix(subject, graph)
            evaluated_request = evaluate_edit(matrix, graph, request.edits)
            await self.check_and_log_request(evaluated_request)

            # apply the edits
            session.tx._add_pending_edits(request.edits)
            await session.commit()
            self.on_graph_edited(request.edits, graph)

        accepted_revisions = [e.revision for e in request.edits]
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

    async def watch_edits(
        self, subject: Subject, request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
