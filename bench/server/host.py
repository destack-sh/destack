from typing import AsyncIterator
from uuid import UUID

import grpclib
import structlog

from bench.proto.discovery import ExtendedServiceBase
from bench.proto.wire import (
    ModuleHostBase,
    ReadNodesRequest,
    ReadNodesResponse,
    SearchNodesRequest,
    SearchNodesResponse,
    CommitEditsRequest,
    CommitEditsResponse,
    PushEditsRequest,
    WatchEditsRequest,
    WatchEditsResponse,
    PasteNodesRequest,
    PasteNodesResponse,
    SnapshotModuleRequest,
    SnapshotModuleResponse,
    UploadBlobRequest,
    UploadBlobResponse,
    DownloadBlobRequest,
    DownloadBlobResponse,
    SearchLogsRequest,
    SearchLogsResponse,
    WatchLogsRequest,
    WatchLogsResponse,
    PushWorkerLogsRequest,
    StartRunRequest,
    StartRunResponse,
    KillRunRequest,
    KillRunResponse,
    RunProxyStatementRequest,
    RunProxyStatementResponse,
    PushEditsResponse,
)
from bench.utils.monitoring import Monitored

logger = structlog.get_logger(__name__)


class ModuleHostMultiplexer(ExtendedServiceBase, ModuleHostBase):
    """
    Multiplexes requests per module to the appropriate ModuleHost using gRPC metadata.
    'bench-id' and 'module-id' are required. Hosts are loaded for all active modules on startup,
    and new ones 'ping' the multiplexer to add themselves.
    """

    def __init__(self):
        self._hosts_by_module_id: dict[UUID, ModuleHost] = {}

    async def start_quick(self) -> None:
        raise NotImplementedError("nocheckin: ModuleHostMultiplexer.start_quick")

    # nocheckin: "proxy" ModuleHost / start and connect relevant Bench module hosts


class ModuleHost(Monitored, ModuleHostBase):
    """
    Host for an (active) Bench module. Manages basically everything that's not actually running it.
    Frontend and worker connects to this to do anything with the module.
    """

    def __init__(self, bench_id: UUID, module_id: UUID):
        self.bench_id = bench_id
        self.module_id = module_id

    async def start_quick(self) -> None:
        raise NotImplementedError("nocheckin: ModuleHost.start_quick")

    #
    # Module IO
    #

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

    async def push_edits(self, push_edits_request: "PushEditsRequest") -> "PushEditsResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def watch_edits(
        self, watch_edits_request: "WatchEditsRequest"
    ) -> AsyncIterator["WatchEditsResponse"]:
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def paste_nodes(self, paste_nodes_request: "PasteNodesRequest") -> "PasteNodesResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def snapshot(
        self, snapshot_module_request: "SnapshotModuleRequest"
    ) -> "SnapshotModuleResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    #
    # Blobs
    #

    async def upload_blob(self, upload_blob_request: "UploadBlobRequest") -> "UploadBlobResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def download_blob(
        self, download_blob_request: "DownloadBlobRequest"
    ) -> "DownloadBlobResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    #
    # Logs
    #

    async def search_logs(self, search_logs_request: "SearchLogsRequest") -> "SearchLogsResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def watch_logs(
        self, watch_logs_request: "WatchLogsRequest"
    ) -> AsyncIterator["WatchLogsResponse"]:
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def push_worker_logs(
        self, push_worker_logs_request: "PushWorkerLogsRequest"
    ) -> "PushWorkerLogsRequest":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    #
    # Runs
    #

    async def start_run(self, start_run_request: "StartRunRequest") -> "StartRunResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def kill_run(self, kill_run_request: "KillRunRequest") -> "KillRunResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)

    async def run_proxy_statement(
        self, run_proxy_statement_request: "RunProxyStatementRequest"
    ) -> "RunProxyStatementResponse":
        raise grpclib.GRPCError(grpclib.const.Status.UNIMPLEMENTED)
