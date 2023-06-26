# increment when making backwards-incompatible changes to messages
import abc
import enum
import typing
from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum
from itertools import chain
from typing import Optional
from uuid import UUID

from bench.bench.const import ExecutionTriggerType
from bench.bench.core import ModuleReference
from bench.bench.mutate import ModuleMutation
from bench.bench.query import Query, Sort
from bench.bench.wire import (
    ExecutionFrameData,
    ModuleTreeData,
    RecordData,
    RemoteObjectData,
    SecretData,
    XBlockData,
)

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

    # Worker <-> Internal
    REQUEST_REGISTER_WORKER = "worker.register"
    REPLY_REGISTER_WORKER = "worker.register.rep"
    WORKER_HEARTBEAT = "worker.heartbeat"
    REQUEST_READ_MODULE = "module.read"
    REPLY_READ_MODULE = "module.read.rep"
    REQUEST_WRITE_MODULE = "module.write"
    REPLY_WRITE_MODULE = "module.write.rep"
    REQUEST_SEARCH_DATASET = "module.dataset.search"
    REPLY_SEARCH_DATASET = "module.dataset.search.rep"
    REQUEST_READ_OBJECT = "object.read"
    REPLY_READ_OBJECT = "object.read.rep"
    REQUEST_WRITE_OBJECT = "object.write"
    REPLY_WRITE_OBJECT = "object.write.rep"
    REQUEST_READ_SECRET = "secret.read"
    REPLY_READ_SECRET = "secret.read.rep"
    REQUEST_RUN_INFERENCE = "model.inference"
    REPLY_RUN_INFERENCE = "model.inference.rep"
    EXECUTION_CHANGED = "execution.changed"
    EXECUTION_SAVED = "execution.saved"
    EXECUTION_MARKED_DEAD = "execution.marked_dead"

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
    NMessageType.REQUEST_READ_OBJECT: NMessageType.REPLY_READ_OBJECT,
    NMessageType.REQUEST_WRITE_OBJECT: NMessageType.REPLY_WRITE_OBJECT,
    NMessageType.REQUEST_SEARCH_DATASET: NMessageType.REPLY_SEARCH_DATASET,
    NMessageType.REQUEST_READ_SECRET: NMessageType.REPLY_READ_SECRET,
    NMessageType.REQUEST_RUN_INFERENCE: NMessageType.REPLY_RUN_INFERENCE,
    NMessageType.REQUEST_RUN: NMessageType.REPLY_RUN,
    NMessageType.REQUEST_CANCEL_RUN: NMessageType.REPLY_CANCEL_RUN,
    NMessageType.REQUEST_LANGSERVER: NMessageType.REPLY_LANGSERVER,
}
REQUEST_BY_REPLY_TYPE = {v: k for k, v in REPLY_BY_REQUEST_TYPE.items()}

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
    default_build_id: Optional[UUID]
    arguments: dict[str, typing.Any]
    block: bool
    tracing_level: int
    trigger_type: ExecutionTriggerType
    trigger_id: Optional[UUID]
    execution_id: Optional[UUID]


class RunErrorType(enum.StrEnum):
    INTERNAL_ERROR = "internal_error"
    NOT_READY = "not_ready"
    INVALID_RUNCONFIG = "invalid_runconfig"
    TIMEOUT = "timeout"
    RUNTIME_ERROR = "runtime_error"


@payload(NMessageType.REPLY_RUN)
class RepRunPayload:
    error: Optional[RunErrorType] = None
    execution_id: Optional[UUID] = None
    execution: Optional[ExecutionFrameData] = None


@payload(NMessageType.REQUEST_CANCEL_RUN)
class ReqCancelRunPayload:
    module_id: UUID
    execution_id: UUID


@payload(NMessageType.REPLY_CANCEL_RUN)
class RepCancelRunPayload:
    success: bool


@payload(NMessageType.EXECUTION_MARKED_DEAD)
class ExecutionMarkedDeadPayload:
    module_id: UUID
    execution_id: UUID


@payload(NMessageType.EXECUTION_CHANGED)
class ExecutionChangedPayload(BatchablePayload):
    module_id: UUID
    frames: list[ExecutionFrameData]

    @staticmethod
    def batch(messages: list["ExecutionChangedPayload"]) -> "ExecutionChangedPayload":
        frames = list(chain.from_iterable(m.frames for m in messages))
        return ExecutionChangedPayload(module_id=messages[0].module_id, frames=frames)


@payload(NMessageType.EXECUTION_SAVED)
class ExecutionSavedPayload(BatchablePayload):
    module_id: UUID
    frames: list[ExecutionFrameData]

    @staticmethod
    def batch(messages: list["ExecutionSavedPayload"]) -> "ExecutionSavedPayload":
        frames = list(chain.from_iterable(m.frames for m in messages))
        return ExecutionSavedPayload(module_id=messages[0].module_id, frames=frames)


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


@payload(NMessageType.REQUEST_SEARCH_DATASET)
class ReqSearchDatasetPayload:
    module_id: UUID
    statement_id: UUID
    backend_id: str
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    after: Optional[list[typing.Any]] = None
    limit: Optional[int] = None
    count: bool = False


@payload(NMessageType.REPLY_SEARCH_DATASET)
class RepSearchDatasetPayload:
    records: list[RecordData]
    total: int
    limit: int
    first_sort_key: Optional[list[typing.Any]] = None
    last_sort_key: Optional[list[typing.Any]] = None


@payload(NMessageType.REQUEST_READ_OBJECT)
class ReqReadObjectPayload:
    objects: list[RemoteObjectData]


@payload(NMessageType.REPLY_READ_OBJECT)
class RepReadObjectPayload:
    get_urls: list[typing.Union[str, None]]


@payload(NMessageType.REQUEST_WRITE_OBJECT)
class ReqWriteObjectPayload:
    objects: list[RemoteObjectData]


@payload(NMessageType.REPLY_WRITE_OBJECT)
class RepWriteObjectPayload:
    post_urls: list[typing.Union[str, None]]


@payload(NMessageType.REQUEST_READ_SECRET)
class ReqReadSecretPayload:
    secrets: list[SecretData]


@payload(NMessageType.REPLY_READ_SECRET)
class RepReadSecretPayload:
    secrets: list[SecretData]


@payload(NMessageType.REQUEST_RUN_INFERENCE)
class ReqRunInferencePayload:
    model_fqn: str
    inputs: typing.Any
    timeout: int


@payload(NMessageType.REPLY_RUN_INFERENCE)
class RepRunInferencePayload:
    output: Optional[typing.Any] = None
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
    ExecutionChangedPayload,
    ExecutionSavedPayload,
    ExecutionMarkedDeadPayload,
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
