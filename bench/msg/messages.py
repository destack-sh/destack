# increment when making backwards-incompatible changes to messages
import enum
import typing
from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum
from typing import Optional, cast
from uuid import UUID

from bench.language import wire
from bench.language.wire import ExecutionTracingLevel, ExecutionTriggerType
from bench.msg import sync
from bench.runtime.type import BuildScope, EvaluationResultData, ExecutionFrameData, JobData

PROTOCOL_VERSION = 1

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

    # Bench project version content sync
    # API <-> API,, API <-> Internal
    PROJECT_VERSION_CHANGED = "project_version.changed"
    # Internal -> Worker
    MODULE_CHANGED = "module.changed"

    # Worker <-> Internal
    REQUEST_REGISTER_WORKER = "worker.register"
    REPLY_REGISTER_WORKER = "worker.register.rep"
    WORKER_HEARTBEAT = "worker.heartbeat"
    REQUEST_READ_MODULE = "module.read"
    REPLY_READ_MODULE = "module.read.rep"
    REPLY_WRITE_MODULE = "module.write.rep"
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


@payload(NMessageType.PROJECT_VERSION_CHANGED)
class ProjectVersionChangedPayload:
    project_version_id: UUID
    client_id: UUID
    mutations: list[sync.ProjectMutation]


@payload(NMessageType.MODULE_CHANGED)
class ModuleChangedPayload:
    module_id: UUID
    #  :PartialModuleUpdates
    # mutations: list[wire.ModuleMutation]
    module: wire.ModuleData


@payload(NMessageType.REQUEST_BUILD)
class ReqBuildPayload:
    module_id: UUID
    scope: BuildScope
    buildable_id: Optional[UUID]


class BuildErrorType(enum.Enum):
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


class RunErrorType(enum.Enum):
    INTERNAL_ERROR = "internal_error"
    NOT_READY = "not_ready"
    INVALID_RUNCONFIG = "invalid_runconfig"
    TIMEOUT = "timeout"
    RUNTIME_ERROR = "runtime_error"


@payload(NMessageType.REPLY_RUN)
class RepRunPayload:
    execution_id: Optional[UUID] = None
    error: Optional[RunErrorType] = None
    error_details: Optional[typing.Any] = None
    output: Optional[typing.Any] = None


@payload(NMessageType.REQUEST_GENERATE)
class ReqGeneratePayload:
    module_id: UUID
    generatable: Optional[UUID | str]


class GenerateErrorType(enum.Enum):
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
class ExecutionChangedPayload:
    module_id: UUID
    frames: list[ExecutionFrameData]


@payload(NMessageType.EXECUTION_SAVED)
class ExecutionSavedPayload:
    module_id: UUID
    frames: list[ExecutionFrameData]


@payload(NMessageType.EVALUATION_SAVED)
class EvaluationSavedPayload:
    module_id: UUID
    evaluations: list[EvaluationResultData]


@payload(NMessageType.JOB_SAVED)
class JobSavedPayload:
    module_id: UUID
    job: JobData


@payload(NMessageType.REQUEST_READ_MODULE)
class ReqReadModulePayload:
    module_id: UUID


@payload(NMessageType.REPLY_READ_MODULE)
class RepReadModulePayload:
    module: wire.ModuleData
    project_id: UUID


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
    if message_type == NMessageType.PROJECT_VERSION_CHANGED:
        payload = cast(ProjectVersionChangedPayload, payload)
        return f"{message_type}.{payload.project_version_id}"
    elif message_type == NMessageType.MODULE_CHANGED:
        payload = cast(ModuleChangedPayload, payload)
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
