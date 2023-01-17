import dataclasses
import uuid
from dataclasses import dataclass
from enum import StrEnum
from typing import Any
from uuid import UUID

import structlog
import zmq.asyncio

logger = structlog.get_logger(__name__)

# increment when making backwards-incompatible changes to messages
PROTOCOL_VERSION = 1


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


@dataclass(repr=False)
class ZMessage:
    _type: ZMessageType
    _id: UUID = dataclasses.field(default_factory=uuid.uuid4)
    _version: int = PROTOCOL_VERSION
    # TODO @Performance: expose & use zmq envelope key to filter in zmq

    def __str__(self):
        return f"{self._type} {self._id}"


def serialize_message(message: ZMessage) -> dict:
    # serialize any dataclass as something jsonable
    return dataclasses.asdict(message)


def parse_message(message_json: dict) -> ZMessage:
    raise NotImplementedError  # nocheckin
    # TODO @Incomplete: error if version != PROTOCOL_VERSION


def send_message(sock, message: ZMessage):
    sock.send_json(serialize_message(message))
    logger.debug("send_message", msg=message)


async def recv_message(sock) -> ZMessage:
    msg = parse_message(await sock.recv_json())
    logger.debug("recv_message", msg=msg)
    return msg


zmq_ctx_sync = zmq.Context()
zmq_ctx = zmq.asyncio.Context(shadow=zmq_ctx_sync)

# TODO @Incomplete: close zmq_ctx_sync/zmq_ctx on exit
#  (and ensure all sockets and connections are closed)
