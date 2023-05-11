# increment when making backwards-incompatible changes to messages
import abc
import enum
import typing
from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum
from itertools import chain
from typing import Optional, cast
from uuid import UUID

from bench.language import mutate, wire
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType, RemoteObjectData
from bench.runtime.type import BuildScope, EvaluationResultData, ExecutionFrameData, JobData

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
    MODULE_CHANGED = "module.changed"  # for API
    MODULE_INTERNAL_CHANGED = "module.internal.changed"  # for internal

    # Worker <-> Internal
    REQUEST_REGISTER_WORKER = "worker.register"
    REPLY_REGISTER_WORKER = "worker.register.rep"
    WORKER_HEARTBEAT = "worker.heartbeat"
    REQUEST_READ_MODULE = "module.read"
    REPLY_READ_MODULE = "module.read.rep"
    REQUEST_WRITE_MODULE = "module.write"
    REPLY_WRITE_MODULE = "module.write.rep"
    REQUEST_READ_OBJECT = "object.read"
    REPLY_READ_OBJECT = "object.read.rep"
    REQUEST_WRITE_OBJECT = "object.write"
    REPLY_WRITE_OBJECT = "object.write.rep"
    EXECUTION_CHANGED = "execution.changed"
    EXECUTION_SAVED = "execution.saved"
    JOB_SAVED = "job.saved"
    EVALUATION_SAVED = "evaluation.saved"

    # API <-> Worker
    REQUEST_BUILD = "build"
    REPLY_BUILD = "build.rep"
    REQUEST_RUN = "run"
    REPLY_RUN = "run.rep"
    REQUEST_GENERATE = "generate"
    REPLY_GENERATE = "generate.rep"
    REQUEST_INTERP = "interp.get"
    REPLY_INTERP = "interp.get.rep"
    INTERP_CHANGED = "interp.changed"


REPLY_BY_REQUEST_TYPE = {
    NMessageType.REQUEST_REGISTER_WORKER: NMessageType.REPLY_REGISTER_WORKER,
    NMessageType.REQUEST_READ_MODULE: NMessageType.REPLY_READ_MODULE,
    NMessageType.REQUEST_WRITE_MODULE: NMessageType.REPLY_WRITE_MODULE,
    NMessageType.REQUEST_READ_OBJECT: NMessageType.REPLY_READ_OBJECT,
    NMessageType.REQUEST_WRITE_OBJECT: NMessageType.REPLY_WRITE_OBJECT,
    NMessageType.REQUEST_BUILD: NMessageType.REPLY_BUILD,
    NMessageType.REQUEST_RUN: NMessageType.REPLY_RUN,
    NMessageType.REQUEST_INTERP: NMessageType.REPLY_INTERP,
}
REQUEST_BY_REPLY_TYPE = {v: k for k, v in REPLY_BY_REQUEST_TYPE.items()}

#
# All messages are just Python dataclasses.
# They are serialized and deserialized in serialize.py with some custom logic
#  to support all the nested Python typing we need (e.g. NamedTuples).
# In the future we may want to use a more formal serialization format,
# but for the time being this is both fast and flexible.
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


@payload(NMessageType.CLIENT_CHANGED)
class ClientChangedPayload:
    client: ClientOrigin


@payload(NMessageType.MODULE_CHANGED)
class ModuleChangedPayload(OriginPayload):
    module_id: UUID
    mutations: list[mutate.ModuleMutation]


@payload(NMessageType.MODULE_INTERNAL_CHANGED)
class ModuleInternalChangedPayload(OriginPayload):
    module_id: UUID
    mutations: list[mutate.ModuleMutation]


@payload(NMessageType.REQUEST_REGISTER_WORKER)
class ReqRegisterWorkerPayload:
    worker_id: UUID
    deployment_id: UUID
    project_id: Optional[UUID]
    type: str


@payload(NMessageType.REPLY_REGISTER_WORKER)
class RepRegisterWorkerPayload:
    success: bool


@payload(NMessageType.WORKER_HEARTBEAT)
class WorkerHeartbeatPayload:
    worker_id: UUID


@payload(NMessageType.REQUEST_BUILD)
class ReqBuildPayload:
    module_id: UUID
    scope: BuildScope
    buildable_id: Optional[UUID]


class BuildErrorType(enum.StrEnum):
    NOT_READY = "not_ready"
    INVALID_BUILDABLE = "invalid_buildable"
    COMMITTED = "committed"


@payload(NMessageType.REPLY_BUILD)
class RepBuildPayload:
    error: Optional[BuildErrorType] = None


@payload(NMessageType.REQUEST_RUN)
class ReqRunPayload:
    deployment_id: UUID
    module_id: UUID
    runnable: Optional[UUID | str]
    runnable_type: Optional[str]
    build: Optional[UUID | str]
    arguments: dict[str, typing.Any]
    block: bool
    tracing_level: ExecutionTracingLevel
    trigger_type: ExecutionTriggerType
    trigger_id: Optional[UUID]


class RunErrorType(enum.StrEnum):
    INTERNAL_ERROR = "internal_error"
    NOT_READY = "not_ready"
    INVALID_RUNCONFIG = "invalid_runconfig"
    TIMEOUT = "timeout"
    RUNTIME_ERROR = "runtime_error"


@payload(NMessageType.REPLY_RUN)
class RepRunPayload:
    error: Optional[RunErrorType] = None
    execution: Optional[ExecutionFrameData] = None


