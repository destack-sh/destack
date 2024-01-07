from typing import AsyncIterator

import structlog

from bench.proto.wire import (
    GlobalSupervisorBase,
    CreateUserRequest,
    CreateUserResponse,
    LoginUserRequest,
    LoginUserResponse,
    LogoutUserRequest,
    CreateBenchRequest,
    CreateBenchResponse,
    ReadNodesRequest,
    ReadNodesResponse,
    SearchNodesRequest,
    SearchNodesResponse,
    CommitEditsRequest,
    CommitEditsResponse,
    WatchEditsRequest,
    WatchEditsResponse,
    RestartWorkerSetRequest,
    PingWorkerSetResponse,
    PingWorkerSetRequest,
    LogoutUserResponse,
)
from bench.utils.monitoring import Monitored

logger = structlog.get_logger(__name__)

WORKER_SET_IDLE_SLEEP_TIME = 30 * 60  # 30 minutes
WORKER_SET_GENTLE_RESTART_TIMEOUT = 5  # 5 seconds until force restart


class GlobalSupervisor(Monitored, GlobalSupervisorBase):
    @property
    def ready(self):
        return True

    @property
    def healthy(self):
        return True

    async def create_user(self, create_user_request: "CreateUserRequest") -> "CreateUserResponse":
        return await super().create_user(create_user_request)

    async def login_user(self, login_user_request: "LoginUserRequest") -> "LoginUserResponse":
        return await super().login_user(login_user_request)

    async def logout_user(self, logout_user_request: "LogoutUserRequest") -> "LogoutUserResponse":
        return await super().logout_user(logout_user_request)

    async def create_bench(
        self, create_bench_request: "CreateBenchRequest"
    ) -> "CreateBenchResponse":
        return await super().create_bench(create_bench_request)

    async def read_nodes(self, read_nodes_request: "ReadNodesRequest") -> "ReadNodesResponse":
        return await super().read_nodes(read_nodes_request)

    async def search_nodes(
        self, search_nodes_request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
        return await super().search_nodes(search_nodes_request)

    async def commit_edits(
        self, commit_edits_request: "CommitEditsRequest"
    ) -> "CommitEditsResponse":
        return await super().commit_edits(commit_edits_request)

    async def watch_edits(
        self, watch_edits_request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        return await super().watch_edits(watch_edits_request)

    async def restart_worker_set(
        self, restart_worker_set_request: "RestartWorkerSetRequest"
    ) -> "PingWorkerSetResponse":
        return await super().restart_worker_set(restart_worker_set_request)

    async def ping_worker_set(
        self, ping_worker_set_request: "PingWorkerSetRequest"
    ) -> "PingWorkerSetResponse":
        return await super().ping_worker_set(ping_worker_set_request)
