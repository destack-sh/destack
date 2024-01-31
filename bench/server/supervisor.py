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
    check_access_pre_read,
    SYSTEM_POLICIES,
    Action,
    RequestObject,
)
from bench.language.const import NodeType, ReadType
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
)
from bench.server.auth import check_password, generate_access_token, generate_salt, hash_password
from bench.server.utils import detached_session, validate_bench_data_many
from bench.sql.client import async_pg_cursor
from bench.sql.engine import pg_read_node_data_tree, pg_search_nodes_data_tree
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

    async def signup_user(self, signup_user_request: "SignupUserRequest") -> "SignupUserResponse":
        if self.subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        validate_bench_data_many(signup_user_request.user, signup_user_request.client)
        async with detached_session() as session:
            user: User = wiring.unpack_node(signup_user_request.user, parent=None, session=session)
            user.password_salt = generate_salt()
            user.password_hash = hash_password(signup_user_request.password, user.password_salt)
            user.handle = Handle(slug=user.slug)
            client: Client = wiring.unpack_node(
                signup_user_request.client, parent=user, session=session
            )
            client.token = generate_access_token()
            session.create_many(user.handle, user, client)
            await session.commit()
        return SignupUserResponse(user=user._to_data(), access_token=client.token)

    async def login_user(self, login_user_request: "LoginUserRequest") -> "LoginUserResponse":
        if self.subject.is_authenticated:
            raise GRPCError(GRPCStatus.ALREADY_EXISTS, "already logged in")

        async with detached_session() as session:
            key_name, key_value = betterproto.which_one_of(login_user_request, "user")
            if key_value is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no user provided")
            user = await User.get(cast(Expression, User.__properties__[key_name] == key_value))
            if not await check_password(
                login_user_request.password, user.password_salt, user.password_hash
            ):
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "incorrect password")

            client: Client = wiring.unpack_node(
                login_user_request.client, parent=user, session=session
            )
            client.logged_in_at = client.last_seen_at = utcnow_with_tz()
            client.access_token = generate_access_token()
            session.upsert(client)
            await session.commit()
        return LoginUserResponse(
            user=user._to_data(), client=client._to_data(), access_token=client.access_token
        )

    async def logout_user(self, logout_user_request: "LogoutUserRequest") -> "LogoutUserResponse":
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

    async def create_bench(
        self, create_bench_request: "CreateBenchRequest"
    ) -> "CreateBenchResponse":
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

    async def read_nodes(self, read_nodes_request: "ReadNodesRequest") -> "ReadNodesResponse":
        roots: tuple[NodeReference, ...] = tuple(
            wiring.unpack_struct(r) for r in read_nodes_request.roots
        )
        options: ReadOptions | None = wiring.unpack_struct_maybe(read_nodes_request.options)
        options = options or ReadOptions.default()
        get_root_requests = tuple(
            Request(
                self.subject,
                ReadType.GET,
                RequestObject(type=root.type, is_sensitive=options.include_sensitive),
            )
            for root in roots
        )
        action = Action(
            self.subject, (*Request.from_read_options(options, self.subject), *get_root_requests)
        )
        options = check_access_pre_read(action, SYSTEM_POLICIES, options)

        roots_by_type: dict[NodeType, list[NodeReference]] = group_by(roots, lambda r: r.type)
        tree = NodeDataTree()
        async with async_pg_cursor() as cur:
            for root_node_type, root_node_references in roots_by_type.items():
                node_type = wiring.unpack_enum(NodeType, root_node_type)
                _ = await pg_read_node_data_tree(
                    cur=cur,
                    root_type=node_type,
                    root_ids=tuple(r.id for r in root_node_references),
                    options=options,
                    _tree=tree,  # accumulate into tree
                )

        return ReadNodesResponse(nodes=[wiring.wrap_some_node(n) for n in tree.nodes])

    async def search_nodes(
        self, search_nodes_request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
        if search_nodes_request.bases:
            raise grpclib.GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO has no bases")
        options: ReadOptions | None = wiring.unpack_struct_maybe(search_nodes_request.options)
        options = options or ReadOptions.default()
        list_request = Request(
            self.subject,
            ReadType.LIST,
            RequestObject(type=search_nodes_request.type, is_sensitive=options.include_sensitive),
        )
        action = Action(
            self.subject, (*Request.from_read_options(options, self.subject), list_request)
        )
        options = check_access_pre_read(action, SYSTEM_POLICIES, options)

        async with async_pg_cursor() as cur:
            _ = await pg_search_nodes_data_tree()

        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def aggregate_nodes(
        self, aggregate_nodes_request: "AggregateNodesRequest"
    ) -> "AggregateNodesResponse":
        if aggregate_nodes_request.bases:
            raise grpclib.GRPCError(GRPCStatus.INVALID_ARGUMENT, "global IO has no bases")
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def commit_edits(
        self, commit_edits_request: "CommitEditsRequest"
    ) -> "CommitEditsResponse":
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def watch_edits(
        self, watch_edits_request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # Worker stuff
    #

    async def restart_worker_set(
        self, restart_worker_set_request: "RestartWorkerSetRequest"
    ) -> "PingWorkerSetResponse":
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def ping_worker_set(
        self, ping_worker_set_request: "PingWorkerSetRequest"
    ) -> "PingWorkerSetResponse":
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)
