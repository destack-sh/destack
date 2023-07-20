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
from bench.language.wire import (
    LogEntryData,
    ModuleTreeData,
    RecordData,
    RemoteObjectData,
    RunData,
    SecretData,
    SessionData,
)
from bench.utils.utils import required_field

REGISTERED_MESSAGE_PAYLOADS: dict["NMessageType", typing.Type] = {}


def payload(message_type: "NMessageType"):
    def wrapper(cls):
        if message_type in REGISTERED_MESSAGE_PAYLOADS:
            raise RuntimeError(f"message type {message_type} already registered")
        cls = dataclass(cls, slots=True)  # noqa: this is fine
        REGISTERED_MESSAGE_PAYLOADS[message_type] = cls
        return cls

    return wrapper


class NMessageType(StrEnum):
    """All messages types"""

    # Bench sync
    CLIENT_CHANGED = "client.changed"
    PROJECT_CHANGED = "project.changed"
    COMMENT_CHANGED = "comment.changed"
    SCREEN_CHANGED = "screen.changed"
    MODULE_CHANGED = "module.changed"
    MODULE_INTERNAL_CHANGED = "module.internal.changed"  # for internal
    SESSION_CHANGED = "session.changed"
    LOGS_CHANGED = "logs.changed"
    RUN_MARKED_DEAD = "execution.marked_dead"

    # Worker <-> Internal
    REQUEST_REGISTER_WORKER = "worker.register"
    REPLY_REGISTER_WORKER = "worker.register.rep"
    WORKER_HEARTBEAT = "worker.heartbeat"
    REQUEST_READ_MODULE = "module.read"
    REPLY_READ_MODULE = "module.read.rep"
    REQUEST_WRITE_MODULE = "module.write"
    REPLY_WRITE_MODULE = "module.write.rep"
    REQUEST_WRITE_SESSION = "session.write"
    REPLY_WRITE_SESSION = "session.write.rep"
    REQUEST_SEARCH_RECORD = "module.search.record"
    REPLY_SEARCH_RECORD = "module.search.record.rep"
    REQUEST_SEARCH_RUN = "module.search.run"
    REPLY_SEARCH_RUN = "module.search.run.rep"
    REQUEST_SEARCH_LOG = "module.search.log"
    REPLY_SEARCH_LOG = "module.search.log.rep"
    REQUEST_READ_OBJECT = "object.read"
    REPLY_READ_OBJECT = "object.read.rep"
    REQUEST_WRITE_OBJECT = "object.write"
    REPLY_WRITE_OBJECT = "object.write.rep"
    REQUEST_MARK_UPLOADED_OBJECT = "object.mark_uploaded"
    REPLY_MARK_UPLOADED_OBJECT = "object.mark_uploaded.rep"
    REQUEST_READ_SECRET = "secret.read"
    REPLY_READ_SECRET = "secret.read.rep"
    REQUEST_RUN_INFERENCE = "model.inference"
    REPLY_RUN_INFERENCE = "model.inference.rep"

    # API <-> Worker
    REQUEST_RUN = "run"
    REPLY_RUN = "run.rep"
    REQUEST_CANCEL_RUN = "run.cancel"
    REPLY_CANCEL_RUN = "run.cancel.rep"
    REQUEST_LANGSERVER = "langserver.get"
    REPLY_LANGSERVER = "langserver.get.rep"


REPLY_BY_REQUEST_TYPE = {
    NMessageType.REQUEST_REGISTER_WORKER: NMessageType.REPLY_REGISTER_WORKER,
    NMessageType.REQUEST_READ_MODULE: NMessageType.REPLY_READ_MODULE,
    NMessageType.REQUEST_WRITE_MODULE: NMessageType.REPLY_WRITE_MODULE,
    NMessageType.REQUEST_WRITE_SESSION: NMessageType.REPLY_WRITE_SESSION,
    NMessageType.REQUEST_READ_OBJECT: NMessageType.REPLY_READ_OBJECT,
    NMessageType.REQUEST_WRITE_OBJECT: NMessageType.REPLY_WRITE_OBJECT,
    NMessageType.REQUEST_MARK_UPLOADED_OBJECT: NMessageType.REPLY_MARK_UPLOADED_OBJECT,
    NMessageType.REQUEST_SEARCH_RECORD: NMessageType.REPLY_SEARCH_RECORD,
    NMessageType.REQUEST_SEARCH_RUN: NMessageType.REPLY_SEARCH_RUN,
    NMessageType.REQUEST_SEARCH_LOG: NMessageType.REPLY_SEARCH_LOG,
    NMessageType.REQUEST_READ_SECRET: NMessageType.REPLY_READ_SECRET,
    NMessageType.REQUEST_RUN_INFERENCE: NMessageType.REPLY_RUN_INFERENCE,
    NMessageType.REQUEST_RUN: NMessageType.REPLY_RUN,
    NMessageType.REQUEST_CANCEL_RUN: NMessageType.REPLY_CANCEL_RUN,
    NMessageType.REQUEST_LANGSERVER: NMessageType.REPLY_LANGSERVER,
}
REQUEST_BY_REPLY_TYPE = {v: k for k, v in REPLY_BY_REQUEST_TYPE.items()}

