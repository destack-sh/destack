import functools
import typing
from datetime import datetime
from typing import Iterable, Optional, Sequence, Union
from uuid import UUID

import strawberry
import strawberry_django
import structlog
from django.core.exceptions import ValidationError
from django.db import IntegrityError, transaction
from more_itertools import first
from strawberry import lazy
from strawberry.channels.handlers.http_handler import ChannelsRequest
from strawberry.channels.handlers.ws_handler import GraphQLWSConsumer
from strawberry.relay import GlobalID
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.language import Q, query
from bench.msg.messages import ClientOrigin
from bench.utils.utils import sentry_capture_if_enabled

if typing.TYPE_CHECKING:
    from bench.api.user import User

logger = structlog.get_logger(__name__)


def async_safe(func):
    return func  # nocheckin


@strawberry.type
class Revisioned:
    revision: int


@strawberry.interface
class HasCrud:
    id: GlobalID
    created_at: datetime
    updated_at: datetime
    deleted_at: Optional[datetime]
    created_by: Optional[typing.Annotated["User", lazy(".user")]]
    last_edited_at: Optional[datetime]
    last_edited_by: Optional[typing.Annotated["User", lazy(".user")]]


@strawberry.interface
class ModuleNode:
    id: GlobalID
    parent: Optional["ModuleNode"]


def safe_mutation(
    func: Optional = None,
    directives: Optional[Sequence[object]] = (),
    atomic: bool = False,
    **kwargs,
):
    """Wraps a mutation resolver to make it async-safe and wrap exceptions into OperationInfo."""

    def wrapper(func):
        func = wrap_exceptions(func)
        if atomic:

            @functools.wraps(func)
            def wrapped_atomic(*args, **kwargs):
                with transaction.atomic():
                    return func(*args, **kwargs)

            wrapped_func = wrapped_atomic
        else:
            wrapped_func = func
        wrapped_func = async_safe(wrapped_func)
        return strawberry_django.mutation(wrapped_func, directives=directives, **kwargs)

    if func is None:
        return wrapper
    return wrapper(func)


def asafe_mutation(
    func: Optional = None,
    directives: Optional[Sequence[object]] = (),
    atomic: bool = False,
    **kwargs,
):
    """Wraps an async resolver to make it atomic and wrap exceptions into OperationInfo."""

    def wrapper(func):
        func = wrap_exceptions(func)
        if atomic:

            @functools.wraps(func)
            async def wrapped_atomic(*args, **kwargs):
                async with transaction.atomic():
                    return await func(*args, **kwargs)

            wrapped_func = wrapped_atomic
        else:
            wrapped_func = func
        return strawberry_django.mutation(wrapped_func, directives=directives, **kwargs)

    if func is None:
        return wrapper
    return wrapper(func)


def asafe_subscription(func, **kwargs):
    """Wraps an async resolver to wrap exceptions into OperationInfo."""

    @functools.wraps(func)
    async def wrapped(*args, **kwargs):
        try:
            async for item in func(*args, **kwargs):
                yield item
        except Exception as e:
            # no way to propagate exception to client here?
            logger.error(
                "subscribe.error", func=func, exc_info=e, sentry=sentry_capture_if_enabled(e)
            )
            raise StopAsyncIteration from e

    return strawberry.subscription(wrapped, **kwargs)


def wrap_exceptions(func):
    """Wraps a mutation resolver to map exceptions to OperationInfo."""

    # check that func returns a union with OperationInfo
    return_type = func.__annotations__.get("return")
    if return_type is None:
        raise TypeError(f"return type annotation required: {func}")
    return_type_args = typing.get_args(return_type)
    if OperationInfo not in return_type_args:
        raise TypeError(f"return must union with OperationInfo: {func} -> {return_type_args}")

    @functools.wraps(func)
    def wrapped(*args, **kwargs):
        try:
            return func(*args, **kwargs)
        except Exception as e:
            e = map_exception(e)
            if isinstance(e, OperationInfo):
                return e
            raise e

    return wrapped


def map_exception(e: Exception) -> Union[OperationInfo, Exception]:
    # extend strawberry's _map_exception
    if isinstance(e, IntegrityError):
        e = ValidationError(e.args[0])
    return _map_exception(e)  # borrowed from strawberry_django_plus


def to_uuid(id: str | UUID | GlobalID | None) -> UUID | None:
    if id is None:
        return id
    if isinstance(id, str):
        return UUID(id)
    if isinstance(id, UUID):
        return id
    return UUID(id.node_id)


def to_uuids(ids: list[UUID | GlobalID] | None) -> list[UUID] | None:
    return [to_uuid(id) for id in ids] if ids else None


def to_global_id(type: str, id: UUID | None) -> GlobalID | None:
    if id is None:
        return None
    return GlobalID(type, str(id))


def get_client_origin_from_info(info: Info) -> ClientOrigin:
    request = info.context["request"]
    if isinstance(request, GraphQLWSConsumer):
        scope = request.scope
    elif isinstance(request, ChannelsRequest):
        scope = request.consumer.scope
    else:
        raise TypeError(f"unexpected request type: {request}")
    client_id = scope["session"]["client_id"]
    # get nonce from list of headers
    client_nonce = first(
        (v.decode() for k, v in scope["headers"] if k.decode().lower() == "x-client-nonce"), None
    )
    client_nonce = UUID(client_nonce) if client_nonce else None
    origin = ClientOrigin("user", client_id, client_nonce)
    return origin


def get_user_from_info(info: Info) -> models.User:
    request = info.context["request"]
    if isinstance(request, GraphQLWSConsumer):
        return request.scope["user"]._wrapped
    elif isinstance(request, ChannelsRequest):
        return request.consumer.scope["user"]._wrapped
    else:
        raise TypeError(f"unexpected request type: {request}")


class ThingBatch(Iterable):
    @property
    def things(self):
        raise NotImplementedError

    # pretend to be an iterable for simpler perms checking
    # (doesn't need to know about the Batch type, which is
    #  required because we can't union list[Statement] | OperationInfo)

    def __getitem__(self, item):
        return self.things[item]

    def __len__(self):
        return len(self.things)

    def __iter__(self):
        return iter(self.things)


SortOrder = strawberry.enum(query.SortOrder)
SortMode = strawberry.enum(query.SortMode)
QueryOp = strawberry.enum(query.QueryOp)
AggregationOp = strawberry.enum(query.AggregationOp)


@strawberry.input
class SearchQuery:
    op: QueryOp
    key: Optional[str] = None
    value: Optional[JSON] = None
    queries: Optional[list["SearchQuery"]] = None

    def to_dsl(self) -> query.Query:
        queries = [q.to_dsl() for q in self.queries] if self.queries else None
        return Q(self.op, queries=queries, key=self.key, value=self.value)


@strawberry.input
class SearchSort:
    key: str
    order: SortOrder = SortOrder.ASCENDING
    mode: Optional[SortMode] = None

    def to_dsl(self) -> query.Sort:
        return query.Sort(self.key, self.order, self.mode)
