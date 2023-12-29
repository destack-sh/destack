import asyncio
import dataclasses
import enum
import os
import typing
import uuid
from dataclasses import dataclass
from datetime import datetime
from functools import wraps
from typing import Any, Awaitable, Callable, Generic, Type
from uuid import UUID

import msgpack
import nats
import nats.aio.client
import structlog
from nats.aio.subscription import Subscription

from bench.utils.dt import utcnow_with_tz
from bench.utils.serialize import from_dict, to_dict
from bench.utils.utils import get_from_env, required_field, sentry_capture

logger = structlog.get_logger(__name__)
NATS_SERVER = get_from_env("NATS_SERVER", "nats://localhost:4222", type_cast=str)
log = logger.bind(server=NATS_SERVER)

nc = nats.NATS()
nc_init = asyncio.Event()
nc_closed = asyncio.Event()


class MessagingError(Exception):
    pass


async def nats_error_cb(e: Exception) -> None:
    log.error("nats.error", exc_info=e, sentry=sentry_capture(e))


async def nats_disconnected_cb() -> None:
    log.error("nats.disconnected")


async def nats_reconnected_cb() -> None:
    log.info("nats.reconnected")


async def nats_closed_cb() -> None:
    log.info("nats.closed")
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
    log.info("nats.connected", connected=nc.is_connected, name=name)
    nc_init.set()


async def drain_nats():
    await nc.drain()


VERSION = os.environ["VERSION"]


class NMessageType(enum.StrEnum):
    pass


@dataclass(repr=False)
class NMessage(Generic[PayloadT]):
    type: NMessageType
    payload: PayloadT = None
    topic: typing.Optional[str] = None
    sent_at: datetime = required_field()
    id: UUID = dataclasses.field(default_factory=uuid.uuid4)
    version: str = VERSION
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
        reply_msg = NMessage(reply_type, payload, sent_at=utcnow_with_tz())
        log.debug("reply", msg=self, reply=reply_msg)
        serialized = _serialize_message(reply_msg)
        await self.msg.respond(serialized)


def _serialize_message(message: NMessage) -> bytes:
    # serialize any dataclass as something jsonable
    message_dict = {
        "type": message.type,
        "topic": message.topic,
        "id": str(message.id),
        "sent_at": str(message.sent_at),
        "version": message.version,
    }
    if message.payload is not None:
        # use custom dict encoder for speed and to handle recursive loops
        message_dict["payload"] = to_dict(message.payload)
    return msgpack.packb(message_dict, use_bin_type=True)


def _parse_message(message_json: bytes) -> NMessage:
    message_dict = msgpack.unpackb(message_json, raw=False)
    payload_cls = REGISTERED_MESSAGE_PAYLOADS.get(message_dict["type"])
    if payload_cls and message_dict.get("payload") is not None:
        try:
            message_dict["payload"] = from_dict(
                payload_cls, message_dict["payload"], _path=["payload"]
            )
        except (ValueError, TypeError, AttributeError) as e:
            logger.exception(
                "message.parse.failed",
                exc_info=e,
                sentry=sentry_capture(e),
                payload_cls=payload_cls,
                id=message_dict.get("id"),
                type=message_dict.get("type"),
                version=message_dict.get("version"),
            )
            raise
    message_dict["type"] = NMessageType(message_dict["type"])
    message_dict["topic"] = message_dict.get("topic")
    message_dict["sent_at"] = datetime.fromisoformat(message_dict["sent_at"])
    message_dict["id"] = UUID(message_dict["id"])
    message_dict["version"] = message_dict.get("version")

    msg = NMessage(**message_dict)
    if payload_cls and msg.payload is None:
        # check for missing payload after message is created to get other fields
        raise ValueError(f"missing payload for {msg}")
    return msg


async def process_nats_message(
    msg: nats.aio.client.Msg,
    func: Callable[[PayloadT], Awaitable[Any]],
    expect_t: Type[PayloadT] = None,
):
    try:
        message = _parse_message(msg.data)
        message.msg = msg
        if expect_t is not None and not isinstance(message.payload, expect_t):
            raise TypeError(f"expected message {expect_t} for {func}, got {message}")
        return await func(message)
    except Exception as e:
        log.exception("message.process.failed", exc_info=True, e=e, sentry=sentry_capture(e))


def message_handler(func=None):
    def wrapper(func):
        try:
            message_type = func.__annotations__["msg"]
            payload_type = typing.get_args(message_type)[0]
        except Exception as e:
            raise RuntimeError(f"could not get message payload type from {func}: {e}")

        @wraps(func)
        async def wrapped_handler(*args) -> None:
            if len(args) == 1:
                msg = args[0]
                f = func
            else:
                self, msg = args
                f = func.__get__(self)
            return await process_nats_message(msg, f, payload_type)

        wrapped_handler.__wrapped_msg__ = True

        return wrapped_handler

    if func is None:
        return wrapper
    return wrapper(func)


