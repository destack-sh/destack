# increment when making backwards-incompatible changes to messages
import abc
import enum
import typing
from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum
from typing import Optional
from uuid import UUID

from bench.language.const import (
    ModuleReference,
    ProjectRegion,
    QueryEngine,
    SessionAccessLevel,
    TriggerType,
    WorkerProfile,
)
from bench.language.edit import EditData
from bench.language.model import ModelErrorType
from bench.language.wire import (
    BlobData,
    EnvironmentData,
    ExpressionData,
    LogEntryData,
    ModuleTreeData,
    NodeData,
    RecordData,
    RunData,
    SecretData,
    SessionData,
    WorkerSetData,
)
from bench.utils.func import try_to_uuid
from bench.utils.utils import required_field

REGISTERED_MESSAGE_PAYLOADS: dict["NMessageType", typing.Type] = {}


class Payload:
    @property
    def topic(self):
        return self.__class__.type.value  # type: ignore


_PayloadT = typing.TypeVar("_PayloadT", bound=Payload)


@typing.dataclass_transform()
def payload(message_type: "NMessageType") -> typing.Callable[[_PayloadT], _PayloadT]:
    def wrapper(cls: typing.Type):
        if message_type in REGISTERED_MESSAGE_PAYLOADS:
            raise RuntimeError(f"message type {message_type} already registered")
        cls = dataclass(cls, slots=True)  # type: ignore
        REGISTERED_MESSAGE_PAYLOADS[message_type] = cls
        cls.type = message_type
        return cls

    return wrapper


class NMessageType(StrEnum):
    """All messages types"""

    # for sync
    CLIENT_CHANGED = "client.changed"
    PROJECT_CHANGED = "project.changed"
    MODULE_CHANGED = "module.changed"
    SESSION_CHANGED = "session.changed"
    RUNS_CHANGED = "runs.changed"
    LOGS_CHANGED = "logs.changed"
    WORKERS_CHANGED = "workers.changed"

    # read/write via runtime
    READ_MODULE = "module.read"
    READ_MODULE_REP = "module.read.rep"
    WRITE_EDITS = "module.write"
    WRITE_EDITS_REP = "module.write.rep"
    PASTE_NODES = "module.paste_nodes"
    PASTE_NODES_REP = "module.paste_nodes.rep"
    SNAPSHOT_MODULE = "module.snapshot"
    SNAPSHOT_MODULE_REP = "module.snapshot.rep"
    WRITE_SESSION = "session.write"
    WRITE_SESSION_REP = "session.write.rep"
    SEARCH_RECORDS = "module.search.records"
    SEARCH_RECORDS_REP = "module.search.records.rep"
    DOWNLOAD_BLOB = "object.read"
    DOWNLOAD_BLOB_REP = "object.read.rep"
    UPLOAD_BLOB = "object.write"
    UPLOAD_BLOB_REP = "object.write.rep"
    MARK_UPLOADED_BLOB = "object.mark_uploaded"
    MARK_UPLOADED_BLOB_REP = "object.mark_uploaded.rep"
    REVEAL_SECRET = "secret.read"
    REVEAL_SECRET_REP = "secret.read.rep"
    RUN_PROXY_INFERENCE = "model.proxy_inference"
    RUN_PROXY_INFERENCE_REP = "model.proxy_inference.rep"
    RUN_PROXY_STATEMENT = "statement.proxy_run"
    RUN_PROXY_STATEMENT_REP = "statement.proxy_run.rep"
    WAKE_RUNTIME = "runtime.wake"
    WAKE_RUNTIME_REP = "runtime.wake.rep"

    # worker management/lifecycle
    CONFIGURE_WORKER_SET = "worker_set.configure"
    CONFIGURE_WORKER_SET_REP = "worker_set.configure.rep"
    WAKE_WORKER_SET = "worker_set.wake"
    WAKE_WORKER_SET_REP = "worker_set.wake.rep"
    RESTART_WORKER_SET = "worker_set.restart"
    RESTART_WORKER_SET_REP = "worker_set.restart.rep"
    DO_RESTART_WORKER_NODE = "worker_set.do_restart"
    DO_RESTART_WORKER_NODE_REP = "worker_set.do_restart.rep"
    GET_ENVIRONMENT = "worker_set.get_environment"
    GET_ENVIRONMENT_REP = "worker_set.get_environment.rep"
    PING_WORKER_SET = "worker_set.ping"
    PING_WORKER_SET_REP = "worker_set.ping.rep"
    PULL_WORKER_RUNS = "worker_set.pull_scheduled_runs"
    PULL_WORKER_RUNS_REP = "worker_set.pull_scheduled_runs.rep"
    # running (routed via project id, maybe later worker set/node/process as well)
    START_RUN = "run.start"
    START_RUN_REP = "run.start.rep"
    KILL_RUN = "run.kill"
    KILL_RUN_REP = "run.kill.rep"
    PAUSE_RUN = "run.pause"
    PAUSE_RUN_REP = "run.pause.rep"
    RESUME_RUN = "run.resume"
    RESUME_RUN_REP = "run.resume.rep"


