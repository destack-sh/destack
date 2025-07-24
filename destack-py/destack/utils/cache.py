from collections.abc import Hashable
from functools import wraps
from typing import Any, Callable


def cached(
    cache: dict[Hashable, Any],
    key: Callable[..., Hashable] | None = None,
) -> Callable[[Callable], Callable]:
    """
    Decorator to cache function results.
    """

    def decorator(func: Callable) -> Callable:
        @wraps(func)
        def wrapper(*args, **kwargs):
            # compute cache key
            if key is not None:  # noqa: SIM108
                cache_key = key(*args, **kwargs)
            else:
                # default key is a tuple of args and sorted kwargs items
                cache_key = _make_key(args, kwargs)

            # check cache
            if cache_key in cache:
                return cache[cache_key]

            # compute and cache result
            result = func(*args, **kwargs)
            cache[cache_key] = result
            return result

        # expose cache for introspection/clearing
        wrapper.cache = cache  # type: ignore
        return wrapper

    return decorator


def _make_key(args: tuple, kwargs: dict) -> Hashable:
    """Create a hashable cache key from function arguments."""
    # convert kwargs to a sorted tuple of items for consistent hashing
    kwargs_items = tuple(sorted(kwargs.items())) if kwargs else ()
    return (args, kwargs_items)