async def request(
    type: NMessageType,
    payload: Payload,
    reply_t: Type[PayloadT] = None,
    *,
    timeout: float = 10,
    retry: int = 0,
    retry_delay: float = 5,
    retry_except: tuple[Type[Exception], ...] = (),
) -> NMessage[PayloadT]:
    if not nc_init.is_set():
        raise RuntimeError("nats not initialized")
    if not isinstance(payload, REGISTERED_MESSAGE_PAYLOADS[type]):
        raise TypeError(f"expected message {type} for {payload}")
    message = NMessage(type=type, payload=payload, sent_at=utcnow_with_tz())
    serialized = _serialize_message(message)
    log.debug("request", topic=payload.topic, message=message, bytes=len(serialized))

    num_retries = retry
    while retry >= 0:
        try:
            reply = await nc.request(payload.topic, serialized, timeout=timeout)
            break
        except Exception as e:
            log.exception(
                "request.failed",
                exc_info=True,
                e=e,
                sentry=sentry_capture(e),
                retry=retry,
            )
            retry -= 1
            if retry < 0 or retry_except and not isinstance(e, retry_except):
                raise MessagingError(
                    f"request {type.name} failed (retries={num_retries}, timeout={timeout})"
                ) from e
            await asyncio.sleep(retry_delay)

    reply_msg = _parse_message(reply.data)
    if reply_t and not isinstance(reply_msg.payload, reply_t):
        raise TypeError(f"expected message {reply_t} for {reply_t}, got {message}")
    reply_msg.msg = reply
    log.debug(
        "request.reply",
        topic=payload.topic,
        message=message,
        reply=reply_msg,
        bytes=len(reply.data),
    )
    return reply_msg


async def _handle_reply(type: str, cb, *, group: str = "") -> Subscription:
    if not nc_init.is_set():
        raise RuntimeError("nats not initialized")
    # topic is type for request/reply
    log.debug("subscribe", type="reply", topic=type, group=group)
    return await nc.subscribe(type, cb=cb, queue=group)


async def publish(type: NMessageType, payload: Any) -> None:
    if not isinstance(payload, REGISTERED_MESSAGE_PAYLOADS[type]):
        raise TypeError(f"expected message {type} for {payload}")
    message1 = NMessage(type=type, topic=payload.topic, payload=payload, sent_at=utcnow_with_tz())
    _serialize_message(message1)  # check that message is serializable for debugging
    message = message1
    message.sent_at = utcnow_with_tz()
    await do_publish(message, message.topic)


async def do_publish(message: NMessage, topic: str):
    if not nc_init.is_set():
        raise RuntimeError("nats not initialized")
    serialized = _serialize_message(message)
    log.debug("publish", topic=topic, message=message, bytes=len(serialized))
    await nc.publish(topic, serialized)


class NSubscription(Generic[PayloadT]):
    def __init__(self):
        self.message_q: asyncio.Queue[NMessage[PayloadT]] = asyncio.Queue()
        self.sub = None

    async def _on_msg(self, msg: nats.aio.client.Msg) -> None:
        message = _parse_message(msg.data)
        message.msg = msg
        log.debug("subscribe.receive", msg=message)
        await self.message_q.put(message)

    async def next_msg(self, timeout: float = None) -> NMessage[PayloadT]:
        return await self.message_q.get()

    async def unsubscribe(self, limit: int = 0) -> None:
        return await self.sub.unsubscribe(limit)


async def subscribe(
    topic: str,
    *,
    payload_t: Type[PayloadT] = None,
    cb: Callable[[NMessage], Awaitable[None]] = None,
) -> NSubscription[PayloadT]:
    if not nc_init.is_set():
        raise RuntimeError("nats not initialized")
    if payload_t is not None and not isinstance(payload_t, type):
        raise TypeError(f"payload_t must be a subclass of Payload, got {payload_t}")

    log.debug("subscribe", topic=topic, cb=cb)
    if cb is not None:
        if hasattr(cb, "__wrapped_msg__"):
            # already wrapped by message_handler
            wrapped_cb = cb
        else:

            async def wrapped_cb(msg: nats.aio.client.Msg) -> None:
                return await process_nats_message(msg, cb, expect_t=payload_t)

        return await nc.subscribe(topic, cb=wrapped_cb)  # noqa: duck typed, but properly typed
    else:
        subscription = NSubscription()
        subscription.sub = await nc.subscribe(topic, cb=subscription._on_msg)
        return subscription


async def subscribe_many(
    topics: dict[str, Type[PayloadT]],
) -> NSubscription[PayloadT]:
    if not nc_init.is_set():
        raise RuntimeError("nats not initialized")

    log.debug("subscribe_many", topics=topics.keys())
    subscription = NSubscription()
    for topic in topics:
        await nc.subscribe(topic, cb=subscription._on_msg)
    return subscription
