import functools
import typing
from typing import Optional, Sequence, Union
from uuid import UUID

from django.core.exceptions import ValidationError
from django.db import IntegrityError, transaction
from strawberry_django_plus import gql
from strawberry_django_plus.mutations.fields import _map_exception
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo
from strawberry_django_plus.utils.resolvers import async_safe


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
        return gql.mutation(wrapped_func, directives=directives, **kwargs)

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
        return gql.mutation(wrapped_func, directives=directives, **kwargs)

    if func is None:
        return wrapper
    return wrapper(func)


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


def to_uuid(id: UUID | GlobalID | None) -> UUID | None:
    if id is None:
        return id
    if isinstance(id, UUID):
        return id
    return UUID(id.node_id)


def to_uuids(ids: list[UUID | GlobalID] | None) -> list[UUID] | None:
    return [to_uuid(id) for id in ids] if ids else None
