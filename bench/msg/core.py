import asyncio
import dataclasses
import json
import uuid
from dataclasses import dataclass
from datetime import datetime
from typing import Any, Awaitable, Callable, Generic, Type, TypeVar, cast
from uuid import UUID

import nats
import nats.aio.client
import structlog
from nats.aio.subscription import Subscription

from bench.msg.messages import (
    PROTOCOL_VERSION,
    REGISTERED_MESSAGE_PAYLOADS,
    REPLY_BY_REQUEST_TYPE,
    NMessageType,
    to_topic,
)
from bench.msg.serialize import from_dict, to_dict
from bench.settings import NATS_SERVER
from bench.utils.func import wrap_task
from bench.utils.utils import sentry_capture_if_enabled

logger = structlog.get_logger(__name__)
log = logger.bind(server=NATS_SERVER)

nc = nats.NATS()
nc_init = asyncio.Event()
nc_closed = asyncio.Event()


async def nats_error_cb(e: Exception) -> None:
    sentry_enabled = sentry_capture_if_enabled(e)
    log.error("nats_error", exc_info=e, sentry_enabled=sentry_enabled)


async def nats_disconnected_cb() -> None:
    log.error("nats_disconnected")


async def nats_reconnected_cb() -> None:
    log.info("nats_reconnected")


async def nats_closed_cb() -> None:
    log.info("nats_closed")
    nc_closed.set()


async def init_nats(name: str = "bench"):
    await nc.connect(
        NATS_SERVER,
        name=name,
        error_cb=nats_error_cb,
        disconnected_cb=nats_disconnected_cb,
        reconnected_cb=nats_reconnected_cb,
        closed_cb=nats_closed_cb,
    )
    nc_init.set()


async def drain_nats():
    await nc.drain()


PayloadT = TypeVar("PayloadT", bound=Any)
# TODO @Broken: fix type checking (it just worked before?)


@dataclass(repr=False)
class NMessage(Generic[PayloadT]):
    type: NMessageType
    payload: PayloadT = None
    id: UUID = dataclasses.field(default_factory=uuid.uuid4)
    sent_at: datetime | None = None
    version: int = PROTOCOL_VERSION
    msg: nats.aio.client.Msg | None = None  # the original nats message (if received)

    def __str__(self):
        return f"{self.type} {self.id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self} {self.sent_at}>"

    def payload_as(self, cls: Type[PayloadT]) -> PayloadT:
        if not isinstance(self.payload, cls):
            raise TypeError(f"expected payload to be {cls}, got {type(self.payload)}")
        return self.payload

    @property
    def p(self):
        return self.payload

    async def reply(self, payload: Any) -> None:
        if self.msg is None:
            raise RuntimeError("cannot reply to a message without a msg")
        reply_type = REPLY_BY_REQUEST_TYPE[self.type]
        reply_payload_cls = REGISTERED_MESSAGE_PAYLOADS[reply_type]
        if not isinstance(payload, reply_payload_cls):
            raise TypeError(
                f"expected reply payload to {self} to be {reply_payload_cls}, got {type(payload)}: {payload}"
            )
        serialized = _serialize_message(NMessage(reply_type, payload))
        await self.msg.respond(serialized.encode("utf-8"))


def _serialize_message(message: NMessage) -> str:
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


def _parse_message(message_json: str) -> NMessage:
    message_dict = json.loads(message_json)
    if message_dict["version"] != PROTOCOL_VERSION:  # inelegant exit for now
        raise RuntimeError(
            f"message version mismatch: {message_dict['_version']} != {PROTOCOL_VERSION}"
        )

    payload_cls = REGISTERED_MESSAGE_PAYLOADS.get(message_dict["type"])
    if payload_cls and message_dict.get("payload") is not None:
        try:
            message_dict["payload"] = from_dict(
                payload_cls, message_dict["payload"], _path=["payload"]
            )
        except (ValueError, TypeError, AttributeError) as e:
            sentry_enabled = sentry_capture_if_enabled(e)
            logger.exception(
                "parse_message_failed", exc_info=True, e=e, sentry_enabled=sentry_enabled
            )
            raise
    message_dict["type"] = NMessageType(message_dict["type"])
    message_dict["sent_at"] = datetime.fromisoformat(message_dict["sent_at"])
    message_dict["id"] = UUID(message_dict["id"])

    msg = NMessage(**message_dict)
    if payload_cls and msg.payload is None:
        # check for missing payload after message is created to get other fields
        raise ValueError(f"missing payload for {msg}")
    return msg


