import abc
import asyncio
from datetime import datetime
from typing import AsyncIterator, NamedTuple, cast, final, override
from uuid import UUID

import betterproto
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.language import Expression, NodeReference, ReadOptions, Session, Subject
from bench.language.access import (
    AccessError,
    adapt_read_options,
    evaluate_and_adapt_read,
    evaluate_edit,
    generate_access_matrix,
)
from bench.language.bench import Package
from bench.language.connection import ChannelUnavailableError, GetOptions, GraphEngine
from bench.language.const import (
    BASED_NODE_TYPES,
    NODE_TYPES,
    EditType,
    NodeType,
    PolicyEffect,
    ReadType,
)
from bench.language.graph import (
    NodeDataDict,
    NodeDataGraph,
    NodeDataGraphLike,
    NodeDict,
    NodeGraphLike,
    NodeSuperGraph,
)
from bench.language.node import (
    EDIT_SUBJECT_TYPES,
    EMPTY_SCOPE,
    GraphScope,
    HasNodeBase,
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
    AggregateNodesRequest,
    AggregateNodesResponse,
    CommitTransactionRequest,
    CommitTransactionResponse,
    EditData,
    GetNodesRequest,
    GetNodesResponse,
    GraphIoBase,
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
from bench.system.connection import (
    CONNECTION_CACHE_ENABLED,
    MAX_TIME_DRIFT_SECONDS,
    AggregateConnection,
    ConnectionIndex,
    GetConnection,
    SearchConnection,
    WatchGetUpdate,
    WatchSearchUpdate,
)
from bench.utils.func import CriticalLock, bittuple, group_by, to_uuid
from bench.utils.oracle import Oracle
from bench.utils.tenacity import RetryOptions

COMMIT_RETRY = RetryOptions(max_attempts=3, retry_on=(ChannelUnavailableError,))


class DoCommitRet(NamedTuple):
    epoch: int
    edits: list[EditData]
    new_edits: list[EditData]
    cascaded_edits: list[EditData]


class GraphIoServiceBase(ServiceBase, GraphIoBase, abc.ABC):
    """Common base for global & Bench-local graph I/O operations."""

    def __init__(
        self,
        *,
        bench_id: UUID | None,
        node_types: bittuple[NodeType],
        logger: structlog.BoundLogger,
        tracer: trace.Tracer,
        oracle: Oracle,
    ):
        super().__init__(logger=logger, tracer=tracer, oracle=oracle)
        self.epoch: int = 0
        self.bench_id: UUID | None = bench_id
        self.scope = GraphScope(bench_id=bench_id)._to_data()
        self.node_types: bittuple[NodeType] = node_types
        self.tx_lock: asyncio.Lock = CriticalLock(
            name=f"{self.__class__.__name__}_{bench_id or ''}"
        )
        self.connector = ConnectionIndex(scope=self.scope, oracle=self.oracle)

    @abc.abstractmethod
    def get_engines(self) -> tuple[GraphEngine, ...]:
        """Gets the graph engines available to this subgraph."""
        ...

    def _validate_request(self, request: betterproto.Message) -> None:
        """Validate a request message for this service."""
        scope: GraphScopeData = getattr(request, "scope", EMPTY_SCOPE._to_data())
        if to_uuid(scope.bench_id) != self.bench_id:
            raise GRPCError(
                GRPCStatus.INVALID_ARGUMENT, f"scope mismatch: {scope.bench_id} != {self.bench_id}"
            )

    async def start(self):
        self.tasks.start_scheduled(
            30, self.connector.gc_connections, task_id="gc_connections", skip_errors=False
        )

    @property
    def request_session_parent(self) -> Package | None:
        return None

    @property
    def split_reads(self) -> bool:
        return False

    @final
    def request_session(
        self,
        supergraph: NodeSuperGraph,
        *,
        engines: tuple[GraphEngine, ...] | None = None,
        readonly: bool = True,
        system_commit: bool = True,
    ):
        """Gets a new session for processing a single request."""
        return Session(
            parent=self.request_session_parent,
            _is_readonly=readonly,
            _default_scope=self.scope,
            _engines=engines if engines is not None else self.get_engines(),
            _epoch=self.epoch,
            _custom_commit=self._commit_system_session if system_commit else None,
            _supergraph=supergraph,
            _split_read=self.split_reads,
            _oracle=self.oracle,
        )

    @override
    async def get_nodes(self, subject: Subject, request: "GetNodesRequest") -> "GetNodesResponse":
        # parse query & fetch
        async with self.request_session(supergraph=subject._supergraph) as session:
            with self.tracer.start_as_current_span("graph.get.parse"):
                roots = [
                    wiring.unpack_object_validate(r, supergraph=None, expect=NodeReference)
                    for r in request.roots
                ]
                if not roots:
                    raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no roots provided")
                if any(not r.id for r in roots):
                    raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "root nodes must have an id")
                options: ReadOptions = (
                    wiring.unpack_object_validate_maybe(
                        request.options, supergraph=None, expect=ReadOptions
                    )
                    or ReadOptions.default()
                )
                roots_by_type: dict[NodeType, list[NodeReference]] = group_by(
                    roots, lambda r: r.type
                )
                if len(roots_by_type) > 1:
                    raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "roots must be of the same type")
                node_type = next(iter(roots_by_type.keys()))

            with self.tracer.start_as_current_span("graph.get.read") as span:
                adapted_options = adapt_read_options(subject, node_type, options)
                query = QueryBuilder(
                    read_type=ReadType.GET,
                    node_type=wiring.unpack_enum(NodeType, node_type),
                    roots=roots,
                    options=adapted_options,
                )
                connection = await self.connector.connect(
                    query=query,
                    session=session,
                    connection_t=GetConnection,
                    cache=CONNECTION_CACHE_ENABLED and not request.no_cache,
                )
                result = connection.result
                span.set_attributes(
                    {"connection_hash": connection.hash, "connection_token": connection.token}
                )
        if any(cast(str, root.id) not in result.graph for root in request.roots):
            missing_roots = tuple(root for root in roots if str(root.id) not in result.graph)
            raise GRPCError(GRPCStatus.NOT_FOUND, f"roots not found: {missing_roots}")

        # check access & prune result
        with self.tracer.start_as_current_span("graph.get.check_access"):
            matrix = generate_access_matrix(subject, result.graph, supergraph=session._supergraph)
            decision, accesses, adapted_nodes = evaluate_and_adapt_read(
                matrix,
                result.graph,
                root_node_type=node_type,
                options=options,
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
            adapted=len(adapted_nodes),
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
        self, subject: Subject, request: WatchGetRequest
    ) -> AsyncIterator[WatchGetResponse]:
        subscription = await self.connector.subscribe(
            subject=subject,
            connection_t=GetConnection,
            update_t=WatchGetUpdate,
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
        self, subject: Subject, request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
        # parse query & fetch
        async with self.request_session(supergraph=subject._supergraph) as session:
            with self.tracer.start_as_current_span("graph.search.parse"):
                node_type: NodeType = wiring.unpack_enum(NodeType, request.node_type)
                filter = wiring.unpack_object_validate_maybe(
                    request.filter, supergraph=session._supergraph, expect=Expression
                )
                sort = [
                    wiring.unpack_object_validate(
                        s, supergraph=session._supergraph, expect=Expression
                    )
                    for s in request.sort
                ] or []
                options = (
                    wiring.unpack_object_validate_maybe(
                        request.options, supergraph=None, expect=ReadOptions
                    )
                    or ReadOptions.default()
                )
                adapted_options = adapt_read_options(subject, node_type, options)
                query = QueryBuilder(
                    read_type=ReadType.SEARCH,
                    node_type=node_type,
                    filter=filter,
                    options=adapted_options,
                    sort=sort,
                    first=request.first,
                    skip=request.skip,
                )

            with self.tracer.start_as_current_span("graph.search.read") as span:
                connection = await self.connector.connect(
                    query=query,
                    session=session,
                    connection_t=SearchConnection,
                    cache=CONNECTION_CACHE_ENABLED and not request.no_cache,
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
                options=options,
                required_nodes=request.bases,
            )
            if decision != PolicyEffect.ALLOW:
                raise AccessError(accesses)

        self.logger.info(
            "graph.search",
            subject=subject,
            query=query,
            connection=connection,
            graph=result.graph,
            adapted=len(adapted_nodes),
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
        self, subject: Subject, request: WatchSearchRequest
    ) -> AsyncIterator[WatchSearchResponse]:
        subscription = await self.connector.subscribe(
            subject=subject,
            connection_t=SearchConnection,
            update_t=WatchSearchUpdate,
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
        self, subject: Subject, request: "AggregateNodesRequest"
    ) -> "AggregateNodesResponse":
        # parse query & fetch
        async with self.request_session(supergraph=subject._supergraph) as session:
            with self.tracer.start_as_current_span("graph.aggregate.parse"):
                node_type: NodeType = wiring.unpack_enum(NodeType, request.node_type)
                filter = wiring.unpack_object_validate_maybe(
                    request.filter, supergraph=session._supergraph, expect=Expression
                )
                aggregation = wiring.unpack_object_validate(
                    request.aggregation, supergraph=session._supergraph, expect=Expression
                )
                adapted_options = adapt_read_options(subject, node_type, ReadOptions())
                query = QueryBuilder(
                    read_type=ReadType.AGGREGATE,
                    node_type=node_type,
                    filter=filter,
                    options=adapted_options,
                    aggregation=aggregation,
                )

            with self.tracer.start_as_current_span("graph.aggregate.read") as span:
                connection = await self.connector.connect(
                    query=query,
                    session=session,
                    connection_t=AggregateConnection,
                    cache=CONNECTION_CACHE_ENABLED and not request.no_cache,
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
        self, subject: Subject, request: WatchAggregateRequest
    ) -> AsyncIterator[WatchAggregateResponse]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED, "watch_aggregate not yet supported")
        yield  # unreachable (for type)

    def _prepare_commit(
        self, subject: Subject, context: SessionContext, edits: list[EditData]
    ) -> tuple["CommitArea", int]:
        """Prepares and validates the edits for a commit."""
        area = parse_commit_area(edits, base_graph=None)
        now = self.oracle.utc()
        epoch = self.epoch
        for edit in edits:
            validate_edit(edit, subject, now)
            epoch += 1
            edit.epoch = epoch
        return area, epoch

    async def _do_commit(
        self,
        *,
        scope: GraphScopeData,
        subject: Subject,
        context: SessionContext,
        edits: list[EditData],
    ) -> DoCommitRet:
        # pre-validate/prepare edits
        area, epoch = self._prepare_commit(subject, context, edits)

        async with self.request_session(
            supergraph=subject._supergraph, readonly=False, system_commit=False
        ) as session:
            # read the affected nodes into a single graph for evaluation
            data_graph = NodeDataGraph(scope=self.scope, node_types=NODE_TYPES)
            with self.tracer.start_as_current_span("graph.commit.read"):
                for node_type, node_references in area.scopes_by_type.items():
                    node_type = wiring.unpack_enum(NodeType, node_type)
                    options = adapt_read_options(subject, node_type, ReadOptions.all())
                    query = QueryBuilder(
                        read_type=ReadType.GET,
                        node_type=node_type,
                        roots=node_references,
                        options=options,
                    )
                    channel = await session._get_channel_for(
                        scope, query.all_node_types, is_readonly=True
                    )
                    connection = await channel.get(query, GetOptions(live=False, unpack=False))
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

            # apply edits in copy (to validate and get current 'old' values)
            # TODO :Robustness!: prevent circular parent/child references
            edit_data_graph(
                graph=data_graph,
                edits=edits,
                options=ReadOptions.all(),
                is_prepass=True,
            )
            unpacked_graph = wiring.unpack_node_graph(
                data_graph, supergraph=subject._supergraph, parent=None, session=session
            )

            for node_id in area.edited_node_ids:
                node = unpacked_graph.get(UUID(node_id))
                if node is None:
                    raise GRPCError(GRPCStatus.NOT_FOUND, f"{node_id} not found")
                node._validate_self(properties=(), invalid=on_invalid_raise)

            # flush edits to get cascaded edits for extend
            assert len(session.tx.edits) == 0, f"unexpected edits in {session.tx!r}"
            session.tx._add_pending_edits(edits)
            _, cascaded_edits = await session.flush()

            # extend commit
            new_edits = await self.extend_commit(
                supergraph=session._supergraph,
                session=session,
                context=context,
                graph=unpacked_graph,
                edits=edits,
                cascaded_edits=cascaded_edits,
            )

            # actually commit (with new edits)
            edits, cascaded_edits = await session.commit()
        session.untrack_many(*unpacked_graph.nodes)

        # handle on commit
        self.epoch = epoch
        await self.on_commit(
            supergraph=session._supergraph,
            graph=unpacked_graph,
            data_graph=data_graph,
            edits=edits,
            cascaded_edits=cascaded_edits,
            epoch=self.epoch,
        )

        return DoCommitRet(
            epoch=epoch, edits=edits, new_edits=new_edits, cascaded_edits=cascaded_edits
        )

    @override
    async def commit_transaction(
        self, subject: Subject, request: "CommitTransactionRequest"
    ) -> "CommitTransactionResponse":
        assert subject.client is not None, f"no client for {subject!r}"

        # figure out context
        context = wiring.unpack_object_validate_maybe(
            request.context, supergraph=subject._supergraph, expect=SessionContext
        )
        if context is None:
            context = SessionContext(
                client=subject.client,
                server=subject.server,
                user=subject.user,
                _supergraph=subject._supergraph,
            )

        # NOTE :Performance: obviously, putting a big lock around commit is not ideal,
        #  but we have to guarantee absolute order + integrity of any loaded graphs (in Host).
        # We can probably optimize this by only locking some tighter critical sections
        #  if we rollback somehow on failure. Maybe we can even 'cache' apply some edits only in memory.
        # Later, we'll get to figure out how to thread and eventually shard the Host :)
        async with self.tx_lock:
            retry = COMMIT_RETRY.new(self.oracle)
            while retry.should_retry:
                retry.on_attempt()
                try:
                    commit = await self._do_commit(
                        scope=self.scope, subject=subject, context=context, edits=request.edits
                    )
                    break
                except Exception as e:
                    # this is most likely a temporary error (i.e. channel unavailable)
                    self.logger.warning("graph.commit.error", subject=subject, exc_info=e)
                    if not retry.on_error(e):
                        raise
                    await self.oracle.sleep(retry.get_wait_interval())
            else:
                raise retry.to_error("commit")

        self.logger.info(
            "graph.commit",
            subject=subject,
            request=request,
            request_edits=len(request.edits),
            epoch=self.epoch,
            span="current",
        )
        accepted_revisions = [cast(int, e.revision) for e in request.edits]
        return CommitTransactionResponse(
            revisions=accepted_revisions, cascaded_edits=commit.cascaded_edits, epoch=self.epoch
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
        # NOTE :Performance: we could be a smarter to avoid packing edited nodes here
        #  (but it doesn't really matter since the number of nodes here is usually small)
        edit_data_graph = NodeDataDict(
            {str(node.id): node._to_data() for node in session._edited_nodes_by_id.values()}
        )

        try:
            # flush edits to get cascaded edits
            edits, cascaded_edits = await session._tx.flush()

            # extend commit
            await self.extend_commit(
                supergraph=session._supergraph,
                session=session,
                context=None,
                graph=edit_graph,
                edits=edits,
                cascaded_edits=cascaded_edits,
            )

            # commit
            edits, cascaded_edits = await session._tx.commit()
        except ChannelUnavailableError as e:
            self.logger.error("graph.commit.error", session=session, error=e)
            await session._tx.reset()
            raise

        # handle on commit
        self.epoch = session.epoch
        await self.on_commit(
            supergraph=session._supergraph,
            graph=edit_graph,
            data_graph=edit_data_graph,
            edits=edits,
            cascaded_edits=cascaded_edits,
            epoch=self.epoch,
        )
        return edits, cascaded_edits

    async def extend_commit(
        self,
        supergraph: NodeSuperGraph,
        session: Session,
        context: SessionContext | None,
        graph: NodeGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
    ) -> list[EditData]:
        """Extend a commit in a request session. Returns any new edits, but must add them to session."""
        return []  # do nothing by default

    async def on_commit(
        self,
        supergraph: NodeSuperGraph,
        graph: NodeGraphLike,
        data_graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        """Handle a commit in the request session."""
        self.connector.on_commit(data_graph, edits, cascaded_edits, epoch)


class CommitArea(NamedTuple):
    """The scope of relevant nodes for a transaction."""

    edited_node_ids: set[str]
    scopes_by_type: dict[NodeType, list[NodeReference]]
    graph_scopes: tuple[GraphScopeData, ...]


def parse_commit_area(edits: list[EditData], base_graph: NodeDataGraph | None) -> CommitArea:
    """
    Gets the specific nodes (scopes) and related nodes that are edited. :NodeEditScope
    """
    from bench.proto import wiring

    edited_node_ids: set[str] = set()
    node_scopes_by_id: dict[str, NodeReferenceData] = {}
    graph_scopes: dict[int, GraphScopeData] = {}
    in_tx_created_nodes_ids: set[str] = set()
    for edit in edits:
        node_type = NodeType(edit.node_ptr.type)
        assert edit.node_ptr.id, f"missing id for {edit!r}"
        node_id = edit.node_ptr.id
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
                assert (
                    new_node.parent_ptr is not None
                ), f"missing parent for {new_node!r} in {edit!r}"
                assert new_node.parent_ptr.id, f"missing parent id for {new_node!r} in {edit!r}"
                node_scopes_by_id[new_node.parent_ptr.id] = new_node.parent_ptr
        if node_type in BASED_NODE_TYPES and edit.node_ptr.base_ck is not None:
            # also add base as node scope
            assert base_graph is not None, f"missing base graph for {edit!r}"
            node_cls = cast(type[HasNodeBase], NODE_CLASS_BY_TYPE[node_type])
            # add current base (base is immutable)
            old_base_node = base_graph.get(edit.node_ptr.base_ck)
            if old_base_node is not None:
                old_base_ptr = NodeReference.from_node_data(old_base_node)
                assert old_base_ptr.id, f"missing base id for {old_base_ptr!r} in {edit!r}"
                node_scopes_by_id[old_base_ptr.id] = old_base_ptr
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
    node_scopes_by_type = group_by(node_scopes.values(), lambda n: n.type)
    return CommitArea(
        edited_node_ids=edited_node_ids,
        scopes_by_type=node_scopes_by_type,
        graph_scopes=tuple(graph_scopes.values()),
    )


def _is_allowable_drift(dt: datetime, now: datetime) -> bool:
    """Check if the given datetime is within the allowed time drift."""
    return abs((now - dt).total_seconds()) <= MAX_TIME_DRIFT_SECONDS


def validate_edit(edit: EditData, subject: Subject, now: datetime) -> None:
    """Checks the given (non-system) edit for basic validity."""
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
            f"bad edited_at in {edit!r}: {edit.edited_at} !~= {now}",
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
