from contextlib import asynccontextmanager
from typing import AsyncIterator, Collection
from uuid import UUID

from grpclib import GRPCError
from grpclib import Status as GRPCStatus
import structlog

from bench.language import Subject, NodeReference, ReadOptions, Expression, Aggregation
from bench.language.access import (
    adapt_read_options,
    generate_access_matrix,
    evaluate_and_adapt_read,
    get_edited_scopes,
    evaluate_edit,
    Request,
    AccessError,
)
from bench.language.const import NodeType, ABOVE_SOURCE_NODE_TYPES, AggregationOp, PolicyEffect
from bench.language.graph import NodeDataGraph
from bench.language.node import NODE_CLASS_BY_TYPE
from bench.language.query import StoreEngine
from bench.proto import wiring
from bench.proto.wire import (
    GraphScope,
    EditData,
    GraphIoBase,
    GetNodesResponse,
    SearchNodesResponse,
    AggregateNodesResponse,
    AnyNodeData,
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
)
from bench.sql.engine import (
    pg_get_node_data_graph,
    pg_search_nodes_data_graph,
    pg_count,
    compile_pg_conditional,
    pg_exists,
    pg_write_regular_edits,
)
from bench.system.utils import global_pg_cursor
from bench.utils.func import group_by

logger = structlog.get_logger(__name__)


class GraphIoService(GraphIoBase):
    """Common base for global & Bench-local graph I/O operations."""

    def __init__(self):
        self.epoch: int = 0

    def on_edited_graph(self, edits: list[EditData]):
        # nocheckin: track and buffer edits for recent epochs
        # self.watchers....
        self.epoch += 1

    def get_engines_for(
        self, subject: Subject, scope: GraphScope, node_type: NodeType
    ) -> tuple[StoreEngine, ...]:
        # nocheckin: use Session/Transaction for GraphIoService
        raise NotImplementedError

    @asynccontextmanager
    async def session_for(
        self, subject: Subject, scope: GraphScope, root_node_types: Collection[NodeType]
    ):
        raise NotImplementedError

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
        async with global_pg_cursor() as cur:
            for root_node_type, root_node_references in roots_by_type.items():
                adapted_options = adapt_read_options(subject, root_node_type, options)
                node_type = wiring.unpack_enum(NodeType, root_node_type)
                _ = await pg_get_node_data_graph(
                    cur=cur,
                    root_type=node_type,
                    roots=tuple(r.id for r in root_node_references),
                    options=adapted_options,
                    _graph=graph,  # accumulate into graph
                )
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
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        filter: Expression | None = wiring.unpack_struct_interp_maybe(request.filter)
        sort: list[Expression] = [wiring.unpack_struct_interp(s) for s in request.sort] or None
        options: ReadOptions = (
            wiring.unpack_struct_interp_maybe(request.options) or ReadOptions.default()
        )

        adapted_options = adapt_read_options(subject, node_type, options)
        async with global_pg_cursor() as cur:
            combined_filter = adapted_options.filter(node_type, filter)
            roots, graph = await pg_search_nodes_data_graph(
                cur=cur,
                node_type=node_type,
                filter=combined_filter,
                sort=sort,
                first=request.first,
                after=request.after,
                options=adapted_options,
            )
            if request.count:
                count = await pg_count(
                    cur=cur,
                    table=node_cls.__table__,
                    where=compile_pg_conditional(node_cls, combined_filter),
                )
            else:
                count = None
        access = generate_access_matrix(subject, graph)
        evaluated_request, adapted_nodes = evaluate_and_adapt_read(
            access, graph, adapt_nodes_in_place=True, required_nodes=request.bases
        )
        await self.check_and_log_request(evaluated_request)

        return SearchNodesResponse(
            roots=[NodeReference.from_node_data(r) for r in roots.nodes],
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            cursors=list(roots.cursors),
            start_cursor=roots.start_cursor,
            total=count,
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
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        filter: Expression | None = wiring.unpack_struct_interp_maybe(request.filter)
        aggregation: Expression = wiring.unpack_struct_interp(request.aggregation)

        adapted_options = adapt_read_options(subject, node_type, ReadOptions())
        async with global_pg_cursor() as cur:
            combined_filter = adapted_options.filter(node_type, filter)
            if aggregation.op == AggregationOp.EXISTS:
                exists = await pg_exists(
                    cur=cur,
                    table=node_cls.__table__,
                    where=compile_pg_conditional(node_cls, combined_filter),
                )
                result = Aggregation(op=aggregation.op, exists=exists)
            elif aggregation.op == AggregationOp.COUNT:
                count = await pg_count(
                    cur=cur,
                    table=node_cls.__table__,
                    where=compile_pg_conditional(node_cls, combined_filter),
                )
                result = Aggregation(op=aggregation.op, count=count)
            else:
                raise GRPCError(GRPCStatus.UNIMPLEMENTED, f"can't {aggregation.op.name} yet")

        return AggregateNodesResponse(aggregation=result._to_data(), epoch=self.epoch)

    async def commit_transaction(
        self, subject: Subject, request: "CommitTransactionRequest"
    ) -> "CommitTransactionResponse":
        # figure out the node (scopes) we need to evaluate the edit
        edited_scopes_ptr: dict[UUID, NodeReference] = get_edited_scopes(request.transaction.edits)
        edited_scopes_by_type: dict[NodeType, list[NodeReference]] = group_by(
            edited_scopes_ptr.values(), lambda r: r.type
        )
        async with global_pg_cursor() as cur:
            # read the required nodes into a single graph for evaluation
            graph = NodeDataGraph()
            for node_type, node_references in edited_scopes_by_type.items():
                node_type = wiring.unpack_enum(NodeType, node_type)
                # TODO @Performance: select only require properties for edit eval (id/policies/...?)
                adapted_options = adapt_read_options(subject, node_type, ReadOptions.default())
                _ = await pg_get_node_data_graph(
                    cur=cur,
                    root_type=node_type,
                    roots=tuple(r.id for r in node_references),
                    options=adapted_options,
                    _graph=graph,  # accumulate into graph
                )
                if any(str(r.id) not in graph for r in node_references):
                    missing = tuple(r for r in node_references if str(r.id) not in graph)
                    raise GRPCError(GRPCStatus.NOT_FOUND, f"edited scopes not found: {missing}")

            # evaluate the edits
            matrix = generate_access_matrix(subject, graph)
            evaluated_request = evaluate_edit(matrix, graph, request.transaction.edits)
            await self.check_and_log_request(evaluated_request)

            # apply the edits
            changed_nodes: list[AnyNodeData] = await pg_write_regular_edits(
                cur=cur, edits=request.transaction.edits, return_nodes=True
            )
            await cur.connection.commit()
            self.on_edited_graph(request.transaction.edits)

        return CommitTransactionResponse(
            changed_nodes=[wiring.wrap_some_node(n) for n in changed_nodes],
            epoch=self.epoch,
        )

    async def complete_transaction(
        self, subject: Subject, request: "CompleteTransactionRequest"
    ) -> "CompleteTransactionResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def cancel_transaction(
        self, subject: Subject, request: "CancelTransactionRequest"
    ) -> "CancelTransactionResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def watch_edits(
        self, subject: Subject, request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
