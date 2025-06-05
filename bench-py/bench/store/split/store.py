import asyncio
from collections import defaultdict
from collections.abc import Mapping, Sequence
from typing import override

from more_itertools import flatten

from bench.language import (
    Area,
    Change,
    ChangeResult,
    CustomNodeInstance,
    IsGlobal,
    Query,
    QueryResult,
    RelationType,
    Store,
)
from bench.language.registry import NODE_CLASS_BY_TYPE


class SplitStore(Store):
    """
    Split and route Queries and Changes to the appropriate Stores.
    Does not support atomic Changes across Stores (yet).
    NOTE: obviously SplitStore sharding/routing is very crude for now
    """

    def __init__(self, store_by_area: Mapping[Area, Store]):
        self.store_by_area = store_by_area

    def __str__(self):
        content_parts: list[str] = []
        for area, store in self.store_by_area.items():
            content_parts.append(f"{area.name}={store!s}")
        return ", ".join(content_parts)

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    @override
    async def query(self, query: Query) -> QueryResult:
        area = _get_query_node_area(query)
        if area is None:
            raise ValueError(f"no area for {query!r}")
        store = self.store_by_area[area]
        return await store.query(query)

    @override
    async def commit(self, changes: Sequence[Change]) -> Sequence[ChangeResult]:
        changes_by_area: dict[Area, list[Change]] = defaultdict(list)
        for change in changes:
            area = _get_change_node_area(change)
            if area is None:
                raise ValueError(f"no area for {change!r}")
            changes_by_area[area].append(change)
        commit = await asyncio.gather(
            *[self.store_by_area[area].commit(changes) for area, changes in changes_by_area.items()]
        )
        return tuple(flatten(commit))


def _get_query_node_area(query: Query) -> Area | None:
    if query.relation.type == RelationType.BUILTIN_NODE:
        assert query.relation.node_type is not None, f"no node_type for {query.relation!r}"
        node_cls = NODE_CLASS_BY_TYPE[query.relation.node_type]
    elif query.relation.type == RelationType.CUSTOM_NODE:
        node_cls = CustomNodeInstance
    else:
        return None
    if issubclass(node_cls, IsGlobal):
        return Area.GLOBAL_DATABASE
    else:
        return Area.MAIN_DATABASE


def _get_change_node_area(change: Change) -> Area | None:
    if not change.edits:
        return None
    node_cls = NODE_CLASS_BY_TYPE[change.edits[0].node_type]
    if issubclass(node_cls, IsGlobal):
        return Area.GLOBAL_DATABASE
    else:
        return Area.MAIN_DATABASE
