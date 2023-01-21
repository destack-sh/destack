from typing import Optional, Sequence

from strawberry_django_plus import gql
from strawberry_django_plus.utils.resolvers import async_safe


def async_safe_mutation(
    func: Optional = None, directives: Optional[Sequence[object]] = (), **kwargs
):
    """Wraps a mutation resolver to make it async-safe by wrapping in sync_to_async as needed."""

    def wrapper(func):
        return gql.mutation(async_safe(func), directives=directives, **kwargs)

    if func is None:
        return wrapper
    return wrapper(func)