REPLY_BY_REQUEST_TYPE = {
    NMessageType.CONFIGURE_WORKER_SET: NMessageType.CONFIGURE_WORKER_SET_REP,
    NMessageType.WAKE_WORKER_SET: NMessageType.WAKE_WORKER_SET_REP,
    NMessageType.RESTART_WORKER_SET: NMessageType.RESTART_WORKER_SET_REP,
    NMessageType.DO_RESTART_WORKER_NODE: NMessageType.DO_RESTART_WORKER_NODE_REP,
    NMessageType.READ_MODULE: NMessageType.READ_MODULE_REP,
    NMessageType.WRITE_EDITS: NMessageType.WRITE_EDITS_REP,
    NMessageType.PASTE_NODES: NMessageType.PASTE_NODES_REP,
    NMessageType.SNAPSHOT_MODULE: NMessageType.SNAPSHOT_MODULE_REP,
    NMessageType.WRITE_SESSION: NMessageType.WRITE_SESSION_REP,
    NMessageType.DOWNLOAD_BLOB: NMessageType.DOWNLOAD_BLOB_REP,
    NMessageType.SEARCH_RECORDS: NMessageType.SEARCH_RECORDS_REP,
    NMessageType.UPLOAD_BLOB: NMessageType.UPLOAD_BLOB_REP,
    NMessageType.MARK_UPLOADED_BLOB: NMessageType.MARK_UPLOADED_BLOB_REP,
    NMessageType.REVEAL_SECRET: NMessageType.REVEAL_SECRET_REP,
    NMessageType.RUN_PROXY_INFERENCE: NMessageType.RUN_PROXY_INFERENCE_REP,
    NMessageType.RUN_PROXY_STATEMENT: NMessageType.RUN_PROXY_STATEMENT_REP,
    NMessageType.START_RUN: NMessageType.START_RUN_REP,
    NMessageType.KILL_RUN: NMessageType.KILL_RUN_REP,
    NMessageType.PAUSE_RUN: NMessageType.PAUSE_RUN_REP,
    NMessageType.RESUME_RUN: NMessageType.RESUME_RUN_REP,
    NMessageType.WAKE_RUNTIME: NMessageType.WAKE_RUNTIME_REP,
    NMessageType.GET_ENVIRONMENT: NMessageType.GET_ENVIRONMENT_REP,
    NMessageType.PING_WORKER_SET: NMessageType.PING_WORKER_SET_REP,
    NMessageType.PULL_WORKER_RUNS: NMessageType.PULL_WORKER_RUNS_REP,
}
REQUEST_BY_REPLY_TYPE = {v: k for k, v in REPLY_BY_REQUEST_TYPE.items()}

# assert that all REQUEST types have a REPLY type
_reply_types = {t for t in NMessageType if t.name.endswith("REP")}
_missing_request_types = _reply_types - set(REPLY_BY_REQUEST_TYPE.values())
assert not _missing_request_types, f"missing reply types for {_missing_request_types}"

#
# All messages are just Python dataclasses.
# They are serialized and deserialized in serialize.py with some custom logic
#  to support all the nested Python typing we need (e.g. NamedTuples).
# In the future we should want to use a more formal serialization format (and RPC - proto?),
# but for the time being this is both fast enough and flexible.
#  :WireFormat
#


ClientOrigin = typing.NamedTuple(
    "ClientOrigin", [("type", str), ("id", typing.Union[UUID, str]), ("nonce", Optional[UUID])]
)


