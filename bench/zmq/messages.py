# increment when making backwards-incompatible changes to messages
import typing
from dataclasses import dataclass
from enum import StrEnum
from uuid import UUID

from bench import language

PROTOCOL_VERSION = 1
REGISTERED_MESSAGE_PAYLOADS: dict["ZMessageType", typing.Type] = {}


def register_payload(message_type: "ZMessageType"):
    def wrapper(cls):
        if message_type in REGISTERED_MESSAGE_PAYLOADS:
            raise RuntimeError(f"message type {message_type} already registered")
        cls = dataclass(cls)
        REGISTERED_MESSAGE_PAYLOADS[message_type] = cls
        return cls

    return wrapper


class ZMessageType(StrEnum):
    """All messages types"""

    # Bench project version sync
    # API <-> API, API -> Worker, API <-> Internal
    PROJECT_VERSION_CHANGED = "project_version_changed"
    # Internal -> Worker
    MODULE_CHANGED = "module_changed"

    # Worker internal communication and orchestration
    # Worker <-> Internal
    WORKER_HEARTBEAT = "worker_heartbeat"
    REQ_READ_MODULE = "req_read_module"
    REP_READ_MODULE = "rep_read_module"
    REQ_WRITE_MODULE = "req_write_module"
    REP_WRITE_MODULE = "rep_write_module"

    # Bench module runtime state sync
    # API <-> Worker
    REQ_MODULE_STATE = "req_module_state"
    REP_MODULE_STATE = "rep_module_state"
    MODULE_STATE_CHANGED = "module_state_changed"


@register_payload(ZMessageType.REQ_READ_MODULE)
class ReqReadModulePayload:
    module_id: UUID


@register_payload(ZMessageType.REP_READ_MODULE)
class RepReadModulePayload:
    module: language.Module


@register_payload(ZMessageType.REQ_MODULE_STATE)
class ReqModuleStatePayload:
    module_id: UUID


@register_payload(ZMessageType.REP_MODULE_STATE)
class RepModuleStatePayload:
    module_id: UUID
    all_symbols: list[language.Statement]
    errors: list[language.Error]
