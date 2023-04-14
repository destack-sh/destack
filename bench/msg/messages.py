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
from bench.runtime.type import BuildCandidateData, EvaluationResultData, ExecutionFrameData, JobData

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
    REQUEST_WRITE_MODULE = "module.write"
    REPLY_WRITE_MODULE = "module.write.rep"
    EXECUTION_CHANGED = "execution.changed"
    EXECUTION_SAVED = "execution.saved"
    REQUEST_WRITE_JOB = "job.write"
    REPLY_WRITE_JOB = "job.write.rep"
    JOB_SAVED = "job.saved"
    REQUEST_WRITE_EVALUATION = "evaluation.write"
    REPLY_WRITE_EVALUATION = "evaluation.write.rep"
    EVALUATION_SAVED = "evaluation.saved"
    REQUEST_WRITE_BUILD_CANDIDATE = "build.candidate.write"
    REPLY_WRITE_BUILD_CANDIDATE = "build.candidate.write.rep"
    BUILD_CANDIDATE_SAVED = "build.candidate.saved"
    REQUEST_WRITE_BUILD = "build.result.write"
    REPLY_WRITE_BUILD = "build.result.write.rep"

    # API <-> Worker
    REQUEST_MODULE_BUILD = "runtime.build"
    REPLY_MODULE_BUILD = "runtime.build.rep"
    REQUEST_MODULE_RUN = "runtime.run"
    REPLY_MODULE_RUN = "runtime.run.rep"
    REQUEST_INTERP_MODULE = "interp.get"
    REPLY_INTERP_MODULE = "interp.get.rep"
    INTERP_MODULE_CHANGED = "interp.changed"


