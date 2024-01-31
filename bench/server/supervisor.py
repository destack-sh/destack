from typing import AsyncIterator, cast

import betterproto
import grpclib
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Expression, Handle, User, Client, NodeReference
from bench.language.access import (
    ReadOptions,
    Request,
    adapt_access_pre_read,
    SYSTEM_POLICIES,
    RequestObject,
    adapt_access_post_read,
)
from bench.language.const import NodeType, ReadType
from bench.language.node import NODE_CLASS_BY_TYPE
from bench.language.tree import NodeDataTree
from bench.proto import wiring
from bench.proto.services import BenchServiceBase
from bench.proto.wire import (
    AggregateNodesRequest,
    AggregateNodesResponse,
    CommitEditsRequest,
    CommitEditsResponse,
    CreateBenchRequest,
    CreateBenchResponse,
    GlobalSupervisorBase,
    GlobalSupervisorStub,
    LoginUserRequest,
    LoginUserResponse,
    LogoutUserRequest,
    LogoutUserResponse,
    PingWorkerSetRequest,
    PingWorkerSetResponse,
    ReadNodesRequest,
    ReadNodesResponse,
    RestartWorkerSetRequest,
    SearchNodesRequest,
    SearchNodesResponse,
    SignupUserRequest,
    SignupUserResponse,
    WatchEditsRequest,
    WatchEditsResponse,
    AggregationOp,
)
from bench.server.auth import check_password, generate_access_token, generate_salt, hash_password
from bench.server.utils import detached_session, validate_bench_data_many
from bench.sql.client import async_pg_cursor
from bench.sql.engine import pg_read_node_data_tree, pg_search_nodes_data_tree, pg_count
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import group_by

logger = structlog.get_logger("global_supervisor")

WORKER_SET_IDLE_SLEEP_TIME = 30 * 60  # 30 minutes
WORKER_SET_GENTLE_RESTART_TIMEOUT = 5  # 5 seconds until force restart


