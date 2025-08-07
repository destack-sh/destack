from collections.abc import Iterable


def first[T](iterable: Iterable[T], default: T | None = None) -> T | None:
    """Return the first item of iterable, or default if iterable is empty."""
    for item in iterable:
        return item
    return default


def flatten[T](iterable: Iterable[Iterable[T]]) -> Iterable[T]:
    """Return an iterator that flattens one level of nesting."""
    for sub_iterable in iterable:
        yield from sub_iterable
