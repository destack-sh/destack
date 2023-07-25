# increment when making backwards-incompatible changes to messages
import abc
import enum
import typing
from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum
from typing import Optional
from uuid import UUID

from bench.language.const import RunTriggerType
from bench.language.core import ModuleReference
from bench.language.mutate import ModuleMutation
from bench.language.query import Query, Sort
from bench.language.session import WorkerProfile, WorkerRegion
from bench.language.wire import (
    EnvironmentData,
    LogEntryData,
    ModuleTreeData,
    RecordData,
    RemoteObjectData,
    RunData,
    SecretData,
    SessionData,
    WorkerSetData,
)
from bench.utils.utils import required_field

REGISTERED_MESSAGE_PAYLOADS: dict["NMessageType", typing.Type] = {}


class Payload:
    @property
    def topic(self):
        return self.__class__.type.value


def payload(message_type: "NMessageType"):
    def wrapper(cls):
        if message_type in REGISTERED_MESSAGE_PAYLOADS:
            raise RuntimeError(f"message type {message_type} already registered")
        cls = dataclass(cls, slots=True)  # noqa: this is fine
        REGISTERED_MESSAGE_PAYLOADS[message_type] = cls
        cls.type = message_type
        return cls

    return wrapper


class NMessageType(StrEnum):
    """All messages types"""

    # for sync
    CLIENT_CHANGED = "client.changed"
    PROJECT_CHANGED = "project.changed"
    COMMENT_CHANGED = "comment.changed"
    SCREEN_CHANGED = "screen.changed"
    MODULE_CHANGED = "module.changed"
    MODULE_INTERNAL_CHANGED = "module.internal.changed"  # for internal sync
    SESSION_CHANGED = "session.changed"
    LOGS_CHANGED = "logs.changed"
    WORKERS_CHANGED = "workers.changed"
    RUN_MARKED_DEAD = "run.marked_dead"

    # read/write via server
    READ_MODULE = "module.read"
    READ_MODULE_REP = "module.read.rep"
    WRITE_MODULE = "module.write"
    WRITE_MODULE_REP = "module.write.rep"
    WRITE_SESSION = "session.write"
    WRITE_SESSION_REP = "session.write.rep"
    SEARCH_RECORD = "module.search.record"
    SEARCH_RECORD_REP = "module.search.record.rep"
    SEARCH_RUN = "module.search.run"
    SEARCH_RUN_REP = "module.search.run.rep"
    SEARCH_LOG = "module.search.log"
    SEARCH_LOG_REP = "module.search.log.rep"
    READ_OBJECT = "object.read"
    READ_OBJECT_REP = "object.read.rep"
    WRITE_OBJECT = "object.write"
    WRITE_OBJECT_REP = "object.write.rep"
    MARK_UPLOADED_OBJECT = "object.mark_uploaded"
    MARK_UPLOADED_OBJECT_REP = "object.mark_uploaded.rep"
    READ_SECRET = "secret.read"
    READ_SECRET_REP = "secret.read.rep"
    RUN_PROXY_INFERENCE = "model.proxy_inference"
    RUN_PROXY_INFERENCE_REP = "model.proxy_inference.rep"
    WAKE_LANGSERVER = "langserver.wake"
    WAKE_LANGSERVER_REP = "langserver.wake.rep"

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
    # running (routed via project id)
    START_RUN = "run.start"
    START_RUN_REP = "run.start.rep"
    CANCEL_RUN = "run.cancel"
    CANCEL_RUN_REP = "run.cancel.rep"
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
    NMessageType.WRITE_MODULE: NMessageType.WRITE_MODULE_REP,
    NMessageType.WRITE_SESSION: NMessageType.WRITE_SESSION_REP,
    NMessageType.READ_OBJECT: NMessageType.READ_OBJECT_REP,
    NMessageType.WRITE_OBJECT: NMessageType.WRITE_OBJECT_REP,
    NMessageType.MARK_UPLOADED_OBJECT: NMessageType.MARK_UPLOADED_OBJECT_REP,
    NMessageType.SEARCH_RECORD: NMessageType.SEARCH_RECORD_REP,
    NMessageType.SEARCH_RUN: NMessageType.SEARCH_RUN_REP,
    NMessageType.SEARCH_LOG: NMessageType.SEARCH_LOG_REP,
    NMessageType.READ_SECRET: NMessageType.READ_SECRET_REP,
    NMessageType.RUN_PROXY_INFERENCE: NMessageType.RUN_PROXY_INFERENCE_REP,
    NMessageType.START_RUN: NMessageType.START_RUN_REP,
    NMessageType.CANCEL_RUN: NMessageType.CANCEL_RUN_REP,
    NMessageType.PAUSE_RUN: NMessageType.PAUSE_RUN_REP,
    NMessageType.RESUME_RUN: NMessageType.RESUME_RUN_REP,
    NMessageType.WAKE_LANGSERVER: NMessageType.WAKE_LANGSERVER_REP,
    NMessageType.GET_ENVIRONMENT: NMessageType.GET_ENVIRONMENT_REP,
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
# In the future we should want to use a more formal serialization format,
# but for the time being this is both fast enough and flexible.
#  :WireFormat
#