class GlobalSupervisor(BenchServiceBase[GlobalSupervisorStub], GlobalSupervisorBase):
    def __init__(self):
        super().__init__(loopback_stub_to=GlobalSupervisorStub)

    def __str__(self):
        return "shards=*"

    def __repr__(self):
        return f"<GlobalSupervisor {self}>"

    async def start_quick(self) -> None:
        pass

    def close(self) -> None:
        pass

    async def wait_closed(self) -> None:
        pass

    #
    # Users
    #

    async def signup_user(self, request: "SignupUserRequest") -> "SignupUserResponse":
        if self.subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        validate_bench_data_many(request.user, request.client)
        async with detached_session() as session:
            user: User = wiring.unpack_node(request.user, parent=None, session=session)
            user.password_salt = generate_salt()
            user.password_hash = hash_password(request.password, user.password_salt)
            user.handle = Handle(slug=user.slug)
            client: Client = wiring.unpack_node(request.client, parent=user, session=session)
            client.token = generate_access_token()
            session.create_many(user.handle, user, client)
            await session.commit()
        return SignupUserResponse(user=user._to_data(), access_token=client.token)

    async def login_user(self, request: "LoginUserRequest") -> "LoginUserResponse":
        if self.subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with detached_session() as session:
            key_name, key_value = betterproto.which_one_of(request, "user")
            if key_value is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no user provided")
            user = await User.get(cast(Expression, User.__properties__[key_name] == key_value))
            if not await check_password(request.password, user.password_salt, user.password_hash):
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "incorrect password")

            client: Client = wiring.unpack_node(request.client, parent=user, session=session)
            client.logged_in_at = client.last_seen_at = utcnow_with_tz()
            client.access_token = generate_access_token()
            session.upsert(client)
            await session.commit()
        return LoginUserResponse(
            user=user._to_data(), client=client._to_data(), access_token=client.access_token
        )

    async def logout_user(self, request: "LogoutUserRequest") -> "LogoutUserResponse":
        client: Client | None = self.subject.client
        if client is None:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        async with detached_session() as session:
            session.track(client)
            client.logged_in_at = None
            client.access_token = None
            client.last_seen_at = utcnow_with_tz()
            await session.commit()
        return LogoutUserResponse()

    #
    # Bench management
    #

    async def create_bench(self, request: "CreateBenchRequest") -> "CreateBenchResponse":
        user: User | None = self.subject.user
        if not user:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        async with detached_session() as session:
            session.track(user)
            if user.bench:  # can't create secondary benches yet
                raise GRPCError(GRPCStatus.ALREADY_EXISTS, "bench already exists")
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # General Bench IO for > package & global nodes only :BenchIO
    #

    async def read_nodes(self, request: "ReadNodesRequest") -> "ReadNodesResponse":
        roots: tuple[NodeReference, ...] = tuple(wiring.unpack_struct(r) for r in request.roots)
        base_requests = tuple(
            Request(self.subject, ReadType.GET, RequestObject(type=root.type)) for root in roots
        )
        options: ReadOptions = wiring.unpack_struct_maybe(request.options) or ReadOptions.default()
        adapted_options = adapt_access_pre_read(base_requests, SYSTEM_POLICIES, options)

        roots_by_type: dict[NodeType, list[NodeReference]] = group_by(roots, lambda r: r.type)
        tree = NodeDataTree()
        async with async_pg_cursor() as cur:
            for root_node_type, root_node_references in roots_by_type.items():
                node_type = wiring.unpack_enum(NodeType, root_node_type)
                _ = await pg_read_node_data_tree(
                    cur=cur,
                    root_type=node_type,
                    root_ids=tuple(r.id for r in root_node_references),
                    options=adapted_options,
                    _tree=tree,  # accumulate into tree
                )
        tree = adapt_access_post_read(base_requests, SYSTEM_POLICIES, tree, options)

        return ReadNodesResponse(nodes=[wiring.wrap_some_node(n) for n in tree.nodes])

    async def search_nodes(self, request: "SearchNodesRequest") -> "SearchNodesResponse":
        if request.bases:
            raise grpclib.GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO has no bases")

        node_type: NodeType = wiring.unpack_enum(NodeType, request.type)
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        base_request = Request(self.subject, ReadType.LIST, RequestObject(type=node_type))
        filter: Expression | None = wiring.unpack_struct_maybe(request.filter)
        sort: list[Expression] = [wiring.unpack_struct(s) for s in request.sort] or None
        options: ReadOptions = wiring.unpack_struct_maybe(request.options) or ReadOptions.default()
        adapted_options = adapt_access_pre_read((base_request,), SYSTEM_POLICIES, options)

        async with async_pg_cursor() as cur:
            combined_filter = adapted_options.combined_filter(node_type, filter)
            roots, tree = await pg_search_nodes_data_tree(
                cur=cur,
                node_type=node_type,
                filter=combined_filter,
                sort=sort,
                first=request.limit,
                after=request.after,
                options=adapted_options,
            )
            if request.count:
                count = await pg_count(cur, node_cls.__table__, combined_filter)
            else:
                count = None
        tree = adapt_access_post_read(self.subject, (base_request,), SYSTEM_POLICIES, tree, options)

        return SearchNodesResponse(
            roots_ids=tuple(r.id for r in roots.nodes),
            nodes=tuple(wiring.wrap_some_node(n) for n in tree.nodes),
            cursors=roots.cursors,
            start_cursor=roots.start_cursor,
            total=count,
        )

    async def aggregate_nodes(self, request: "AggregateNodesRequest") -> "AggregateNodesResponse":
        if request.bases:
            raise grpclib.GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO has no bases")

        node_type: NodeType = wiring.unpack_enum(NodeType, request.type)
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        filter: Expression | None = wiring.unpack_struct_maybe(request.filter)
        sort: list[Expression] = [wiring.unpack_struct(s) for s in request.sort] or None
        aggregation: Expression = wiring.unpack_struct(request.aggregation)
        if aggregation.op in (AggregationOp.EXISTS, AggregationOp.COUNT):
            base_request = Request(self.subject, ReadType.LIST, RequestObject(type=node_type))
        else:
            raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def commit_edits(self, request: "CommitEditsRequest") -> "CommitEditsResponse":
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def watch_edits(
        self, request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # Worker stuff
    #

    async def restart_worker_set(
        self, request: "RestartWorkerSetRequest"
    ) -> "PingWorkerSetResponse":
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def ping_worker_set(self, request: "PingWorkerSetRequest") -> "PingWorkerSetResponse":
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)
