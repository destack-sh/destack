import abc
import asyncio
from contextlib import asynccontextmanager
from datetime import datetime
from typing import AsyncIterator, Literal, Mapping, NamedTuple, Sequence, cast, final, override
from uuid import UUID

import pytz
import structlog
from google.protobuf.message import Message as ProtoMessage
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import Expression, NodeReference, SelectOptions, Session, Subject
from bench.language.access import (
    AccessError,
    adapt_read_query,
    evaluate_and_adapt_read,
    evaluate_edit,
    generate_access_matrix,
)
from bench.language.bench import Package
from bench.language.block import Block
from bench.language.connection import ChannelUnavailableError, GetOptions, GraphEngine
from bench.language.const import (
    BASED_NODE_TYPES,
    NODE_TYPES,
    EditType,
    NodeType,
    PolicyEffect,
    QueryType,
)
from bench.language.graph import NodeDataGraph, NodeGraph, NodeSuperGraph
from bench.language.node import EDIT_SUBJECT_TYPES, EMPTY_SCOPE, Node
from bench.language.query import NodeNotFoundError, QueryBuilder
from bench.language.session import RuntimeContext
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.language.transaction import edit_data_graph
from bench.language.validation import ValidationError, on_invalid_raise
from bench.language.value import unpack_proto_json, unpack_value_scalar_data
from bench.proto import wiring
from bench.proto.services import ServiceBase
from bench.proto.wire import (
    AggregateNodesRequest,
    AggregateNodesResponse,
    CommitTransactionRequest,
    CommitTransactionResponse,
    EditData,
    GetNodesRequest,
    GetNodesResponse,
    GraphIOBase,
    GraphScopeData,
    NodeReferenceData,
    SearchNodesRequest,
    SearchNodesResponse,
    WatchAggregateRequest,
    WatchAggregateResponse,
    WatchGetRequest,
    WatchGetResponse,
    WatchSearchRequest,
    WatchSearchResponse,
)
from bench.system.graph.connection import (
    AggregateConnection,
    ConnectionIndex,
    GetConnection,
    SearchConnection,
    WatchGetUpdateData,
    WatchSearchUpdateData,
)
from bench.utils.func import bittuple, group_by, to_uuid
from bench.utils.oracle import Oracle
from bench.utils.sync import RWLock
from bench.utils.tenacity import RetryOptions
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

MAX_TIME_DRIFT_SECONDS = get_from_env(
    "MAX_TIME_DRIFT_SECONDS",
    typ=int,
    default=60,
    description="Maximum allowable delta between our time and client transaction time",
)
COMMIT_RETRY = RetryOptions(max_attempts=3, retry_on=(ChannelUnavailableError,))


class GraphLock:
    """
    Locks for synchronizing graph operations.

    NOTE :Performance!: obviously, putting broad locks around graph access is not ideal,
     but we have to guarantee absolute order and integrity of any loaded graphs (esp. in Host).
    We must prevent sync failures with non-repeatable reads where a node is edited while being read,
     whether that's in a loaded graph or in a Postgres transaction or whatever.
    For instance, if not locking carefully, it can happen that we read & cache a Connection's Runs
     while simulatenously committing an Edit to that Run, and then the Connection is out of sync and
     maybe even invalid because the commit happened during the read. I can't think of a good way to
     fix this sort of issue without resorting to locks at some point.)
    We can probably optimize this by only locking some tighter critical sections
     if we rollback somehow on failure. Maybe we can even 'cache' apply some edits only in memory.
    We'll also eventually need to thread/shard the Host (maybe lock only on overlapping edits?).
    """

    def __init__(self) -> None:
        self._locks: dict[NodeType, RWLock] = {node_type: RWLock() for node_type in NODE_TYPES}

    @asynccontextmanager
    async def read(self, ctx: QueryBuilder | Literal["all"]):
        if isinstance(ctx, QueryBuilder):  # noqa: SIM108
            node_types = ctx.all_node_types
        else:
            node_types = NODE_TYPES
        node_types = sorted(node_types)  # for consistent lock order

        with tracer.start_as_current_span("graph.lock.read"):
            for node_type in node_types:
                await self._locks[node_type].acquire_read()

        try:
            yield
        finally:
            for node_type in reversed(node_types):
                await self._locks[node_type].release_read()

    @asynccontextmanager
    async def write(self, ctx: "CommitArea | Literal['all']"):
        if isinstance(ctx, CommitArea):  # noqa: SIM108
            node_types = ctx.node_types
        else:
            node_types = NODE_TYPES
        node_types = sorted(node_types)  # for consistent lock order

        with tracer.start_as_current_span("graph.lock.write"):
            for node_type in node_types:
                await self._locks[node_type].acquire_write()

        try:
            yield
        finally:
            for node_type in reversed(node_types):
                await self._locks[node_type].release_write()