class BatchablePayload(abc.ABC):
    @staticmethod
    @abc.abstractmethod
    def batch(messages: list["BatchablePayload"]) -> "BatchablePayload":
        raise NotImplementedError


@dataclass
class HasOrigin:
    origins: list[ClientOrigin]

    @property
    def origin(self):
        return self.origins[0]

    def has_origin(self, id: UUID | str, nonce: Optional[UUID | str] = None) -> bool:
        id = try_to_uuid(id)
        if nonce is None:
            return any(c.id == id for c in self.origins)
        else:
            nonce = nonce if isinstance(nonce, UUID) else UUID(nonce)
            return any(c.id == id and c.nonce == nonce for c in self.origins)


@dataclass
class ProjectScoped:
    project_id: UUID

    @property
    def topic(self):
        return f"{self.__class__.type}.{self.project_id}"  # type: ignore


@dataclass
class ModuleScoped:
    project_id: UUID
    module_id: UUID

    @property
    def topic(self):
        return f"{self.__class__.type}.{self.project_id}.{self.module_id}"  # type: ignore


@dataclass(repr=False, slots=True)  # not sure where to put this?
class ClientData:
    id: UUID
    created_at: datetime
    last_seen_at: datetime
    closed_at: Optional[datetime]
    user_id: UUID
    type: str
    device_name: Optional[str]
    browser_name: Optional[str]
    project_id: Optional[UUID]
    project_version_id: Optional[UUID]
    file_id: Optional[UUID]
    statement_id: Optional[UUID]
    field_id: Optional[UUID]
    record_id: Optional[UUID]
    path: Optional[str]


@payload(NMessageType.CLIENT_CHANGED)
class ClientChangedPayload(Payload):
    origin: ClientOrigin
    client: ClientData


@payload(NMessageType.PROJECT_CHANGED)
class ProjectChangedPayload(ProjectScoped, HasOrigin, Payload):
    pass


@payload(NMessageType.MODULE_CHANGED)
class ModuleChangedPayload(ModuleScoped, HasOrigin, Payload):
    edits: list[EditData]


@payload(NMessageType.START_RUN)
class ReqStartRunPayload(ModuleScoped, Payload):
    trigger_type: TriggerType
    trigger_id: Optional[UUID] = None
    run_id: Optional[UUID] = None
    session_id: Optional[UUID] = None
    statement: Optional[UUID | str] = None
    scope: Optional[UUID | str] = None
    code: Optional[str] = None
    scheduled_at: Optional[datetime] = None
    inputs: dict[str, typing.Any] = None
    block: Optional[float] = None
    keyed: bool = True
    keyed_return: bool = True
    tags: Optional[list[str]] = None
    root_value: Optional[dict[str, typing.Any]] = None
    global_value: Optional[dict[str, typing.Any]] = None
    access_level: Optional[SessionAccessLevel] = None


class StartRunErrorType(enum.StrEnum):
    UNAVAILABLE = "unavailable"
    INVALID_RUN = "invalid_run"
    INTERNAL_ERROR = "internal_error"
    TIMEOUT = "timeout"
    RUNTIME_ERROR = "runtime_error"
    ALREADY_PREPARED = "already_prepared"


@payload(NMessageType.START_RUN_REP)
class RepStartRunPayload(Payload):
    error: Optional[StartRunErrorType] = None
    run_id: Optional[UUID] = None
    run: Optional[RunData] = None
    logs: Optional[list[LogEntryData]] = None


@payload(NMessageType.KILL_RUN)
class ReqKillRunPayload(ModuleScoped, Payload):
    run_id: UUID
    session_id: Optional[UUID]


@payload(NMessageType.KILL_RUN_REP)
class RepKillRunPayload(Payload):
    success: bool


@payload(NMessageType.SESSION_CHANGED)
class SessionChangedPayload(ModuleScoped, Payload):
    session: SessionData
    runs: list[RunData]


@payload(NMessageType.RUNS_CHANGED)
class RunsChangedGlobalPayload(Payload):
    runs: list[RunData]


@payload(NMessageType.WORKERS_CHANGED)
class WorkersChangedPayload(ProjectScoped, Payload):
    worker_sets: list[WorkerSetData]


