import json
import uuid
from collections import defaultdict, deque
from itertools import chain
from typing import Any, Deque, Iterator, Type, TypeVar

from django.core.validators import RegexValidator
from django.db import models
from django.db.models import QuerySet

from bench.utils.uuidt import NAME_REGEX, UUIDT


class UUIDModel(models.Model):
    id = models.UUIDField(primary_key=True, default=uuid.uuid4, editable=False)

    class Meta:
        abstract = True


class UUIDTModel(UUIDModel):
    id = models.UUIDField(primary_key=True, default=UUIDT, editable=False)

    class Meta:
        abstract = True


NAME_VALIDATOR = RegexValidator(NAME_REGEX)

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
    all_ids = set()
    for obj in objects:
        children_by_parent_id[getattr(obj, parent_id_attr)].append(obj)
        all_ids.add(obj.id)
    # find roots (objects without parent in the set)
    root_ids = set()
    for parent_id in children_by_parent_id:
        if parent_id not in all_ids:
            root_ids.add(parent_id)
    roots = list(chain.from_iterable(children_by_parent_id.pop(root_id) for root_id in root_ids))
    # batch by level (breadth-first)
    # source objects may be mutated during iteration so do this upfront
    seen_ids = set()
    children_levels = []
    children = roots
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
