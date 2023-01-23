# increment when making backwards-incompatible changes to messages
import typing
from dataclasses import dataclass
from enum import StrEnum
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
    REQ_WORKER_COMPILE = "req_worker_compile"
    REQ_WORKER_RUN = "req_worker_run"

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


# TODO @Performance @Robustness: use custom message types & format beyond JSON?


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


@_register_payload(ZMessageType.REQ_READ_MODULE)
class ReqReadModulePayload:
    module_id: UUID


@_register_payload(ZMessageType.REP_READ_MODULE)
class RepReadModulePayload:
    module: wire.ModuleData


@_register_payload(ZMessageType.REQ_MODULE_RUNTIME)
class ReqModuleRuntimePayload:
    module_id: UUID


@_register_payload(ZMessageType.REP_MODULE_RUNTIME)
class RepModuleRuntimePayload:
    module: wire.ModuleData
    dependencies: list[wire.ModuleData]
    errors: list[wire.ErrorData]


@_register_payload(ZMessageType.MODULE_RUNTIME_CHANGED)
class ModuleRuntimeChangedPayload:
    module_id: UUID
    #  :PartialModuleUpdates
    module: wire.ModuleData
    dependencies: list[wire.ModuleData]
    errors: list[wire.ErrorData]


# invert REGISTERED_MESSAGE_PAYLOADS
MESSAGE_TYPE_BY_PAYLOAD_CLASS: dict[typing.Type, "ZMessageType"] = {
    payload_class: message_type
    for message_type, payload_class in REGISTERED_MESSAGE_PAYLOADS.items()
}