@payload(NMessageType.LOGS_CHANGED)
class LogsChangedPayload(ModuleScoped, Payload):
    logs: list[LogEntryData]


@payload(NMessageType.READ_MODULE)
class ReqReadModulePayload(Payload):
    ref: typing.Union[ModuleReference, UUID]


@payload(NMessageType.READ_MODULE_REP)
class RepReadModulePayload(Payload):
    module: ModuleTreeData
    project_id: UUID
    os_name: str
    pg_name: str


@payload(NMessageType.WRITE_EDITS)
class ReqWriteEditsPayload(Payload):
    module_id: UUID
    edits: list[EditData]
    client: ClientOrigin


@payload(NMessageType.WRITE_EDITS_REP)
class RepWriteEditsPayload(Payload):
    nodes: list[NodeData]
    success: bool
    error: Optional[str] = None


@payload(NMessageType.PASTE_NODES)
class ReqPasteNodesPayload(Payload):
    source_module_id: UUID
    target_module_id: UUID
    source_ids: list[UUID]
    target_ids: dict[UUID, UUID]
    target_cks: dict[UUID, UUID]
    target_parent_ids: dict[UUID, UUID]
    target_order_keys: dict[UUID, str]
    client: ClientOrigin


@payload(NMessageType.PASTE_NODES_REP)
class RepPasteNodesPayload(Payload):
    nodes: list[NodeData]
    success: bool
    error: Optional[str] = None


@payload(NMessageType.SNAPSHOT_MODULE)
class ReqSnapshotModulePayload(Payload):
    module_id: UUID
    client: ClientOrigin
    name: Optional[str] = None
    tag: Optional[str] = None
    description: Optional[str] = None


@payload(NMessageType.SNAPSHOT_MODULE_REP)
class RepSnapshotModulePayload(Payload):
    success: bool
    error: Optional[str] = None


@payload(NMessageType.WRITE_SESSION)
class ReqWriteSessionPayload(Payload):
    module_id: UUID
    session: Optional[SessionData]
    runs: list[RunData]
    client: ClientOrigin


@payload(NMessageType.WRITE_SESSION_REP)
class RepWriteSessionPayload(Payload):
    success: bool
    error: Optional[str] = None


@dataclass
class ReqSearch(abc.ABC, Payload):
    query: Optional[ExpressionData] = None
    sort: Optional[list[ExpressionData]] = None
    after: Optional[str] = None
    limit: Optional[int] = None
    count: bool = False
    engine: Optional[QueryEngine] = None


@dataclass
class RepSearch(abc.ABC):
    total: Optional[int]
    limit: int
    engine: Optional[QueryEngine]
    error: Optional[str] = None


@payload(NMessageType.SEARCH_RECORDS)
class ReqSearchRecordsPayload(ReqSearch, Payload):
    module_id: UUID = required_field()
    statement_id: UUID = required_field()
    statement_ck: UUID = required_field()
    statement_key: str = required_field()


@payload(NMessageType.SEARCH_RECORDS_REP)
class RepSearchRecordsPayload(RepSearch, Payload):
    records: Optional[list[RecordData]] = None
    cursors: Optional[list[str]] = None


@payload(NMessageType.DOWNLOAD_BLOB)
class ReqDownloadBlobPayload(Payload):
    blobs: list[BlobData]


@payload(NMessageType.DOWNLOAD_BLOB_REP)
class RepDownloadBlobPayload(Payload):
    get_urls: list[typing.Union[str, None]]


@payload(NMessageType.UPLOAD_BLOB)
class ReqUploadBlobPayload(Payload):
    module_id: UUID
    blobs: list[BlobData]


@payload(NMessageType.UPLOAD_BLOB_REP)
class RepUploadBlobPayload(Payload):
    blobs: list[BlobData]
    post_urls: list[typing.Union[str, None]]


@payload(NMessageType.MARK_UPLOADED_BLOB)
class ReqMarkUploadedBlobPayload(Payload):
    blobs: list[BlobData]


@payload(NMessageType.MARK_UPLOADED_BLOB_REP)
class RepMarkUploadedBlobPayload(Payload):
    success: bool


@payload(NMessageType.REVEAL_SECRET)
class ReqRevealSecretPayload(Payload):
    secrets: list[SecretData]