REPLY_BY_REQUEST_TYPE = {
    NMessageType.REQUEST_REGISTER_WORKER: NMessageType.REPLY_REGISTER_WORKER,
    NMessageType.REQUEST_READ_MODULE: NMessageType.REPLY_READ_MODULE,
    NMessageType.REQUEST_WRITE_MODULE: NMessageType.REPLY_WRITE_MODULE,
    NMessageType.REQUEST_WRITE_BUILD: NMessageType.REPLY_WRITE_BUILD,
    NMessageType.REQUEST_WRITE_BUILD_CANDIDATE: NMessageType.REPLY_WRITE_BUILD_CANDIDATE,
    NMessageType.REQUEST_WRITE_EVALUATION: NMessageType.REPLY_WRITE_EVALUATION,
    NMessageType.REQUEST_WRITE_JOB: NMessageType.REPLY_WRITE_JOB,
    NMessageType.REQUEST_MODULE_BUILD: NMessageType.REPLY_MODULE_BUILD,
    NMessageType.REQUEST_MODULE_RUN: NMessageType.REPLY_MODULE_RUN,
    NMessageType.REQUEST_INTERP_MODULE: NMessageType.REPLY_INTERP_MODULE,
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
    mutations: list[sync.ProjectMutation]


@payload(NMessageType.MODULE_CHANGED)
class ModuleChangedPayload:
    module_id: UUID
    #  :PartialModuleUpdates
    # mutations: list[wire.ModuleMutation]
    module: wire.ModuleData


@payload(NMessageType.REQUEST_MODULE_BUILD)
class ReqModuleBuildPayload:
    module_id: UUID
    buildable_id: Optional[UUID]


class ModuleBuildErrorType(enum.Enum):
    NOT_READY = "not_ready"
    INVALID_BUILDABLE = "invalid_buildable"


@payload(NMessageType.REPLY_MODULE_BUILD)
class RepModuleBuildPayload:
    error: Optional[ModuleBuildErrorType] = None


@payload(NMessageType.REQUEST_MODULE_RUN)
class ReqModuleRunPayload:
    deployment_id: UUID
    module_id: UUID
    runnable: Optional[UUID | str]
    runnable_type: Optional[str]
    build: Optional[UUID | str]
    arguments: dict[str, wire.LiteralValue]
    block: bool
    tracing_level: ExecutionTracingLevel
    trigger_type: ExecutionTriggerType
    trigger_id: Optional[UUID]


class ModuleRunErrorType(enum.Enum):
    INTERNAL_ERROR = "internal_error"
    NOT_READY = "not_ready"
    INVALID_RUNCONFIG = "invalid_runconfig"
    TIMEOUT = "timeout"
    RUNTIME_ERROR = "runtime_error"


@payload(NMessageType.REPLY_MODULE_RUN)
class RepModuleRunPayload:
    execution_id: Optional[UUID] = None
    error: Optional[ModuleRunErrorType] = None
    error_details: Optional[typing.Any] = None
    output: Optional[wire.LiteralValue] = None


@payload(NMessageType.EXECUTION_CHANGED)
class ExecutionChangedPayload:
    module_id: UUID
    frames: list[ExecutionFrameData]


@payload(NMessageType.EXECUTION_SAVED)
class ExecutionSavedPayload:
    module_id: UUID
    frames: list[ExecutionFrameData]


@payload(NMessageType.REQUEST_WRITE_EVALUATION)
class ReqWriteEvaluationPayload:
    module_id: UUID
    evaluations: list[EvaluationResultData]


@payload(NMessageType.REPLY_WRITE_EVALUATION)
class RepWriteEvaluationPayload:
    success: bool


@payload(NMessageType.EVALUATION_SAVED)
class EvaluationSavedPayload:
    module_id: UUID
    evaluations: list[EvaluationResultData]


@payload(NMessageType.REQUEST_WRITE_BUILD_CANDIDATE)
class ReqWriteBuildCandidatePayload:
    module_id: UUID
    build_id: UUID
    build_candidates: list[BuildCandidateData]
    delete_others: bool


@payload(NMessageType.REPLY_WRITE_BUILD_CANDIDATE)
class RepWriteBuildCandidatePayload:
    success: bool


@payload(NMessageType.BUILD_CANDIDATE_SAVED)
class BuildCandidateSavedPayload:
    module_id: UUID
    build_candidates: list[BuildCandidateData]


@payload(NMessageType.REQUEST_WRITE_BUILD)
class ReqWriteBuildPayload:
    module_id: UUID
    build_ids: list[UUID]
    delete_files: list[UUID]
    files: list[wire.FileData]
    generated_mappings: list[tuple[UUID, list[wire.GeneratedMapping]]]


@payload(NMessageType.REPLY_WRITE_BUILD)
class RepWriteBuildPayload:
    success: bool


@payload(NMessageType.REQUEST_WRITE_JOB)
class ReqWriteJobPayload:
    module_id: UUID
    job: JobData


@payload(NMessageType.REPLY_WRITE_JOB)
class RepWriteJobPayload:
    success: bool


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


@payload(NMessageType.REQUEST_WRITE_MODULE)
class ReqWriteModulePayload:
    module_id: UUID
    generated_mappings: list[tuple[UUID, list[wire.GeneratedMapping]]]
    files: list[wire.FileData]


@payload(NMessageType.REPLY_WRITE_MODULE)
class RepWriteModulePayload:
    success: bool


@payload(NMessageType.REQUEST_INTERP_MODULE)
class ReqInterpModulePayload:
    module_id: UUID


@payload(NMessageType.REPLY_INTERP_MODULE)
class RepInterpModulePayload:
    module_id: UUID
    updated_at: datetime
    module: wire.ModuleData
    dependencies: list[wire.ModuleData]
    errors: list[wire.ErrorData]
    stale_symbols: list[UUID]
    builds_by_symbol: Optional[dict[UUID, list[UUID]]]


@payload(NMessageType.INTERP_MODULE_CHANGED)
class InterpModuleChangedPayload:
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
    elif message_type == NMessageType.INTERP_MODULE_CHANGED:
        payload = cast(InterpModuleChangedPayload, payload)
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
    elif message_type == NMessageType.BUILD_CANDIDATE_SAVED:
        payload = cast(BuildCandidateSavedPayload, payload)
        return f"{message_type}.{payload.module_id}"

    return message_type