async def process_nats_message(
    msg: nats.aio.client.Msg, func: Callable[[PayloadT], Awaitable[Any]], expect_t: Type[PayloadT]
):
    try:
        message = _parse_message(msg.data.decode())
        message.msg = msg
        if not isinstance(message.payload, expect_t):
            raise TypeError(f"expected message {expect_t}, got {message}")
        return await func(message)
    except Exception as e:
        sentry_enabled = sentry_capture_if_enabled(e)
        log.exception("message_handler_error", exc_info=True, e=e, sentry_enabled=sentry_enabled)


def message_handler(func=None):
    def wrapper(func):
        # TODO @Broken: get message type from function signature generics
        message_type = func.__annotations__["msg"]

        async def wrapped_handler(msg: nats.aio.client.Msg) -> None:
            return await process_nats_message(msg, func, message_type)

        return wrapped_handler

    if func is None:
        return wrapper
    return wrapper(func)


async def request(
    type: NMessageType,
    payload: Any,
    reply_t: Type[PayloadT],
    *,
    topic: str = None,
    timeout: float = 10,
) -> PayloadT:
    if not nc_init.is_set():
        raise RuntimeError("nats not initialized")
    if topic is None:
        topic = to_topic(type, payload)
    message = NMessage(type, payload)
    serialized = _serialize_message(message)
    reply = await nc.request(topic, serialized.encode("utf-8"), timeout=timeout)
    reply_msg = _parse_message(reply.data.decode())
    reply_msg.msg = reply
    return reply_msg.payload_as(reply_t)


async def handle_reply(type: NMessageType, cb, *, group: str = None) -> Subscription:
    if not nc_init.is_set():
        raise RuntimeError("nats not initialized")
    # topic is type for request/reply
    return await nc.subscribe(type, cb=cb, queue=group)


async def publish(type: NMessageType, payload: Any, *, topic: str = None) -> None:
    if not nc_init.is_set():
        raise RuntimeError("nats not initialized")
    if topic is None:
        topic = to_topic(type, payload)
    message = NMessage(type, payload)
    serialized = _serialize_message(message)
    await nc.publish(topic, serialized.encode("utf-8"))


def publish_soon(type: NMessageType, payload: Any, *, topic: str = None) -> None:
    asyncio.create_task(wrap_task(publish(type, payload, topic=topic), f"publish_soon_{type}"))


class NSubscription(Generic[PayloadT]):
    def __init__(self, sub: Subscription):
        self.sub = sub

    async def next_msg(self, timeout: float = None) -> NMessage[PayloadT]:
        nats_msg = await self.sub.next_msg(timeout=timeout)
        msg = _parse_message(nats_msg.data.decode())
        msg.msg = nats_msg
        return msg

    async def unsubscribe(self, limit: int = 0) -> None:
        return await self.sub.unsubscribe(limit)


async def subscribe(
    topic: str, payload_t: Type[PayloadT], *, cb: Callable[[NMessage], Awaitable[None]] = None
) -> NSubscription[PayloadT]:
    if not nc_init.is_set():
        raise RuntimeError("nats not initialized")

    async def wrapped_cb(msg: nats.aio.client.Msg) -> None:
        return process_nats_message(msg, cb, expect_t=payload_t)

    if cb is not None:
        cb = wrapped_cb

    sub = await nc.subscribe(topic, cb=cb)
    return cast(NSubscription[PayloadT], NSubscription(sub))


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
