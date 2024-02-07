from typing import AsyncIterator

import betterproto
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Client, Expression, Handle, NodeReference, User
from bench.language.access import (
    ReadOptions,
    adapt_read_options,
    evaluate_and_adapt_read,
    generate_access_matrix,
    Subject,
)
from bench.language.const import NodeType, ABOVE_SOURCE_NODE_TYPES, AggregationOp
from bench.language.expression import Aggregation
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
    SupervisorBase,
    SupervisorStub,
    LoginUserRequest,
    LoginUserResponse,
    LogoutUserRequest,
    LogoutUserResponse,
    PingServerRequest,
    PingServerResponse,
    ReadNodesRequest,
    ReadNodesResponse,
    RestartServerRequest,
    SearchNodesRequest,
    SearchNodesResponse,
    SignupUserRequest,
    SignupUserResponse,
    WatchEditsRequest,
    WatchEditsResponse,
    ChangeUserPasswordRequest,
    ChangeUserPasswordResponse,
)
from bench.system.auth import check_password, generate_access_token, generate_salt, hash_password
from bench.system.utils import detached_session
from bench.sql.client import async_pg_cursor
from bench.sql.engine import (
    pg_count,
    pg_read_node_data_tree,
    pg_search_nodes_data_tree,
    compile_pg_conditional,
    pg_exists,
)
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import group_by, to_uuid

logger = structlog.get_logger("global_supervisor")

SERVER_IDLE_SLEEP_TIME = 30 * 60  # 30 minutes
SERVER_GENTLE_RESTART_TIMEOUT = 5  # 5 seconds until force restart


