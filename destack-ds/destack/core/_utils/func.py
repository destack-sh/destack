from collections.abc import Iterator


def get_superclasses[T](cls: type[T], seen: set[type[T]] | None = None) -> Iterator[type[T]]:
    """Gets all superclasses of a class recursively (BFS)."""
    seen = seen if seen is not None else set()
    queue = [cls]
    while queue:
        current = queue.pop(0)
        if current not in seen:
            seen.add(current)
            yield current
            queue.extend(current.__bases__)


def get_subclasses[T](cls: type[T], seen: set[type[T]] | None = None) -> Iterator[type[T]]:
    """Gets all subclasses of a class recursively (BFS)."""
    seen = seen if seen is not None else set()
    queue = [cls]
    while queue:
        current = queue.pop(0)
        if current not in seen:
            seen.add(current)
            yield current
            queue.extend(current.__subclasses__())