@payload(NMessageType.REVEAL_SECRET_REP)
class RepRevealSecretPayload(Payload):
    secrets: list[SecretData]


@payload(NMessageType.RUN_PROXY_INFERENCE)
class ReqRunInferencePayload(Payload):
    project_id: UUID
    model_path: str
    inputs: typing.Any
    timeout: int
    run_id: UUID


@payload(NMessageType.RUN_PROXY_INFERENCE_REP)
class RepRunInferencePayload(Payload):
    outputs: Optional[typing.Any] = None
    error_kind: Optional[ModelErrorType] = None
    error_message: Optional[str] = None


@payload(NMessageType.RUN_PROXY_STATEMENT)
class ReqRunStatementPayload(Payload):
    project_id: UUID
    module_name: str
    statement: str
    inputs: typing.Any


@payload(NMessageType.RUN_PROXY_STATEMENT_REP)
class RepRunStatementPayload(Payload):
    outputs: Optional[typing.Any] = None
    error: Optional[typing.Any] = None


@payload(NMessageType.WAKE_RUNTIME)
class ReqWakeRuntimePayload(Payload):
    module_id: UUID


@payload(NMessageType.WAKE_RUNTIME_REP)
class RepWakeRuntimePayload(Payload):
    module_id: UUID


@payload(NMessageType.CONFIGURE_WORKER_SET)
class ReqConfigureWorkerSetPayload(Payload):
    project_id: UUID
    profile: WorkerProfile
    region: ProjectRegion
    target_replicas: int


@payload(NMessageType.CONFIGURE_WORKER_SET_REP)
class RepConfigureWorkerSetPayload(Payload):
    worker_set_id: UUID
    success: bool


@payload(NMessageType.WAKE_WORKER_SET)
class ReqWakeWorkerSetPayload(Payload):
    project_id: UUID


@payload(NMessageType.WAKE_WORKER_SET_REP)
class RepWakeWorkerSetPayload(Payload):
    worker_set_id: Optional[UUID]
    success: bool


@payload(NMessageType.RESTART_WORKER_SET)
class ReqRestartWorkerSetPayload(Payload):
    project_id: UUID


@payload(NMessageType.RESTART_WORKER_SET_REP)
class RepRestartWorkerSetPayload(Payload):
    worker_set_id: Optional[UUID]
    success: bool


@payload(NMessageType.DO_RESTART_WORKER_NODE)
class ReqDoRestartWorkerNodePayload(Payload):
    project_id: UUID
    worker_set_id: Optional[UUID]
    worker_node_id: Optional[str]
    worker_process_id: Optional[str]

    @property
    def topic(self) -> str:
        return f"{self.__class__.type}.{self.project_id}.{self.worker_set_id or 'all'}"  # type: ignore


@payload(NMessageType.DO_RESTART_WORKER_NODE_REP)
class RepDoRestartWorkerNodePayload(Payload):
    worker_set_id: Optional[UUID]
    worker_node_id: Optional[str]
    worker_process_id: Optional[str]
    success: bool


@payload(NMessageType.GET_ENVIRONMENT)
class ReqGetEnvironmentPayload(ProjectScoped, Payload):
    pass


@payload(NMessageType.GET_ENVIRONMENT_REP)
class RepGetEnvironmentPayload(Payload):
    environment: EnvironmentData


@payload(NMessageType.PING_WORKER_SET)
class ReqPingWorkerSetPayload(ProjectScoped, Payload):
    pass


@payload(NMessageType.PING_WORKER_SET_REP)
class RepPingWorkerSetPayload(Payload):
    success: bool


@payload(NMessageType.PULL_WORKER_RUNS)
class ReqPullWorkerRunsPayload(Payload):
    project_id: UUID
    module_id: UUID
    worker_set_id: UUID
    worker_node_id: Optional[str]
    worker_process_id: Optional[str]


@payload(NMessageType.PULL_WORKER_RUNS_REP)
class RepPullWorkerRunsPayload(Payload):
    runs: list[RunData]
    success: bool


# invert REGISTERED_MESSAGE_PAYLOADS
MESSAGE_TYPE_BY_PAYLOAD_CLASS: dict[typing.Type, "NMessageType"] = {
    payload_class: message_type
    for message_type, payload_class in REGISTERED_MESSAGE_PAYLOADS.items()
}