# assert that all REQUEST types have a REPLY type
_request_types = {t for t in NMessageType if t.name.startswith("REQUEST")}
_missing_reply_types = _request_types - set(REPLY_BY_REQUEST_TYPE.keys())
assert not _missing_reply_types, f"missing reply types for {_missing_reply_types}"

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
class ClientChangedPayload:
    origin: ClientOrigin
    client: ClientData


@payload(NMessageType.PROJECT_CHANGED)
class ProjectChangedPayload(OriginPayload):
    project_id: UUID


@payload(NMessageType.MODULE_CHANGED)
class ModuleChangedPayload(OriginPayload):
    module_id: UUID
    mutations: list[ModuleMutation]


@payload(NMessageType.MODULE_INTERNAL_CHANGED)
class ModuleInternalChangedPayload(OriginPayload):
    module_id: UUID
    mutations: list[ModuleMutation]


@payload(NMessageType.REQUEST_REGISTER_WORKER)
class ReqRegisterWorkerPayload:
    worker_id: UUID
    project_id: Optional[UUID]
    tenancy: str


@payload(NMessageType.REPLY_REGISTER_WORKER)
class RepRegisterWorkerPayload:
    success: bool


@payload(NMessageType.WORKER_HEARTBEAT)
class WorkerHeartbeatPayload:
    worker_id: UUID


@payload(NMessageType.REQUEST_RUN)
class ReqRunPayload:
    module_id: UUID
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
    INTERNAL_ERROR = "internal_error"
    NOT_READY = "not_ready"
    INVALID_RUNCONFIG = "invalid_runconfig"
    TIMEOUT = "timeout"
    RUNTIME_ERROR = "runtime_error"


@payload(NMessageType.REPLY_RUN)
class RepRunPayload:
    error: Optional[RunErrorType] = None
    run_id: Optional[UUID] = None
    run: Optional[RunData] = None
    logs: Optional[list[LogEntryData]] = None


@payload(NMessageType.REQUEST_CANCEL_RUN)
class ReqCancelRunPayload:
    module_id: UUID
    run_id: UUID


@payload(NMessageType.REPLY_CANCEL_RUN)
class RepCancelRunPayload:
    success: bool


@payload(NMessageType.RUN_MARKED_DEAD)
class RunMarkedDeadPayload:
    module_id: UUID
    run_id: UUID


@payload(NMessageType.SESSION_CHANGED)
class SessionChangedPayload:
    module_id: UUID
    session: SessionData
    runs: list[RunData]


@payload(NMessageType.LOGS_CHANGED)
class LogsChangedPayload:
    module_id: UUID
    logs: list[LogEntryData]


@payload(NMessageType.REQUEST_READ_MODULE)
class ReqReadModulePayload:
    ref: typing.Union[ModuleReference, UUID]


@payload(NMessageType.REPLY_READ_MODULE)
class RepReadModulePayload:
    module: ModuleTreeData
    project_id: UUID


@payload(NMessageType.REQUEST_WRITE_MODULE)
class ReqWriteModulePayload:
    module_id: UUID
    mutations: list[ModuleMutation]
    client: ClientOrigin
    wait: bool


@payload(NMessageType.REPLY_WRITE_MODULE)
class RepWriteModulePayload:
    success: bool


@payload(NMessageType.REQUEST_WRITE_SESSION)
class ReqWriteSessionPayload:
    module_id: UUID
    session: SessionData
    runs: list[RunData]
    logs: list[LogEntryData]
    client: ClientOrigin


