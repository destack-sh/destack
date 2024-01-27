from typing import AsyncIterator, cast

import betterproto
import grpclib
import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus

from bench.language import Expression, Handle, User
from bench.language.const import NodeType
from bench.language.node import NODE_CLASS_BY_TYPE
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
from bench.server.utils import (
    check_authenticated_client,
    detached_session,
    validate_bench_data_many,
)

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
        validate_bench_data_many(signup_user_request.user, signup_user_request.client)
        async with detached_session() as session:
            user = wiring.unpack_node(signup_user_request.user, parent=None, session=session)
            user.password_salt = generate_salt()
            user.password_hash = hash_password(signup_user_request.password, user.password_salt)
            user.handle = Handle(slug=user.slug)
            client = wiring.unpack_node(signup_user_request.client, parent=user, session=session)
            client.token = generate_access_token()
            session.create_many(user.handle, user, client)
            await session.commit()
        return SignupUserResponse(user=user._to_data(), access_token=client.token)

    async def login_user(self, login_user_request: "LoginUserRequest") -> "LoginUserResponse":
        async with detached_session() as session:
            key_name, key_value = betterproto.which_one_of(login_user_request, "user")
            if key_value is None:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "no user provided")
            user = await User.get(cast(Expression, User.__properties__[key_name] == key_value))
            if not await check_password(
                login_user_request.password, user.password_salt, user.password_hash
            ):
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "incorrect password")

            client = wiring.unpack_node(login_user_request.client, parent=user, session=session)
            client.token = generate_access_token()
            session.upsert(client)
            await session.commit()
        return LoginUserResponse(
            user=user._to_data(), client=client._to_data(), access_token=client.token
        )

    async def logout_user(self, logout_user_request: "LogoutUserRequest") -> "LogoutUserResponse":
        async with detached_session(commit=True):
            client = await check_authenticated_client(self.metadata)
            client.access_token = None
        return LogoutUserResponse()

    async def create_bench(
        self, create_bench_request: "CreateBenchRequest"
    ) -> "CreateBenchResponse":
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    #
    # General Bench IO (for global nodes only) :BenchIO
    #

    async def read_nodes(self, read_nodes_request: "ReadNodesRequest") -> "ReadNodesResponse":
        for root in read_nodes_request.roots:
            node_type = wiring.unpack_enum(NodeType, root.type)
            node_cls = NODE_CLASS_BY_TYPE[node_type]
            if node_cls.__is_in_package__:
                raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "can't read package in global scope")

        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def search_nodes(
        self, search_nodes_request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
        raise grpclib.GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def aggregate_nodes(
        self, aggregate_nodes_request: "AggregateNodesRequest"
    ) -> "AggregateNodesResponse":
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
