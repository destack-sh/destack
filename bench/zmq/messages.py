# increment when making backwards-incompatible changes to messages
import enum
import typing
from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum
from typing import Optional
from uuid import UUID

from bench.language import wire
from bench.zmq import sync

PROTOCOL_VERSION = 1

REGISTERED_MESSAGE_PAYLOADS: dict["ZMessageType", typing.Type] = {}


def _register_payload(message_type: "ZMessageType"):
    def wrapper(cls):
        if message_type in REGISTERED_MESSAGE_PAYLOADS:
            raise RuntimeError(f"message type {message_type} already registered")
        cls = dataclass(cls)
        REGISTERED_MESSAGE_PAYLOADS[message_type] = cls
        return cls

    return wrapper


class ZMessageType(StrEnum):
    """All messages types"""

    # Bench project version content sync
    # API <-> API, API -> Worker, API <-> Internal
    PROJECT_VERSION_CHANGED = "project_version_changed"
    # Internal -> Worker
    MODULE_CHANGED = "module_changed"

    # Bench commands
    # API -> Worker
    REQ_MODULE_BUILD = "req_module_build"
    REP_MODULE_BUILD = "rep_module_build"
    REQ_MODULE_RUN = "req_module_run"
    REP_MODULE_RUN = "rep_module_run"

    # Worker internal communication and orchestration
    # Worker <-> Internal
    WORKER_HEARTBEAT = "worker_heartbeat"
    REQ_WORKER_SHUTDOWN = "req_worker_shutdown"
    REQ_READ_MODULE = "req_read_module"
    REP_READ_MODULE = "rep_read_module"
    REQ_WRITE_MODULE = "req_write_module"
    REP_WRITE_MODULE = "rep_write_module"

    # Bench module runtime state sync
    # API <-> Worker
    REQ_MODULE_RUNTIME = "req_module_runtime"
    REP_MODULE_RUNTIME = "rep_module_runtime"
    MODULE_RUNTIME_CHANGED = "module_runtime_changed"


#
# All messages are just Python dataclasses.
# They are serialized and deserialized in serialize.py with some custom logic
#  to support all the nested Python typing we need (e.g. NamedTuples).
# In the future we may want to use a more formal serialization format,
# but for the time being this is both fast and flexible.
#


@_register_payload(ZMessageType.PROJECT_VERSION_CHANGED)
class ProjectVersionChangedPayload:
    project_version_id: UUID
    mutations: list[sync.ProjectMutation]


@_register_payload(ZMessageType.MODULE_CHANGED)
class ModuleChangedPayload:
    module_id: UUID
    #  :PartialModuleUpdates
    # mutations: list[wire.ModuleMutation]
    module: wire.ModuleData


@_register_payload(ZMessageType.REQ_MODULE_BUILD)
class ReqModuleBuildPayload:
    module_id: UUID
    build_id: Optional[UUID]
    buildable_id: Optional[UUID]


class ModuleBuildErrorType(enum.Enum):
    NOT_READY = "not_ready"
    INVALID_BUILDABLE = "invalid_buildable"


@_register_payload(ZMessageType.REP_MODULE_BUILD)
class RepModuleBuildPayload:
    error: Optional[ModuleBuildErrorType] = None


@_register_payload(ZMessageType.REQ_MODULE_RUN)
class ReqModuleRunPayload:
    module_id: UUID
    runconfig_id: Optional[UUID]
    runnable_id: Optional[UUID]
    build_id: Optional[UUID]
    arguments: dict[str, wire.LiteralValue]
    blocking: bool


class ModuleRunErrorType(enum.Enum):
    NOT_READY = "not_ready"
    INVALID_RUNCONFIG = "invalid_runconfig"
    RUNTIME_ERROR = "runtime_error"


@_register_payload(ZMessageType.REP_MODULE_RUN)
class RepModuleRunPayload:
    error: Optional[ModuleRunErrorType] = None
    execution_id: Optional[UUID] = None
    output: Optional[wire.LiteralValue] = None


@_register_payload(ZMessageType.REQ_READ_MODULE)
class ReqReadModulePayload:
    module_id: UUID


@_register_payload(ZMessageType.REP_READ_MODULE)
class RepReadModulePayload:
    module: wire.ModuleData


@_register_payload(ZMessageType.REQ_WRITE_MODULE)
class ReqWriteModulePayload:
    module_id: UUID
    files: list[wire.FileData]


@_register_payload(ZMessageType.REP_WRITE_MODULE)
class RepWriteModulePayload:
    success: bool


@_register_payload(ZMessageType.REQ_MODULE_RUNTIME)
class ReqModuleRuntimePayload:
    module_id: UUID


@_register_payload(ZMessageType.REP_MODULE_RUNTIME)
class RepModuleRuntimePayload:
    updated_at: datetime
    module: wire.ModuleData
    dependencies: list[wire.ModuleData]
    errors: list[wire.ErrorData]


@_register_payload(ZMessageType.MODULE_RUNTIME_CHANGED)
class ModuleRuntimeChangedPayload:
    module_id: UUID
    updated_at: datetime
    #  :PartialModuleUpdates
    module: wire.ModuleData
    dependencies: list[wire.ModuleData]
    errors: list[wire.ErrorData]


# invert REGISTERED_MESSAGE_PAYLOADS
MESSAGE_TYPE_BY_PAYLOAD_CLASS: dict[typing.Type, "ZMessageType"] = {
    payload_class: message_type
    for message_type, payload_class in REGISTERED_MESSAGE_PAYLOADS.items()
}
