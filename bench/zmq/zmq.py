import dataclasses
import json
import typing
import uuid
from dataclasses import dataclass
from datetime import datetime
from enum import Enum
from uuid import UUID

import structlog
import zmq.asyncio
from dacite import Config, from_dict

from bench.zmq.messages import PROTOCOL_VERSION, REGISTERED_MESSAGE_PAYLOADS, ZMessageType

logger = structlog.get_logger(__name__)


PayloadT = typing.TypeVar("PayloadT", bound=typing.Any)


@dataclass(repr=False)
class ZMessage:
    type: ZMessageType
    payload: typing.Any = None
    _id: UUID = dataclasses.field(default_factory=uuid.uuid4)
    _version: int = PROTOCOL_VERSION
    # TODO @Performance: expose & use zmq envelope key to filter in zmq

    def __str__(self):
        return f"{self.type} {self._id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    def payload_as(self, cls: typing.Type[PayloadT]) -> PayloadT:
        if not isinstance(self.payload, cls):
            raise TypeError(f"expected payload to be {cls}, got {type(self.payload)}")
        return self.payload


def serialize_message(message: ZMessage) -> str:
    # serialize any dataclass as something jsonable
    message_dict = dataclasses.asdict(message)
    return json.dumps(message_dict, cls=MessageJSONEncoder)


def parse_message(message_json: str) -> ZMessage:
    message_dict = json.loads(message_json)
    if message_dict["_version"] != PROTOCOL_VERSION:
        raise RuntimeError(
            f"message version mismatch: {message_dict['_version']} != {PROTOCOL_VERSION}"
        )

    dacite_config = Config(cast=[UUID, datetime, Enum])
    payload_cls = REGISTERED_MESSAGE_PAYLOADS.get(message_dict["type"])
    if payload_cls:
        message_dict["payload"] = from_dict(payload_cls, message_dict["payload"], dacite_config)
    return from_dict(ZMessage, message_dict, config=dacite_config)


def send_message(sock: zmq.Socket, message: ZMessage):
    sock.send_string(serialize_message(message))
    logger.debug("send_message", msg=message)


async def recv_message(sock: zmq.asyncio.Socket) -> ZMessage:
    msg = parse_message(await sock.recv_string())
    logger.debug("recv_message", msg=msg)
    return msg


async def recv_message_poll(poller: zmq.asyncio.Poller, timeout: int | None = None) -> ZMessage:
    events = dict(await poller.poll(timeout))
    pollin_events = {sock: msg for sock, msg in events.items() if msg & zmq.POLLIN}
    if len(pollin_events) != 1:
        # TODO @Robustness: handle multiple simultaneous zmq pollin messages
        raise RuntimeError("expected exactly one event on poller")
    for sock, event in pollin_events.items():
        return await recv_message(sock)


zmq_ctx_sync = zmq.Context()
zmq_ctx = zmq.asyncio.Context(shadow=zmq_ctx_sync)

# TODO @Incomplete: close zmq_ctx_sync/zmq_ctx on exit
#  (and ensure all sockets and connections are closed)


class MessageJSONEncoder(json.JSONEncoder):
    """
    JSONEncoder subclass that knows how to encode datetimes and UUIDs.
    Trimmed down and customised DjangoJSONEncoder without the dependency.
    """

    def default(self, o):
        # See "Date Time String Format" in the ECMA-262 specification.
        if isinstance(o, datetime):
            r = o.isoformat()
            if o.microsecond:
                r = r[:23] + r[26:]
            if r.endswith("+00:00"):
                r = r[:-6] + "Z"
            return r
        elif isinstance(o, UUID):
            return str(o)
        else:
            return super().default(o)