class GraphIoServiceBase(ServiceBase, GraphIOBase, abc.ABC):
    """Common base for global & Bench-local graph I/O operations."""

    def __init__(
        self,
        *,
        bench_id: UUID | None,
        node_types: bittuple[NodeType],
        logger: structlog.BoundLogger,
        tracer: trace.Tracer,
        oracle: Oracle,
        scope: GraphScopeData,
    ):
        super().__init__(logger=logger, tracer=tracer, oracle=oracle)
        self.epoch: int = 0
        self.bench_id: UUID | None = bench_id
        self.node_types: bittuple[NodeType] = node_types
        self.connector = ConnectionIndex(owner=self, scope=scope, oracle=self.oracle)

        self._scope = scope
        self._graph_lock = GraphLock()

    @abc.abstractmethod
    def get_engines(self) -> tuple[GraphEngine, ...]:
        """Gets the graph engines available to this subgraph."""
        ...

    def _validate_request(self, request: ProtoMessage) -> None:
        """Validate a request message for this service."""
        scope: GraphScopeData = getattr(request, "scope", EMPTY_SCOPE._to_data())
        if to_uuid(scope.bench_id) != self.bench_id:
            raise GRPCError(
                GRPCStatus.INVALID_ARGUMENT, f"scope mismatch: {scope.bench_id} != {self.bench_id}"
            )

    async def start(self):
        self.tasks.start_scheduled(
            10, self.connector.gc_connections, task_id="gc_connections", skip_errors=False
        )
        asyncio.get_running_loop().set_task_factory(asyncio.eager_task_factory)

    @abc.abstractmethod
    def resolve_request_block(self, block_ptr: UUID | NodeReference) -> Block | None:
        """Resolve a block pointer from a request message."""
        ...

    @property
    def request_session_parent(self) -> Package | None:
        return None

    @property
    def split_reads(self) -> bool:
        return False

    @final
    def new_request_session(
        self,
        supergraph: NodeSuperGraph,
        *,
        engines: tuple[GraphEngine, ...] | None = None,
        readonly: bool = True,
        raw_commit: bool = False,
    ):
        """Gets a new session for processing a single request."""
        return Session(
            parent=self.request_session_parent,
            _is_readonly=readonly,
            _default_scope=self._scope,
            _engines=engines if engines is not None else self.get_engines(),
            _local_epoch=self.epoch,
            _on_commit_prepare=self._on_commit_prepare_hook if not raw_commit else None,
            _on_commit=self._on_commit_hook if not raw_commit else None,
            _on_commit_failed=self._on_commit_failed_hook if not raw_commit else None,
            _supergraph=supergraph,
            _split_read=self.split_reads,
            _oracle=self.oracle,
        )

    def _parse_commit(
        self, subject: Subject, context: RuntimeContext, edits: Sequence[EditData]
    ) -> "CommitArea":
        """Prepares and validates the edits for a commit."""
        area = extract_commit_area(edits, base_graph=None)
        now = self.oracle.utc()
        for edit in edits:
            validate_edit(edit, subject, now)
        return area

    @final
    async def _on_commit_prepare_hook(
        self,
        session: Session,
        graph: NodeGraph,
        data_graph: NodeDataGraph,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
    ) -> Sequence[EditData]:
        return await self.on_commit_prepare(
            session=session,
            graph=graph,
            data_graph=data_graph,
            context=None,
            edits=edits,
            cascaded_edits=cascaded_edits,
        )

    async def on_commit_prepare(
        self,
        session: Session,
        graph: NodeGraph,
        data_graph: NodeDataGraph,
        context: RuntimeContext | None,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
    ) -> Sequence[EditData]:
        """Extend a commit. Returns any new edits."""
        return []  # do nothing by default

    async def _on_commit_hook(
        self,
        session: Session,
        graph: NodeGraph,
        data_graph: NodeDataGraph,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
        new_edits: Sequence[EditData],
    ) -> None:
        assert session._local_epoch is not None, f"no system epoch in {session!r}"
        self.epoch = session._local_epoch
        await self.on_commit(
            session=session,
            graph=graph,
            data_graph=data_graph,
            edits=edits,
            cascaded_edits=cascaded_edits,
            new_edits=new_edits,
        )

    async def on_commit(
        self,
        session: Session,
        graph: NodeGraph,
        data_graph: NodeDataGraph,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
        new_edits: Sequence[EditData],
    ):
        """Handle an accepted commit."""
        self.connector.on_commit(data_graph, edits, cascaded_edits, self.epoch)  # update cache

    @final
    async def _on_commit_failed_hook(
        self,
        session: Session,
        exc: BaseException,
    ) -> None:
        await self.on_commit_failed(session=session, exc=exc)

    async def on_commit_failed(self, session: Session, exc: BaseException):
        """Handle a failed commit."""
        pass  # do nothing by default

    async def _do_commit(
        self,
        *,
        area: "CommitArea",
        scope: GraphScopeData,
        subject: Subject,
        context: RuntimeContext,
        edits: Sequence[EditData],
    ) -> tuple[Sequence[EditData], Sequence[EditData]]:
        """Commits some edits."""

        # pre-validate/prepare edits
        include_deleted = any(e.type == EditType.RESTORE for e in edits)

        async with self.new_request_session(
            supergraph=subject._supergraph, readonly=False
        ) as session:
            # read the affected nodes into a single graph for evaluation
            data_graph = NodeDataGraph(scope=self._scope, node_types=NODE_TYPES)
            with self.tracer.start_as_current_span("graph.commit.read"):
                for (base_ck, node_type), node_references in area.scopes_by_base_and_type.items():
                    node_type = wiring.unpack_enum(NodeType, node_type)
                    block = self.resolve_request_block(base_ck) if base_ck else None
                    select = (
                        SelectOptions(select_fields=list(block.fields))
                        if block is not None
                        else None
                    )
                    query = QueryBuilder(
                        type=QueryType.GET,
                        node_type=node_type,
                        base_block=block,
                        roots=node_references,
                        include_deleted=include_deleted,
                        select=select,
                    )
                    adapted_query = adapt_read_query(subject, query)
                    channel = await session._get_channel_for(
                        scope, adapted_query.all_node_types, include_deleted=False, is_readonly=True
                    )
                    connection = await channel.get(
                        adapted_query, GetOptions(live=False, mode="packed")
                    )
                    # merge result into data_graph (there may be duplicates)
                    for node_data in connection.result_data.graph.nodes:
                        if node_data.id not in data_graph:
                            data_graph.add(node_data)
                self.logger.trace("graph.commit.read", graph=data_graph)

            # check access
            with self.tracer.start_as_current_span("graph.commit.check_access"):
                matrix = generate_access_matrix(subject, data_graph, supergraph=session._supergraph)
                decision, accesses = evaluate_edit(matrix, data_graph, edits)
                if decision != PolicyEffect.ALLOW:
                    raise AccessError(accesses)

            # apply edits in copy to validate
            # (and update true 'old' values in prepass, simplify edits for sql engine)
            session.tx._track_edits(edits)  # (assign epochs)
            flat_edits = edit_data_graph(
                graph=data_graph, edits=edits, include_deleted=True, is_prepass=True
            )
            assert flat_edits and len(flat_edits) == len(edits), f"{flat_edits} != {edits}"
            unpacked_graph = wiring.unpack_node_graph(
                data_graph, supergraph=subject._supergraph, parent=None, session=session
            )
            for node_id in area.edited_node_ids:
                node = unpacked_graph.get(UUID(node_id))
                if node is None:
                    raise GRPCError(GRPCStatus.NOT_FOUND, f"{node_id} not found")
                node._validate_self(properties=(), invalid=on_invalid_raise)
                session._pending_nodes_by_id[node.id] = node

            # actually commit
            session.tx._add_pending_edits(flat_edits)
            _, cascaded_edits = await session.commit(_data_graph=data_graph)
            # NOTE :Robustness: edited nodes are 'disconnected' copies (from unpack) :StaleNodes
            #  So we should really untrack them (to disable further edits) or keep them in sync,
            #   but I'm not sure how that should work yet, and we need to edit them async sometimes
            #   (e.g. in the scheduler we try scheduling new Runs and then update them accordingly).

        return edits, cascaded_edits

    @override
    async def commit_transaction(
        self, request: "CommitTransactionRequest", headers: Mapping
    ) -> "CommitTransactionResponse":
        metadata = wiring.unpack_rpc_headers(headers)
        subject = await self.get_request_subject(request, metadata)
        assert subject.client is not None, f"no client for {subject!r}"

        # figure out context
        context = wiring.unpack_object_validate_maybe(
            request.context, supergraph=subject._supergraph, expect=RuntimeContext
        )
        if context is None:
            context = RuntimeContext(
                client=subject.client,
                server=subject.server,
                user=subject.user,
                _supergraph=subject._supergraph,
            )

        area = self._parse_commit(subject, context, request.edits)
        async with self._graph_lock.write(area):
            retry = COMMIT_RETRY.new(self.oracle)
            while retry.should_retry:
                retry.on_attempt()
                try:
                    _, cascaded_edits = await self._do_commit(
                        area=area,
                        scope=self._scope,
                        subject=subject,
                        context=context,
                        edits=request.edits,
                    )
                    break
                except BaseException as e:
                    self.logger.error(
                        "graph.commit.error", subject=subject, exc_info=e, span="current"
                    )
                    if not retry.on_error(e):
                        raise
                    await self.oracle.sleep(retry.get_wait_interval())
            else:
                raise retry.to_error("commit")

        self.logger.info(
            "graph.commit",
            subject=subject,
            request_edits=len(request.edits),
            epoch=self.epoch,
            span="current",
        )
        return CommitTransactionResponse(cascaded_edits=cascaded_edits, epoch=self.epoch)

    @override
    async def get_nodes(self, request: "GetNodesRequest", headers: Mapping) -> "GetNodesResponse":
        # check that there is at least one root
        if not request.roots:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no roots provided")
        # parse query & fetch
        metadata = wiring.unpack_rpc_headers(headers)
        subject = await self.get_request_subject(request, metadata)
        async with self.new_request_session(supergraph=subject._supergraph) as session:
            # build the query
            with self.tracer.start_as_current_span("graph.get.parse"):
                roots = [
                    wiring.unpack_object_validate(r, supergraph=None, expect=NodeReference)
                    for r in request.roots
                ]
                if not roots:
                    raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no roots provided")
                if any(not r.id for r in roots):
                    raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "root nodes must have an id")
                if request.block_ptr.metatype:
                    block_ptr = wiring.unpack_object(
                        request.block_ptr, supergraph=None, expect=NodeReference
                    )
                    block = self.resolve_request_block(block_ptr)
                    if block is None:
                        raise NodeNotFoundError(block_ptr)
                else:
                    block = None
                ancestor_types = [wiring.unpack_enum(NodeType, t) for t in request.ancestor_types]
                descendant_types = [
                    wiring.unpack_enum(NodeType, t) for t in request.descendant_types
                ]
                select = (
                    wiring.unpack_object_validate_maybe(
                        request.select, supergraph=None, expect=SelectOptions
                    )
                    or SelectOptions.default()
                )
                roots_by_type: dict[NodeType, list[NodeReference]] = group_by(
                    roots, lambda r: r.node_type
                )
                if len(roots_by_type) > 1:
                    raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "roots must be of the same type")
                node_type = next(iter(roots_by_type.keys()))
                query = QueryBuilder(
                    type=QueryType.GET,
                    node_type=wiring.unpack_enum(NodeType, node_type),
                    roots=roots,
                    base_block=block,
                    ancestor_types=ancestor_types,
                    descendant_types=descendant_types,
                    include_deleted=request.include_deleted,
                    select=select,
                )
                adapted_query = adapt_read_query(subject, query)

            # get nodes
            with self.tracer.start_as_current_span("graph.get.read") as span:
                async with self._graph_lock.read(query):
                    connection = await self.connector.connect(
                        query=adapted_query,
                        session=session,
                        connection_t=GetConnection,
                        cache=not request.no_cache,
                    )
                    result = connection.result
                    span.set_attributes(
                        {"connection_hash": connection.hash, "connection_token": connection.token}
                    )

        # check if all roots are found (if not optional)
        if not request.is_optional and any(
            cast(str, root.id) not in result.graph for root in request.roots
        ):
            missing_roots = tuple(root for root in roots if str(root.id) not in result.graph)
            raise GRPCError(GRPCStatus.NOT_FOUND, f"roots not found: {missing_roots}")

        # check access & prune result
        with self.tracer.start_as_current_span("graph.get.check_access"):
            matrix = generate_access_matrix(subject, result.graph, supergraph=session._supergraph)
            decision, accesses, adapted_nodes = evaluate_and_adapt_read(
                matrix,
                result.graph,
                root_node_type=node_type,
                query=adapted_query,
                required_nodes=request.roots,
            )
            if decision != PolicyEffect.ALLOW:
                raise AccessError(accesses)

        self.logger.info(
            "graph.get",
            subject=subject,
            query=query,
            connection=connection,
            graph=result.graph,
            nodes=len(adapted_nodes),
            epoch=self.epoch,
            span="current",
        )
        return GetNodesResponse(
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            connection_token=connection.token,
            epoch=self.epoch,
        )

    @override
    async def watch_get(
        self, request: WatchGetRequest, headers: Mapping
    ) -> AsyncIterator[WatchGetResponse]:
        metadata = wiring.unpack_rpc_headers(headers)
        subject = await self.get_request_subject(request, metadata)
        subscription = await self.connector.subscribe(
            subject=subject,
            connection_t=GetConnection,
            update_t=WatchGetUpdateData,
            connection_token=request.connection_token,
            since_epoch=request.since_epoch,
        )
        try:
            self.logger.info(
                "graph.watch_get",
                subject=subject,
                subscription=subscription,
                connection=subscription.connection,
                epoch=self.epoch,
                span="current",
            )
            while True:
                update = await subscription.queue.get()
                yield WatchGetResponse(
                    edits=update.edits,
                    cascaded_edits=update.cascaded_edits,
                    added_nodes=[wiring.wrap_some_node(n) for n in update.added_nodes],
                    removed_nodes_ptr=update.removed_nodes_ptr,
                    epoch=update.epoch,
                )
        finally:
            subscription.cancel()

    @override
    async def search_nodes(
        self, request: "SearchNodesRequest", headers: Mapping
    ) -> "SearchNodesResponse":
        # parse query & fetch
        metadata = wiring.unpack_rpc_headers(headers)
        subject = await self.get_request_subject(request, metadata)
        async with self.new_request_session(supergraph=subject._supergraph) as session:
            # build the query
            with self.tracer.start_as_current_span("graph.search.parse"):
                node_type: NodeType = wiring.unpack_enum(NodeType, request.node_type)
                if request.block_ptr.metatype:
                    block_ptr = wiring.unpack_object(
                        request.block_ptr, supergraph=None, expect=NodeReference
                    )
                    block = self.resolve_request_block(block_ptr)
                    if block is None:  # raising here is not great.. :SearchWithMissingBlock
                        raise NodeNotFoundError(block_ptr)
                else:
                    block = None
                filter = wiring.unpack_object_validate_maybe(
                    request.filter, supergraph=session._supergraph, expect=Expression
                )
                sort = [
                    wiring.unpack_object_validate(
                        s, supergraph=session._supergraph, expect=Expression
                    )
                    for s in request.sort
                ] or []
                ancestor_types = [wiring.unpack_enum(NodeType, t) for t in request.ancestor_types]
                descendant_types = [
                    wiring.unpack_enum(NodeType, t) for t in request.descendant_types
                ]
                select = (
                    wiring.unpack_object_validate_maybe(
                        request.select, supergraph=None, expect=SelectOptions
                    )
                    or SelectOptions.default()
                )
                query = QueryBuilder(
                    type=QueryType.SEARCH,
                    node_type=node_type,
                    base_block=block,
                    filter=filter,
                    sort=sort,
                    ancestor_types=ancestor_types,
                    descendant_types=descendant_types,
                    first=request.first or None,
                    skip=request.skip or None,
                    select=select,
                )
                adapted_query = adapt_read_query(subject, query)

            # read the nodes
            with self.tracer.start_as_current_span("graph.search.read") as span:
                async with self._graph_lock.read(query):
                    connection = await self.connector.connect(
                        query=adapted_query,
                        session=session,
                        connection_t=SearchConnection,
                        cache=not request.no_cache,
                    )
                    result = connection.result
                    span.set_attributes(
                        {"connection_hash": connection.hash, "connection_token": connection.token}
                    )

        # check access & prune result
        with self.tracer.start_as_current_span("graph.search.check_access"):
            matrix = generate_access_matrix(subject, result.graph, supergraph=session._supergraph)
            decision, accesses, adapted_nodes = evaluate_and_adapt_read(
                matrix,
                result.graph,
                root_node_type=node_type,
                query=adapted_query,
                required_nodes=[request.block_ptr] if request.block_ptr.metatype else [],
            )
            if decision != PolicyEffect.ALLOW:
                raise AccessError(accesses)

        self.logger.info(
            "graph.search",
            subject=subject,
            query=query,
            connection=connection,
            graph=result.graph,
            nodes=len(adapted_nodes),
            epoch=self.epoch,
            span="current",
        )
        return SearchNodesResponse(
            roots_ptr=result.roots_ptr,
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            total=result.total,
            connection_token=connection.token,
            epoch=self.epoch,
        )

    @override
    async def watch_search(
        self, request: WatchSearchRequest, headers: Mapping
    ) -> AsyncIterator[WatchSearchResponse]:
        metadata = wiring.unpack_rpc_headers(headers)
        subject = await self.get_request_subject(request, metadata)
        subscription = await self.connector.subscribe(
            subject=subject,
            connection_t=SearchConnection,
            update_t=WatchSearchUpdateData,
            connection_token=request.connection_token,
            since_epoch=request.since_epoch,
        )
        try:
            self.logger.info(
                "graph.watch_search",
                subject=subject,
                subscription=subscription,
                connection=subscription.connection,
                epoch=self.epoch,
                span="current",
            )
            while True:
                update = await subscription.queue.get()
                yield WatchSearchResponse(
                    edits=update.edits,
                    cascaded_edits=update.cascaded_edits,
                    added_nodes=[wiring.wrap_some_node(n) for n in update.added_nodes],
                    removed_nodes_ptr=update.removed_nodes_ptr,
                    roots_ptr=update.roots_ptr,
                    total=update.total,
                    epoch=update.epoch,
                )
        finally:
            subscription.cancel()

    @override
    async def aggregate_nodes(
        self, request: "AggregateNodesRequest", headers: Mapping
    ) -> "AggregateNodesResponse":
        # parse query & fetch
        metadata = wiring.unpack_rpc_headers(headers)
        subject = await self.get_request_subject(request, metadata)
        async with self.new_request_session(supergraph=subject._supergraph) as session:
            with self.tracer.start_as_current_span("graph.aggregate.parse"):
                node_type: NodeType = wiring.unpack_enum(NodeType, request.node_type)
                filter = wiring.unpack_object_validate_maybe(
                    request.filter, supergraph=session._supergraph, expect=Expression
                )
                aggregation = wiring.unpack_object_validate(
                    request.aggregation, supergraph=session._supergraph, expect=Expression
                )
                query = QueryBuilder(
                    type=QueryType.AGGREGATE,
                    node_type=node_type,
                    filter=filter,
                    aggregation=aggregation,
                )
                adapted_query = adapt_read_query(subject, query)

            with self.tracer.start_as_current_span("graph.aggregate.read") as span:
                async with self._graph_lock.read(query):
                    connection = await self.connector.connect(
                        query=adapted_query,
                        session=session,
                        connection_t=AggregateConnection,
                        cache=not request.no_cache,
                    )
                    span.set_attributes(
                        {"connection_hash": connection.hash, "connection_token": connection.token}
                    )

        # TODO :Security!: check aggregation access

        self.logger.debug("graph.aggregate", subject=subject, epoch=self.epoch, span="current")
        return AggregateNodesResponse(
            aggregation=connection.result.aggregation,
            connection_token=connection.token,
            epoch=self.epoch,
        )

    @override
    async def watch_aggregate(
        self, request: WatchAggregateRequest, headers: Mapping
    ) -> AsyncIterator[WatchAggregateResponse]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED, "watch_aggregate not yet supported")
        yield  # unreachable (for type checking)


