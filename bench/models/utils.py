import json
import uuid
from collections import defaultdict, deque
from typing import Any, Deque, Iterator, Type, TypeVar

from django.db import models
from django.db.models import QuerySet

from bench.utils.uuidt import UUIDT


class UUIDModel(models.Model):
    id = models.UUIDField(primary_key=True, default=uuid.uuid4, editable=False)

    class Meta:
        abstract = True


class UUIDTModel(UUIDModel):
    id = models.UUIDField(primary_key=True, default=UUIDT, editable=False)

    class Meta:
        abstract = True


ModelT = TypeVar("ModelT", bound=models.Model)


def proxies(query: QuerySet, proxy_type: Type[ModelT]) -> QuerySet[ModelT]:
    """
    Return proxies of query type.
    """
    if query.model not in proxy_type.mro():
        raise TypeError(f"cannot cast {query.model} to {proxy_type}")
    # TODO @Robustness: test and/or replace mechanism for "casting" model to proxy subtype
    #  Potential alternatives include using QuerySet.annotate.
    #  May also want to monkey-patch (and type) .proxies on standard QuerySet.
    query.model = proxy_type
    return query


def is_jsonable(value: Any) -> bool:
    """Check if value is JSON-serializable."""
    try:
        json.dumps(value)
        return True
    except TypeError:
        return False


T = TypeVar("T")


def walk_children_bfs_qs(objects: list[T], child_attr: str) -> Iterator[T]:
    """
    Walk all children of an object in breadth-first order using a children attribute.
    """
    queue: Deque[T] = deque(objects)
    while queue:
        obj = queue.popleft()
        # copy children before yielding to avoid concurrent modification while copying
        children = list(getattr(obj, child_attr).all())
        yield obj
        queue.extend(children)


def walk_children_bfs_batched(objects: list[T], parent_id_attr: str) -> Iterator[T]:
    """
    Walk all children of an object in breadth-first order
    by building the tree in-memory with the parent attribute.

    Yields batches of *all* children at each level.
    """
    children_by_parent_id = defaultdict(list)
    for obj in objects:
        children_by_parent_id[getattr(obj, parent_id_attr)].append(obj)

    # first batch by level (breadth-first) since source objects may be mutated during iteration
    seen_ids = set()
    children_levels = []
    children = children_by_parent_id[None]
    while children:
        children_levels.append(children)
        next_children = []
        for obj in children:
            if obj.id in seen_ids:
                continue  # ignore cycles here (shouldn't happen)
            seen_ids.add(obj.id)
            if obj.id in children_by_parent_id:
                next_children.extend(children_by_parent_id[obj.id])
        children = next_children
    # then yield from each level
    yield from children_levels


def walk_children_bfs(objects: list[T], parent_attr: str) -> Iterator[T]:
    """
    Walk all children of an object in breadth-first order
    by building the tree in-memory with the parent attribute.
    """
    for batch in walk_children_bfs_batched(objects, parent_attr):
        for obj in batch:
            yield obj