class Supervisor(BenchServiceBase[SupervisorStub], SupervisorBase):
    def __init__(self):
        super().__init__(loopback_stub_to=SupervisorStub)

    def __str__(self):
        return "shards=*"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start_quick(self) -> None:
        pass

    def close(self) -> None:
        pass

    async def wait_closed(self) -> None:
        pass

    #
    # Users
    #

    async def signup_user(
        self, subject: Subject, request: "SignupUserRequest"
    ) -> "SignupUserResponse":
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with detached_session() as session:
            user = User(
                id=to_uuid(request.id),
                slug=request.slug,
                name=request.name,
                email=request.email,
                is_activated=True,
                _is_new=True,  # create, despite already having an id in init
            )
            user.password_salt = generate_salt()
            user.password_hash = hash_password(request.password, user.password_salt)
            user.handle = Handle(slug=user.slug)
            client: Client = wiring.unpack_node(request.client, parent=user, session=session)
            client.access_token = generate_access_token()
            session.create_many(user.handle, user, client)
            await session.commit()
        return SignupUserResponse(user=user._to_data(), access_token=client.access_token)

    async def change_user_password(
        self, subject: Subject, request: "ChangeUserPasswordRequest"
    ) -> "ChangeUserPasswordResponse":
        if not subject.is_authenticated:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        async with detached_session() as session:
            user = subject.user
            if not await check_password(
                request.old_password, user.password_salt, user.password_hash
            ):
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "incorrect password")

            # set new password
            user.password_salt = generate_salt()
            user.password_hash = hash_password(request.password, user.password_salt)
            session.track(user)
            await session.commit()
        return ChangeUserPasswordResponse(user=user._to_data())

    async def login_user(
        self, subject: Subject, request: "LoginUserRequest"
    ) -> "LoginUserResponse":
        if subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with detached_session() as session:
            key_name, key_value = betterproto.which_one_of(request, "user")
            if key_value is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no user provided")
            user = await User.include(User.password_salt, User.password_hash).get(
                User.__properties__[key_name] == key_value
            )
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

    async def logout_user(
        self, subject: Subject, request: "LogoutUserRequest"
    ) -> "LogoutUserResponse":
        if subject.client is None:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        async with detached_session() as session:
            # log out the current or the specified clients
            if request.client_ids:
                clients = await Client.filter(
                    parent=subject.user, id__in=request.client_ids
                ).tolist()
            elif request.logout_all:
                clients = await Client.filter(parent=subject.user).tolist()
            else:
                clients = (subject.client,)
                session.track(subject.client)
            for client in clients:
                client.logged_in_at = None
                client.access_token = None
                client.last_seen_at = utcnow_with_tz()
            await session.commit()
        return LogoutUserResponse()

    #
    # Bench management
    #

    async def create_bench(
        self, subject: Subject, request: "CreateBenchRequest"
    ) -> "CreateBenchResponse":
        user: User | None = subject.user
        if not user:
            raise GRPCError(GRPCStatus.UNAUTHENTICATED, "not logged in")
        async with detached_session() as session:
            session.track(user)
            if user.bench:  # can't create secondary benches yet
                raise GRPCError(GRPCStatus.ALREADY_EXISTS, "bench already exists")
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # General Bench IO for > package & global nodes only :BenchIO
    #

    async def read_nodes(
        self, subject: Subject, request: "ReadNodesRequest"
    ) -> "ReadNodesResponse":
        roots: tuple[NodeReference, ...] = tuple(wiring.unpack_struct(r) for r in request.roots)
        if not roots:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no roots provided")
        if any(root.type not in ABOVE_SOURCE_NODE_TYPES for root in roots):
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO can't read packages")
        options: ReadOptions = (
            wiring.unpack_struct_interp_maybe(request.options) or ReadOptions.default()
        )

        roots_by_type: dict[NodeType, list[NodeReference]] = group_by(roots, lambda r: r.type)
        tree = NodeDataTree()
        async with async_pg_cursor() as cur:
            for root_node_type, root_node_references in roots_by_type.items():
                adapted_options = adapt_read_options(subject, root_node_type, options)
                node_type = wiring.unpack_enum(NodeType, root_node_type)
                _ = await pg_read_node_data_tree(
                    cur=cur,
                    root_type=node_type,
                    root_ids=tuple(r.id for r in root_node_references),
                    options=adapted_options,
                    _tree=tree,  # accumulate into tree
                )
        # check all the roots were found
        if any(root.id not in tree for root in request.roots):
            missing_roots = tuple(root for root in roots if str(root.id) not in tree)
            raise GRPCError(GRPCStatus.NOT_FOUND, f"roots not found: {missing_roots}")

        access = generate_access_matrix(subject, tree)
        action, adapted_nodes = evaluate_and_adapt_read(
            access, tree, required_nodes=request.roots, adapt_nodes_in_place=True
        )

        return ReadNodesResponse(
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes], access=access._to_data()
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

        async with async_pg_cursor() as cur:
            combined_filter = adapted_options.filter(node_type, filter)
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
                count = await pg_count(
                    cur=cur,
                    table=node_cls.__table__,
                    where=compile_pg_conditional(node_cls, combined_filter),
                )
            else:
                count = None
        access = generate_access_matrix(subject, tree)
        action, adapted_nodes = evaluate_and_adapt_read(
            access, tree, adapt_nodes_in_place=True, required_nodes=request.bases
        )
        await self.log_action(action)

        return SearchNodesResponse(
            roots=[NodeReference.from_node_data(r) for r in roots.nodes],
            nodes=[wiring.wrap_some_node(n) for n in adapted_nodes],
            cursors=list(roots.cursors),
            start_cursor=roots.start_cursor,
            total=count,
            access=access._to_data(),
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

        async with async_pg_cursor() as cur:
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

        return AggregateNodesResponse(aggregation=result._to_data())

    async def commit_edits(
        self, subject: Subject, request: "CommitEditsRequest"
    ) -> "CommitEditsResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def watch_edits(
        self, subject: Subject, request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # Server stuff
    #

    async def restart_server(
        self, subject: Subject, request: "RestartServerRequest"
    ) -> "PingServerResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def ping_server(
        self, subject: Subject, request: "PingServerRequest"
    ) -> "PingServerResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
