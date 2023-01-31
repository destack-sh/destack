import dataclasses
import json
import typing
import uuid
from dataclasses import dataclass
from datetime import datetime
from uuid import UUID

import structlog
import zmq.asyncio

from bench.zmq.messages import (
    MESSAGE_TYPE_BY_PAYLOAD_CLASS,
    PROTOCOL_VERSION,
    REGISTERED_MESSAGE_PAYLOADS,
    ZMessageType,
)
from bench.zmq.serialize import from_dict, to_dict

logger = structlog.get_logger(__name__)


PayloadT = typing.TypeVar("PayloadT", bound=typing.Any)


@dataclass(repr=False)
class ZMessage:
    type: ZMessageType
    payload: typing.Any = None
    id: UUID = dataclasses.field(default_factory=uuid.uuid4)
    sent_at: datetime | None = None
    version: int = PROTOCOL_VERSION
    # TODO @Performance: expose & use zmq envelope key to filter in zmq

    def __str__(self):
        return f"{self.type} {self.id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self} {self.sent_at}>"

    def payload_as(self, cls: typing.Type[PayloadT]) -> PayloadT:
        if not isinstance(self.payload, cls):
            raise TypeError(f"expected payload to be {cls}, got {type(self.payload)}")
        return self.payload


def serialize_message(message: ZMessage) -> str:
    # serialize any dataclass as something jsonable
    message_dict = {
        "type": message.type,
        "id": str(message.id),
        "sent_at": str(message.sent_at),
        "version": message.version,
    }
    if message.payload is not None:
        # use custom dict encoder for speed and to handle recursive loops
        message_dict["payload"] = to_dict(message.payload)
    return json.dumps(message_dict, cls=MessageJSONEncoder)


def parse_message(message_json: str) -> ZMessage:
    message_dict = json.loads(message_json)
    if message_dict["version"] != PROTOCOL_VERSION:  # inelegant exit for now
        raise RuntimeError(
            f"message version mismatch: {message_dict['_version']} != {PROTOCOL_VERSION}"
        )

    payload_cls = REGISTERED_MESSAGE_PAYLOADS.get(message_dict["type"])
    if payload_cls and message_dict.get("payload") is not None:
        try:
            message_dict["payload"] = from_dict(payload_cls, message_dict["payload"])
        except (ValueError, TypeError, AttributeError) as e:
            logger.exception("parse_message_failed", exc_info=True, e=e)
            raise
    message_dict["type"] = ZMessageType(message_dict["type"])
    message_dict["sent_at"] = datetime.fromisoformat(message_dict["sent_at"])
    message_dict["id"] = UUID(message_dict["id"])

    msg = ZMessage(**message_dict)
    if payload_cls and msg.payload is None:
        # check for missing payload after message is created to get other fields
        raise ValueError(f"missing payload for {msg}")
    return msg


def send_message(sock: zmq.Socket, message: ZMessage | ZMessageType, payload: typing.Any = None):
    if isinstance(message, ZMessageType):
        message = ZMessage(message, payload)
    payload_cls = REGISTERED_MESSAGE_PAYLOADS.get(message.type)
    if payload_cls and message.payload is None:
        raise ValueError(f"missing payload for {message}")
    if message.sent_at is None:
        message.sent_at = datetime.utcnow()
    sock.send_string(serialize_message(message))
    logger.debug("send_message", msg=message, sock=sock)


async def recv_message(sock: zmq.asyncio.Socket) -> ZMessage:
    msg = parse_message(await sock.recv_string())
    logger.debug("recv_message", msg=msg, sock=sock)
    return msg


async def recv_message_with(
    sock: zmq.asyncio.Socket, typ: ZMessageType | typing.Type[PayloadT]
) -> tuple[ZMessage, PayloadT]:
    msg = await recv_message(sock)
    if isinstance(typ, ZMessageType):
        z_type = typ
        payload_cls = REGISTERED_MESSAGE_PAYLOADS.get(typ)
    else:
        z_type = MESSAGE_TYPE_BY_PAYLOAD_CLASS[typ]
        payload_cls = typ
    if msg.type != z_type:
        raise ValueError(f"expected message type {z_type}, got {msg}")
    payload = msg.payload_as(typ) if payload_cls else None
    return msg, payload


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

# TODO @Robustness: close zmq_ctx_sync/zmq_ctx on exit
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