class CommitArea(NamedTuple):
    """The scope of relevant nodes for a transaction."""

    edited_node_ids: set[str]
    node_types: set[NodeType]
    scopes_by_base_and_type: dict[tuple[UUID | None, NodeType], list[NodeReference]]
    graph_scopes: tuple[GraphScopeData, ...]


@tracer.start_as_current_span(name="graph.extract_commit_area")
def extract_commit_area(edits: Sequence[EditData], base_graph: NodeDataGraph | None) -> CommitArea:
    """
    Gets the specific nodes (scopes) and related snodes that are edited. :NodeEditScope
    """
    from bench.proto import wiring

    edited_node_ids: set[str] = set()
    node_types: set[NodeType] = set()
    node_scopes_by_id: dict[str, NodeReferenceData] = {}
    graph_scopes: dict[int, GraphScopeData] = {}
    in_tx_created_nodes_ids: set[str] = set()

    for edit in edits:
        node_type = NodeType(edit.node_ptr.node_type)
        node_id = edit.node_ptr.id
        assert node_id, f"missing id for {edit!r}"
        edited_node_ids.add(node_id)
        node_types.add(node_type)
        if edit.type == EditType.CREATE or edit.type == EditType.UPSERT:
            assert edit.HasField("node_data"), f"missing node_data for {edit!r}"
            new_node = wiring.unwrap_some_node(edit.node_data)
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
                for op in reversed(edit.operations):
                    prop_id = int(op.path[0])
                    if prop_id == 4:
                        parent_ptr = unpack_value_scalar_data(
                            unpack_proto_json(op.new_value_packed),
                            Node.get_property("parent_ptr").type_info,
                        )
                        parent_ptr = cast(NodeReferenceData, parent_ptr)
                        break
                else:
                    raise RuntimeError(f"missing set parent_ptr for move {edit!r}")
                node_scopes_by_id[parent_ptr.id] = parent_ptr
                node_types.add(NodeType(parent_ptr.node_type))
        if node_type in BASED_NODE_TYPES and edit.node_ptr.base_ck is not None:
            # also add base as node scope
            assert base_graph is not None, f"missing base graph for {edit!r}"
            # add current base (base is immutable)
            old_base_node = base_graph.get(edit.node_ptr.base_ck)
            if old_base_node is not None:
                old_base_ptr = NodeReference._ref_data_from_node_data(old_base_node)
                assert old_base_ptr.id, f"missing base id for {old_base_ptr!r} in {edit!r}"
                node_scopes_by_id[old_base_ptr.id] = old_base_ptr
                node_types.add(NodeType(old_base_ptr.node_type))
        node_scopes_by_id[node_id] = node_scope

        # graph scope
        graph_scope = edit.scope
        graph_scope_hash = hash((graph_scope.bench_id, graph_scope.package_id))
        if graph_scope_hash not in graph_scopes:
            graph_scopes[graph_scope_hash] = graph_scope

    node_scopes: dict[UUID, NodeReference] = {
        UUID(k): wiring.unpack_object(v, supergraph=None, expect=NodeReference)
        for k, v in node_scopes_by_id.items()
    }
    node_scopes_by_type = group_by(node_scopes.values(), lambda n: (n.base_ck, n.node_type))
    return CommitArea(
        edited_node_ids=edited_node_ids,
        node_types=node_types,
        scopes_by_base_and_type=node_scopes_by_type,
        graph_scopes=tuple(graph_scopes.values()),
    )