ClientOrigin = typing.NamedTuple(
    "ClientOrigin", [("type", str), ("id", UUID), ("nonce", Optional[UUID])]
)


class BatchablePayload(abc.ABC):
    @staticmethod
    @abc.abstractmethod
    def batch(messages: list["BatchablePayload"]) -> "BatchablePayload":
        raise NotImplementedError


@dataclass
class OriginPayload:
    origins: list[ClientOrigin]

    @property
    def origin(self):
        return self.origins[0]

    def has_origin(self, id: UUID | str, nonce: Optional[UUID | str] = None) -> bool:
        id = id if isinstance(id, UUID) else UUID(id)
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
        return f"{self.__class__.type}.{self.project_id}".replace("-", "")


@dataclass
class ModuleScoped:
    project_id: UUID
    module_id: UUID

    @property
    def topic(self):
        return f"{self.__class__.type}.{self.project_id}.{self.module_id}".replace("-", "")


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
class ProjectChangedPayload(ProjectScoped, OriginPayload):
    pass


@payload(NMessageType.MODULE_CHANGED)
class ModuleChangedPayload(ModuleScoped, OriginPayload):
    mutations: list[ModuleMutation]


@payload(NMessageType.MODULE_INTERNAL_CHANGED)
class ModuleInternalChangedPayload(ModuleScoped, OriginPayload):
    mutations: list[ModuleMutation]


@payload(NMessageType.START_RUN)
class ReqStartRunPayload(ModuleScoped, Payload):
    runnable: Optional[UUID | str]
    runnable_type: Optional[str]
    arguments: dict[str, typing.Any]
    block: bool
    keyed: bool
    trigger_type: RunTriggerType
    trigger_id: Optional[UUID]
    session_id: Optional[UUID]
    run_id: Optional[UUID]


class RunErrorType(enum.StrEnum):
    UNAVAILABLE = "unavailable"
    INTERNAL_ERROR = "internal_error"
    NOT_READY = "not_ready"
    INVALID_RUNCONFIG = "invalid_runconfig"
    TIMEOUT = "timeout"
    RUNTIME_ERROR = "runtime_error"


@payload(NMessageType.START_RUN_REP)
class RepStartRunPayload(Payload):
    error: Optional[RunErrorType] = None
    run_id: Optional[UUID] = None
    run: Optional[RunData] = None
    logs: Optional[list[LogEntryData]] = None


@payload(NMessageType.CANCEL_RUN)
class ReqCancelRunPayload(ModuleScoped, Payload):
    run_id: UUID


@payload(NMessageType.CANCEL_RUN_REP)
class RepCancelRunPayload(Payload):
    success: bool


@payload(NMessageType.RUN_MARKED_DEAD)
class RunMarkedDeadPayload(Payload):
    module_id: UUID
    run_id: UUID

    @property
    def topic(self) -> str:
        return f"{self.__class__.type}.{self.module_id}"


@payload(NMessageType.SESSION_CHANGED)
class SessionChangedPayload(ModuleScoped, Payload):
    session: SessionData
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


@payload(NMessageType.WRITE_MODULE)
class ReqWriteModulePayload(Payload):
    module_id: UUID
    mutations: list[ModuleMutation]
    client: ClientOrigin
    wait: bool


@payload(NMessageType.WRITE_MODULE_REP)
class RepWriteModulePayload(Payload):
    success: bool


@payload(NMessageType.WRITE_SESSION)
class ReqWriteSessionPayload(Payload):
    module_id: UUID
    session: SessionData
    runs: list[RunData]
    logs: list[LogEntryData]
    client: ClientOrigin


@payload(NMessageType.WRITE_SESSION_REP)
class RepWriteSessionPayload(Payload):
    success: bool


@dataclass
class ReqSearch(abc.ABC, Payload):
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    after: Optional[str] = None
    limit: Optional[int] = None
    count: bool = False