@payload(NMessageType.REPLY_WRITE_SESSION)
class RepWriteSessionPayload:
    success: bool


@dataclass
class ReqSearch(abc.ABC):
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


@payload(NMessageType.REQUEST_SEARCH_RECORD)
class ReqSearchRecordPayload(ReqSearch):
    module_id: UUID = required_field()
    statement_ids: Optional[list[UUID]] = None
    backend_ids: Optional[list[str]] = None


@payload(NMessageType.REPLY_SEARCH_RECORD)
class RepSearchRecordPayload(RepSearch):
    elements: Optional[list[RecordData]] = None


@payload(NMessageType.REQUEST_SEARCH_RUN)
class ReqSearchRunPayload(ReqSearch):
    module_id: UUID = required_field()
    runnables_ids: Optional[list[UUID]] = None


@payload(NMessageType.REPLY_SEARCH_RUN)
class RepSearchRunPayload(RepSearch):
    elements: Optional[list[RunData]] = None


@payload(NMessageType.REQUEST_SEARCH_LOG)
class ReqSearchLogPayload(ReqSearch):
    module_id: UUID = required_field()
    runnables_ids: Optional[list[UUID]] = None


@payload(NMessageType.REPLY_SEARCH_LOG)
class RepSearchLogPayload(RepSearch):
    elements: Optional[list[LogEntryData]] = None


@payload(NMessageType.REQUEST_READ_OBJECT)
class ReqReadObjectPayload:
    objects: list[RemoteObjectData]


@payload(NMessageType.REPLY_READ_OBJECT)
class RepReadObjectPayload:
    get_urls: list[typing.Union[str, None]]


@payload(NMessageType.REQUEST_WRITE_OBJECT)
class ReqWriteObjectPayload:
    module_id: UUID
    objects: list[RemoteObjectData]


@payload(NMessageType.REPLY_WRITE_OBJECT)
class RepWriteObjectPayload:
    objects: list[RemoteObjectData]
    post_urls: list[typing.Union[str, None]]


@payload(NMessageType.REQUEST_MARK_UPLOADED_OBJECT)
class ReqMarkUploadedObjectPayload:
    objects: list[RemoteObjectData]


@payload(NMessageType.REPLY_MARK_UPLOADED_OBJECT)
class RepMarkUploadedObjectPayload:
    success: bool


@payload(NMessageType.REQUEST_READ_SECRET)
class ReqReadSecretPayload:
    secrets: list[SecretData]


@payload(NMessageType.REPLY_READ_SECRET)
class RepReadSecretPayload:
    secrets: list[SecretData]


@payload(NMessageType.REQUEST_RUN_INFERENCE)
class ReqRunInferencePayload:
    model_path: str
    inputs: typing.Any
    timeout: int


@payload(NMessageType.REPLY_RUN_INFERENCE)
class RepRunInferencePayload:
    outputs: Optional[typing.Any] = None
    timeout: bool = False


@payload(NMessageType.REQUEST_LANGSERVER)
class ReqLangserverPayload:
    module_id: UUID


@payload(NMessageType.REPLY_LANGSERVER)
class RepLangserverPayload:
    module_id: UUID


# invert REGISTERED_MESSAGE_PAYLOADS
MESSAGE_TYPE_BY_PAYLOAD_CLASS: dict[typing.Type, "NMessageType"] = {
    payload_class: message_type
    for message_type, payload_class in REGISTERED_MESSAGE_PAYLOADS.items()
}

PROJECT_SCOPED_PAYLOAD_TYPES = (ProjectChangedPayload,)

MODULE_SCOPED_PAYLOAD_TYPES = (
    ModuleChangedPayload,
    ModuleInternalChangedPayload,
    SessionChangedPayload,
    LogsChangedPayload,
    RunMarkedDeadPayload,
)


def to_topic(
    message_type: NMessageType,
    payload: NMessageType,
) -> str:
    """
    Gets the default topic for a message type and payload.
    :NATSTopics
    """
    if isinstance(payload, PROJECT_SCOPED_PAYLOAD_TYPES):
        return f"{message_type}.{payload.project_id}"
    elif isinstance(payload, MODULE_SCOPED_PAYLOAD_TYPES):
        return f"{message_type}.{payload.module_id}"

    return message_type
