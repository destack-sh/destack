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
from bench.runtime.type import ExecutionFrameData

PROTOCOL_VERSION = 1

REGISTERED_MESSAGE_PAYLOADS: dict["NMessageType", typing.Type] = {}


def _register_payload(message_type: "NMessageType"):
    def wrapper(cls):
        if message_type in REGISTERED_MESSAGE_PAYLOADS:
            raise RuntimeError(f"message type {message_type} already registered")
        cls = dataclass(cls)
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
    REQUEST_READ_MODULE = "module.read"
    REPLY_READ_MODULE = "module.read.rep"
    REQUEST_WRITE_MODULE = "module.write"
    REPLY_WRITE_MODULE = "module.write.rep"
    EXECUTION_CHANGED = "execution.changed"
    EXECUTION_SAVED = "execution.saved"

    # API <-> Worker
    REQUEST_MODULE_BUILD = "runtime.build"
    REPLY_MODULE_BUILD = "runtime.build.rep"
    REQUEST_MODULE_RUN = "runtime.run"
    REPLY_MODULE_RUN = "runtime.run.rep"
    REQUEST_MODULE_RUNTIME = "runtime.get"
    REPLY_MODULE_RUNTIME = "runtime.get.rep"
    MODULE_RUNTIME_CHANGED = "runtime.changed"


REPLY_BY_REQUEST_TYPE = {
    NMessageType.REQUEST_READ_MODULE: NMessageType.REPLY_READ_MODULE,
    NMessageType.REQUEST_WRITE_MODULE: NMessageType.REPLY_WRITE_MODULE,
    NMessageType.REQUEST_MODULE_BUILD: NMessageType.REPLY_MODULE_BUILD,
    NMessageType.REQUEST_MODULE_RUN: NMessageType.REPLY_MODULE_RUN,
    NMessageType.REQUEST_MODULE_RUNTIME: NMessageType.REPLY_MODULE_RUNTIME,
}
REQUEST_BY_REPLY_TYPE = {v: k for k, v in REPLY_BY_REQUEST_TYPE.items()}

#
# All messages are just Python dataclasses.
# They are serialized and deserialized in serialize.py with some custom logic
#  to support all the nested Python typing we need (e.g. NamedTuples).
# In the future we may want to use a more formal serialization format,
# but for the time being this is both fast and flexible.
#


@_register_payload(NMessageType.PROJECT_VERSION_CHANGED)
class ProjectVersionChangedPayload:
    project_version_id: UUID
    mutations: list[sync.ProjectMutation]


@_register_payload(NMessageType.MODULE_CHANGED)
class ModuleChangedPayload:
    module_id: UUID
    #  :PartialModuleUpdates
    # mutations: list[wire.ModuleMutation]
    module: wire.ModuleData


@_register_payload(NMessageType.REQUEST_MODULE_BUILD)
class ReqModuleBuildPayload:
    module_id: UUID
    buildable_id: Optional[UUID]


class ModuleBuildErrorType(enum.Enum):
    NOT_READY = "not_ready"
    INVALID_BUILDABLE = "invalid_buildable"


@_register_payload(NMessageType.REPLY_MODULE_BUILD)
class RepModuleBuildPayload:
    error: Optional[ModuleBuildErrorType] = None


@_register_payload(NMessageType.REQUEST_MODULE_RUN)
class ReqModuleRunPayload:
    deployment_id: UUID
    module_id: UUID
    runnable: Optional[UUID | str]
    runnable_type: Optional[str]
    build: Optional[UUID | str]
    arguments: dict[str, wire.LiteralValue]
    blocking: bool
    tracing_level: ExecutionTracingLevel
    trigger_type: ExecutionTriggerType
    trigger_id: Optional[UUID]


class ModuleRunErrorType(enum.Enum):
    INTERNAL_ERROR = "internal_error"
    NOT_READY = "not_ready"
    INVALID_RUNCONFIG = "invalid_runconfig"
    RUNTIME_ERROR = "runtime_error"


@_register_payload(NMessageType.REPLY_MODULE_RUN)
class RepModuleRunPayload:
    execution_id: Optional[UUID] = None
    error: Optional[ModuleRunErrorType] = None
    error_details: Optional[typing.Any] = None
    output: Optional[wire.LiteralValue] = None


@_register_payload(NMessageType.EXECUTION_CHANGED)
class ExecutionChangedPayload:
    module_id: UUID
    frames: list[ExecutionFrameData]


@_register_payload(NMessageType.EXECUTION_SAVED)
class ExecutionSavedPayload:
    module_id: UUID
    frames: list[ExecutionFrameData]


@_register_payload(NMessageType.REQUEST_READ_MODULE)
class ReqReadModulePayload:
    module_id: UUID


@_register_payload(NMessageType.REPLY_READ_MODULE)
class RepReadModulePayload:
    module: wire.ModuleData
    project_id: UUID


@_register_payload(NMessageType.REQUEST_WRITE_MODULE)
class ReqWriteModulePayload:
    module_id: UUID
    files: list[wire.FileData]
    generated_mappings: list[tuple[UUID, list[wire.GeneratedMapping]]]


@_register_payload(NMessageType.REPLY_WRITE_MODULE)
class RepWriteModulePayload:
    success: bool


@_register_payload(NMessageType.REQUEST_MODULE_RUNTIME)
class ReqModuleRuntimePayload:
    module_id: UUID


@_register_payload(NMessageType.REPLY_MODULE_RUNTIME)
class RepModuleRuntimePayload:
    module_id: UUID
    updated_at: datetime
    module: wire.ModuleData
    dependencies: list[wire.ModuleData]
    errors: list[wire.ErrorData]
    jobs: list[wire.JobData]
    stale_symbols: list[UUID]


@_register_payload(NMessageType.MODULE_RUNTIME_CHANGED)
class ModuleRuntimeChangedPayload:
    # unfortunately full data :PartialModuleUpdates
    module_id: UUID
    updated_at: datetime
    module: Optional[wire.ModuleData | None]
    dependencies: Optional[list[wire.ModuleData]]
    errors: Optional[list[wire.ErrorData]]
    jobs: Optional[list[wire.JobData]]
    stale_symbols: Optional[list[UUID]]


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
    if message_type == NMessageType.PROJECT_VERSION_CHANGED:
        payload = cast(ProjectVersionChangedPayload, payload)
        return f"{message_type}.{payload.project_version_id}"
    elif message_type == NMessageType.MODULE_CHANGED:
        payload = cast(ModuleChangedPayload, payload)
        return f"{message_type}.{payload.module_id}"
    elif message_type == NMessageType.MODULE_RUNTIME_CHANGED:
        payload = cast(ModuleRuntimeChangedPayload, payload)
        return f"{message_type}.{payload.module_id}"
    elif message_type == NMessageType.EXECUTION_CHANGED:
        payload = cast(ExecutionChangedPayload, payload)
        return f"{message_type}.{payload.module_id}"

    return message_type