@dataclass
class RepSearch(abc.ABC):
    total: Optional[int]
    limit: int
    start_cursor: Optional[str] = None
    end_cursor: Optional[str] = None
    error: Optional[str] = None


@payload(NMessageType.SEARCH_RECORD)
class ReqSearchRecordPayload(ReqSearch):
    module_id: UUID = required_field()
    statement_ids: Optional[list[UUID]] = None
    backend_ids: Optional[list[str]] = None


@payload(NMessageType.SEARCH_RECORD_REP)
class RepSearchRecordPayload(RepSearch):
    elements: Optional[list[RecordData]] = None


@payload(NMessageType.SEARCH_RUN)
class ReqSearchRunPayload(ReqSearch):
    module_id: UUID = required_field()
    runnables_ids: Optional[list[UUID]] = None


@payload(NMessageType.SEARCH_RUN_REP)
class RepSearchRunPayload(RepSearch):
    elements: Optional[list[RunData]] = None


@payload(NMessageType.SEARCH_LOG)
class ReqSearchLogPayload(ReqSearch):
    module_id: UUID = required_field()
    runnables_ids: Optional[list[UUID]] = None


@payload(NMessageType.SEARCH_LOG_REP)
class RepSearchLogPayload(RepSearch):
    elements: Optional[list[LogEntryData]] = None


@payload(NMessageType.READ_OBJECT)
class ReqReadObjectPayload(Payload):
    objects: list[RemoteObjectData]


@payload(NMessageType.READ_OBJECT_REP)
class RepReadObjectPayload(Payload):
    get_urls: list[typing.Union[str, None]]


@payload(NMessageType.WRITE_OBJECT)
class ReqWriteObjectPayload(Payload):
    module_id: UUID
    objects: list[RemoteObjectData]


@payload(NMessageType.WRITE_OBJECT_REP)
class RepWriteObjectPayload(Payload):
    objects: list[RemoteObjectData]
    post_urls: list[typing.Union[str, None]]


@payload(NMessageType.MARK_UPLOADED_OBJECT)
class ReqMarkUploadedObjectPayload(Payload):
    objects: list[RemoteObjectData]


@payload(NMessageType.MARK_UPLOADED_OBJECT_REP)
class RepMarkUploadedObjectPayload(Payload):
    success: bool


@payload(NMessageType.READ_SECRET)
class ReqReadSecretPayload(Payload):
    secrets: list[SecretData]


@payload(NMessageType.READ_SECRET_REP)
class RepReadSecretPayload(Payload):
    secrets: list[SecretData]


@payload(NMessageType.RUN_PROXY_INFERENCE)
class ReqRunInferencePayload(Payload):
    model_path: str
    inputs: typing.Any
    timeout: int


@payload(NMessageType.RUN_PROXY_INFERENCE_REP)
class RepRunInferencePayload(Payload):
    outputs: Optional[typing.Any] = None
    timeout: bool = False


@payload(NMessageType.WAKE_LANGSERVER)
class ReqWakeLangserverPayload(Payload):
    module_id: UUID


@payload(NMessageType.WAKE_LANGSERVER_REP)
class RepWakeLangserverPayload(Payload):
    module_id: UUID


@payload(NMessageType.CONFIGURE_WORKER_SET)
class ReqConfigureWorkerSetPayload(Payload):
    project_id: UUID
    profile: WorkerProfile
    region: WorkerRegion
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

    @property
    def topic(self) -> str:
        return f"{self.__class__.type}.{self.project_id}.{self.worker_set_id or 'all'}".replace(
            "-", ""
        )


@payload(NMessageType.DO_RESTART_WORKER_NODE_REP)
class RepDoRestartWorkerNodePayload(Payload):
    worker_set_id: Optional[UUID]
    worker_node_id: Optional[UUID]
    success: bool


@payload(NMessageType.GET_ENVIRONMENT)
class ReqGetEnvironmentPayload(ProjectScoped, Payload):
    project_id: UUID
    node_id: Optional[UUID]


@payload(NMessageType.GET_ENVIRONMENT_REP)
class RepGetEnvironmentPayload(Payload):
    environment: EnvironmentData


# invert REGISTERED_MESSAGE_PAYLOADS
MESSAGE_TYPE_BY_PAYLOAD_CLASS: dict[typing.Type, "NMessageType"] = {
    payload_class: message_type
    for message_type, payload_class in REGISTERED_MESSAGE_PAYLOADS.items()
}
