import asyncio
from collections import defaultdict
from collections.abc import Mapping, Sequence
from typing import override

from more_itertools import flatten

from destack.language import (
    AreaType,
    Change,
    ChangeResult,
    CustomEntity,
    Edit,
    IsGlobal,
    Query,
    QueryResult,
    RelationType,
    Store,
)
from destack.language.core.common.edit import ChangeStatus
from destack.language.registry import NODE_CLASS_BY_TYPE

# nocheckin: proper SplitStore


class SplitStore(Store):
    """
    Split and route Queries and Changes to the appropriate Stores.
    Does not support atomic Changes across Stores (yet).
    NOTE: obviously SplitStore sharding/routing is very crude for now
    """

    def __init__(self, store_by_area: Mapping[AreaType, Store]):
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
        results: list[ChangeResult] = []
        for change in changes:
            # split Changes for each area
            edits_by_area: dict[AreaType, list[Edit]] = defaultdict(list)
            for edit in change.edits:
                area = _get_edit_node_area(edit)
                if area is None:
                    raise ValueError(f"no area for {edit!r}")
                edits_by_area[area].append(edit)
            changes_by_area: dict[AreaType, Change] = {}
            for area, edits in edits_by_area.items():
                changes_by_area[area] = Change(
                    id=change.id,
                    created_at=change.created_at,
                    edits=edits,
                )

            # apply changes to relevant stores in parallel (not atomic!)
            area_results = await asyncio.gather(
                *[
                    self.store_by_area[area].commit([change])
                    for area, change in changes_by_area.items()
                ]
            )
            area_edits: list[Edit] = []
            area_cascaded_edits: list[Edit] = []
            change_status = ChangeStatus.COMPLETED
            for area_result in flatten(area_results):
                if area_result.status != ChangeStatus.COMPLETED:
                    change_status = ChangeStatus.FAILED
                area_edits.extend(area_result.edits)
                area_cascaded_edits.extend(area_result.cascaded_edits)
            combined_result = ChangeResult(
                id=change.id,
                created_at=change.created_at,
                status=change_status,
                edits=area_edits,
                cascaded_edits=area_cascaded_edits,
            )
            results.append(combined_result)

        return results


def _get_query_node_area(query: Query) -> AreaType | None:
    if query.relation.type == RelationType.BUILTIN_NODE:
        assert query.relation.node_type is not None, f"no node_type for {query.relation!r}"
        node_cls = NODE_CLASS_BY_TYPE[query.relation.node_type]
    elif query.relation.type == RelationType.CUSTOM_NODE:
        node_cls = CustomEntity
    else:
        return None
    if issubclass(node_cls, IsGlobal):
        return AreaType.GLOBAL_DATABASE
    else:
        return AreaType.SPACE_DATABASE


def _get_edit_node_area(edit: Edit) -> AreaType | None:
    if not edit.node_type:
        return None
    node_cls = NODE_CLASS_BY_TYPE[edit.node_type]
    if issubclass(node_cls, IsGlobal):
        return AreaType.GLOBAL_DATABASE
    else:
        return AreaType.SPACE_DATABASE