@payload(NMessageType.REQUEST_GENERATE)
class ReqGeneratePayload:
    module_id: UUID
    generatable: Optional[UUID | str]


class GenerateErrorType(enum.StrEnum):
    INTERNAL_ERROR = "internal_error"
    NOT_READY = "not_ready"
    INVALID_GENERATABLE = "invalid_generatable"
    TIMEOUT = "timeout"
    RUNTIME_ERROR = "runtime_error"


@payload(NMessageType.REPLY_GENERATE)
class RepGeneratePayload:
    output: Optional[typing.Any] = None
    error: Optional[GenerateErrorType] = None
    error_details: Optional[typing.Any] = None


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


@payload(NMessageType.EVALUATION_SAVED)
class EvaluationSavedPayload(BatchablePayload):
    module_id: UUID
    evaluations: list[EvaluationResultData]

    @staticmethod
    def batch(messages: list["EvaluationSavedPayload"]) -> "EvaluationSavedPayload":
        evaluations = list(chain.from_iterable(m.evaluations for m in messages))
        return EvaluationSavedPayload(module_id=messages[0].module_id, evaluations=evaluations)


@payload(NMessageType.JOB_SAVED)
class JobSavedPayload(BatchablePayload):
    module_id: UUID
    jobs: list[JobData]

    @staticmethod
    def batch(messages: list["JobSavedPayload"]) -> "JobSavedPayload":
        jobs = list(chain.from_iterable(m.jobs for m in messages))
        return JobSavedPayload(module_id=messages[0].module_id, jobs=jobs)


@payload(NMessageType.REQUEST_READ_MODULE)
class ReqReadModulePayload:
    module_id: UUID


@payload(NMessageType.REPLY_READ_MODULE)
class RepReadModulePayload:
    module: wire.ModuleData
    project_id: UUID


@payload(NMessageType.REQUEST_WRITE_MODULE)
class ReqWriteModulePayload:
    module_id: UUID
    mutations: list[mutate.ModuleMutation]
    client: ClientOrigin


@payload(NMessageType.REPLY_WRITE_MODULE)
class RepWriteModulePayload:
    success: bool


@payload(NMessageType.REQUEST_READ_OBJECT)
class ReqReadObjectPayload:
    objects: list[RemoteObjectData]


@payload(NMessageType.REPLY_READ_OBJECT)
class RepReadObjectPayload:
    get_urls: list[typing.Union[str, None]]


@payload(NMessageType.REQUEST_WRITE_OBJECT)
class ReqWriteObjectPayload:
    objects: [RemoteObjectData]


@payload(NMessageType.REPLY_WRITE_OBJECT)
class RepWriteObjectPayload:
    post_urls: list[typing.Union[str, None]]


@payload(NMessageType.REQUEST_INTERP)
class ReqInterpPayload:
    module_id: UUID


@payload(NMessageType.REPLY_INTERP)
class RepInterpPayload:
    module_id: UUID
    updated_at: datetime
    module: wire.ModuleData
    dependencies: list[wire.ModuleData]
    errors: list[wire.ErrorData]
    stale_symbols: list[UUID]
    builds_by_symbol: Optional[dict[UUID, list[UUID]]]


@payload(NMessageType.INTERP_CHANGED)
class InterpChangedPayload:
    # unfortunately full data :PartialModuleUpdates
    module_id: UUID
    updated_at: datetime
    module: Optional[wire.ModuleData | None]
    dependencies: Optional[list[wire.ModuleData]]
    errors: Optional[list[wire.ErrorData]]
    stale_symbols: Optional[list[UUID]]
    builds_by_symbol: Optional[dict[UUID, list[UUID]]]


# invert REGISTERED_MESSAGE_PAYLOADS
MESSAGE_TYPE_BY_PAYLOAD_CLASS: dict[typing.Type, "NMessageType"] = {
    payload_class: message_type
    for message_type, payload_class in REGISTERED_MESSAGE_PAYLOADS.items()
}


def to_topic(
    message_type: NMessageType,
    payload: NMessageType,
) -> str:
    """
    Gets the default topic for a message type and payload.
    :NATSTopics
    """
    # note: this seems a tad repetitive, maybe cleanup somehow (sacrifice type safety?)
    if message_type == NMessageType.MODULE_CHANGED:
        payload = cast(ModuleChangedPayload, payload)
        return f"{message_type}.{payload.module_id}"
    elif message_type == NMessageType.MODULE_INTERNAL_CHANGED:
        payload = cast(ModuleInternalChangedPayload, payload)
        return f"{message_type}.{payload.module_id}"
    elif message_type == NMessageType.INTERP_CHANGED:
        payload = cast(InterpChangedPayload, payload)
        return f"{message_type}.{payload.module_id}"
    elif message_type == NMessageType.EXECUTION_CHANGED:
        payload = cast(ExecutionChangedPayload, payload)
        return f"{message_type}.{payload.module_id}"
    elif message_type == NMessageType.EXECUTION_SAVED:
        payload = cast(ExecutionSavedPayload, payload)
        return f"{message_type}.{payload.module_id}"
    elif message_type == NMessageType.JOB_SAVED:
        payload = cast(JobSavedPayload, payload)
        return f"{message_type}.{payload.module_id}"
    elif message_type == NMessageType.EVALUATION_SAVED:
        payload = cast(EvaluationSavedPayload, payload)
        return f"{message_type}.{payload.module_id}"

    return message_type
