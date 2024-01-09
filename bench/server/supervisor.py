from typing import AsyncIterator

import grpclib
import structlog

from bench.language import Handle, User
from bench.language.auth import check_password, generate_access_token, generate_salt, hash_password
from bench.proto import wiring
from bench.proto.mesh import BenchServiceBase
from bench.proto.wire import (
    CommitEditsRequest,
    CommitEditsResponse,
    CreateBenchRequest,
    CreateBenchResponse,
    CreateUserRequest,
    CreateUserResponse,
    GlobalSupervisorBase,
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
    WatchEditsRequest,
    WatchEditsResponse,
    RpcMetadata,
)
from bench.server.utils import global_session

logger = structlog.get_logger(__name__)

WORKER_SET_IDLE_SLEEP_TIME = 30 * 60  # 30 minutes
WORKER_SET_GENTLE_RESTART_TIMEOUT = 5  # 5 seconds until force restart


class GlobalSupervisor(BenchServiceBase, GlobalSupervisorBase):
    def __str__(self):
        return "0"

    def __repr__(self):
        return f"<GlobalSupervisor {self}>"

    async def start_quick(self) -> None:
        pass

    def close(self) -> None:
        pass

    async def wait_closed(self) -> None:
        pass

    async def create_user(self, create_user_request: "CreateUserRequest") -> "CreateUserResponse":
        async with global_session(commit=True) as session:
            user = wiring.unpack_node(create_user_request.user, parent=None, session=session)
            user.password_salt = generate_salt()
            user.password_hash = hash_password(create_user_request.password, user.password_salt)
            user.handle = Handle(slug=user.username)
            client = wiring.unpack_node(create_user_request.client, parent=user, session=session)
            client.token = generate_access_token()
            session.create_many(user, user.handle, client)
        return CreateUserResponse(user=wiring.pack_node(user), access_token=client.token)

    async def login_user(self, login_user_request: "LoginUserRequest") -> "LoginUserResponse":
        async with global_session(commit=True) as session:
            if login_user_request.user.username:
                user = await User.get(username=login_user_request.user.username)
            elif login_user_request.user.email:
                user = await User.get(email=login_user_request.user.email)
            else:
                raise ValueError("no username or email provided")
            if not check_password(
                login_user_request.password, user.password_salt, user.password_hash
            ):
                raise ValueError("invalid password")

            client = wiring.unpack_node(login_user_request.client, parent=user, session=session)
            client.token = generate_access_token()
            session.upsert(client)
        return LoginUserResponse(user=wiring.pack_node(user), access_token=client.token)

    async def logout_user(self, logout_user_request: "LogoutUserRequest") -> "LogoutUserResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def create_bench(
        self, create_bench_request: "CreateBenchRequest"
    ) -> "CreateBenchResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def read_nodes(self, read_nodes_request: "ReadNodesRequest") -> "ReadNodesResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def search_nodes(
        self, search_nodes_request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def commit_edits(
        self, commit_edits_request: "CommitEditsRequest"
    ) -> "CommitEditsResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def watch_edits(
        self, watch_edits_request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def restart_worker_set(
        self, restart_worker_set_request: "RestartWorkerSetRequest"
    ) -> "PingWorkerSetResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def ping_worker_set(
        self, ping_worker_set_request: "PingWorkerSetRequest"
    ) -> "PingWorkerSetResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)