def _is_allowable_drift(dt: datetime, now: datetime) -> bool:
    """Check if the given datetime is within the allowed time drift."""
    return abs((now - dt).total_seconds()) <= MAX_TIME_DRIFT_SECONDS


def validate_edit(edit: EditData, subject: Subject, now: datetime) -> None:
    """Checks the given (non-system) edit for basic validity."""
    assert subject.client, f"{subject!r} has no client"
    node_type = NodeType(edit.node_ptr.node_type)
    node_cls = NODE_CLASS_BY_TYPE[node_type]

    # scope
    if node_cls.__is_in_bench__ and not edit.scope.bench_id:
        raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"missing bench_id in {edit!r}")
    if node_cls.__is_in_package__ and not edit.scope.package_id:
        raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"missing package_id in {edit!r}")

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
        if not edit.subject_ptr or edit.subject_ptr.node_type not in EDIT_SUBJECT_TYPES:
            raise GRPCError(
                GRPCStatus.PERMISSION_DENIED, f"bad created_by in {edit!r}: {edit.subject_ptr!r}"
            )
    # origin
    if not edit.origin or UUID(edit.origin.id) != subject.client.id:
        raise GRPCError(
            GRPCStatus.PERMISSION_DENIED,
            f"origin mismatch in {edit!r}: {edit.origin!r} != {subject.client!r}",
        )

    # time
    if not edit.edited_at or not _is_allowable_drift(
        edit.edited_at.ToDatetime(tzinfo=pytz.utc), now
    ):
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT,
            f"bad edited_at {edit.edited_at} in {edit!r}: {edit.edited_at} !~= {now}",
        )

    # node data :EditData
    should_set_node_data = edit.type in (
        EditType.CREATE,
        EditType.UPSERT,
        EditType.DELETE,
        EditType.RESTORE,
        EditType.ERASE,
    )
    if should_set_node_data != edit.HasField("node_data"):
        raise GRPCError(
            GRPCStatus.INVALID_ARGUMENT,
            f"bad node_data in {edit!r}: {edit.node_data}",
        )
    # properties
    if edit.operations and edit.type not in (EditType.UPDATE, EditType.MOVE):
        raise GRPCError(GRPCStatus.INVALID_ARGUMENT, f"cannot add operations to {edit!r}")
