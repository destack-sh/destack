from typing import Optional

import structlog
from more_itertools import first
from strawberry.relay import GlobalID
from strawberry.types import Info
from strawberry.types.nodes import FragmentSpread, SelectedField
from strawberry.utils.str_converters import to_camel_case
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import check_can_read_project
from bench.api.utils import ModuleNode
from bench.language.core import MOT
from bench.models import packer
from bench.models.packer import MOT_BY_BASE_MODEL_CLASS

logger = structlog.get_logger(__name__)

# this is a hack until we have proper generated module GQL types
GQL_NODE_NAME_BY_MOT = {
    MOT.MODULE: "ProjectVersion",
    MOT.FILE: "File",
    MOT.STATEMENT: "Statement",
    MOT.RECORD: "Record",
    MOT.FIELD: "Field",
    MOT.TAGGING: "Tagging",
    MOT.TRIGGER: "Trigger",
    MOT.COMMENT: "Comment",
    MOT.DATASET_VIEW: "DatasetView",
    MOT.DATASET_VIEW_FIELD: "DatasetViewField",
    MOT.RESOLVED_FIELD: "ResolvedField",
    MOT.ISSUE: "Issue",
}
MOT_NAME_BY_GQL_NODE = {v: k for k, v in GQL_NODE_NAME_BY_MOT.items()}
assert len(GQL_NODE_NAME_BY_MOT) == len(MOT), f"missing {set(MOT) - GQL_NODE_NAME_BY_MOT.keys()}"


class IdOnlyProxy:
    def __init__(self, id: GlobalID):
        self.id = id


class StaticPrefetchedQueryset:
    """
    Imitate a django queryset from a list of objects.
    Any further filtering is ignored, we only return the objects we have.
    """

    def __init__(self, objects):
        self._result_cache = objects

    def _fetch_all(self):
        pass  # we already have all the objects (this is called by strawberry)

    def all(self):
        return self

    def filter(self, *args, **kwargs):
        return self

    def get(self, *args, **kwargs):
        return first(self._result_cache)

    def first(self):
        return first(self._result_cache)

    def __iter__(self):
        return iter(self._result_cache)

    def __bool__(self):
        return bool(self._result_cache)

    def __getitem__(self, item):
        return self._result_cache[item]

    def __len__(self):
        return len(self._result_cache)


_MODEL_FIELD_NAME_BY_CAMEL: dict[str, str] = {}
_CAMEL_FIELD_NAME_BY_MODEL: dict[str, str] = {}
_BASE_MODEL_BY_CAMEL_FIELD: dict[str, type] = {}


def _add_field_name(name: str):
    gql_name = to_camel_case(name)
    if gql_name in _MODEL_FIELD_NAME_BY_CAMEL and _MODEL_FIELD_NAME_BY_CAMEL[gql_name] != name:
        raise ValueError(
            f"duplicate field name {gql_name} for {name} and {_MODEL_FIELD_NAME_BY_CAMEL[gql_name]}"
        )
    _MODEL_FIELD_NAME_BY_CAMEL[gql_name] = name
    _CAMEL_FIELD_NAME_BY_MODEL[name] = gql_name


def _collect_fields():
    for model in packer.MOT_BY_BASE_MODEL_CLASS.keys():
        for field in model._meta.fields:
            _add_field_name(field.name)
            # and related name if any
            if field.is_relation:
                _add_field_name(field.remote_field.name)
        for field in model._meta.fields_map.values():
            _add_field_name(field.name)
            if field.is_relation:
                _BASE_MODEL_BY_CAMEL_FIELD[to_camel_case(field.name)] = field.related_model
        for field in model._meta.many_to_many:
            _add_field_name(field.name)


_collect_fields()


def _inline_fragments(fields: list[SelectedField | FragmentSpread]) -> list[SelectedField]:
    """Inline any fragment spreads in the fields"""
    if not any(isinstance(f, FragmentSpread) for f in fields):
        return fields
    inlined = []
    for f in fields:
        if isinstance(f, FragmentSpread):
            inlined.extend(f.selections)
        else:
            inlined.append(f)
    return inlined


def _walk_fragments(fields: list[SelectedField | FragmentSpread]) -> list[SelectedField]:
    """Recursively walk the fields and inline any fragment spreads"""
    inlined = []
    for f in fields:
        if not isinstance(f, FragmentSpread):
            inlined.append(f)
        if f.selections:
            inlined.extend(_walk_fragments(f.selections))
    return inlined


ALLOWED_EXTERNAL_RELATIONS = {models.Project, models.User}
FLATTENED_RELATIONS = {(models.ProjectVersion, models.File), (models.File, models.Statement)}


def read_module_node(info: Info, id: GlobalID) -> Optional[ModuleNode] | OperationInfo:
    """
    Reads a module node in an optimized way (that assumes tree-shaped retrieval).
    Any nodes not in the tree will be fetched by the strawberry resolver.

    TODO @Broken: read module node assumes default filters
    """
    assert len(info.selected_fields) == 1, "only one root field expected"

    qs = models.__dict__[id.type_name].objects.all()
    node = qs.filter(id=id.node_id).first()
    if not node:
        return None
    check_can_read_project(info, node)

    logger.debug("module.read_node", id=id, node=node)

    # collect relevant nodes (naively filter by selected fields)
    root_selections = _inline_fragments(info.selected_fields[0].selections)
    included = {models.ProjectVersion, models.File, models.Statement}
    for field in _walk_fragments(root_selections):
        base_model = _BASE_MODEL_BY_CAMEL_FIELD.get(field.name)
        if base_model and base_model not in included:
            included.add(base_model)
    excluded = MOT_BY_BASE_MODEL_CLASS.keys() - included
    visited = packer.collect_node(node, excluded=excluded)

    # map relevant selected fields to the visited nodes
    def _resolve(n: models.ModuleNode, selections: list[SelectedField]) -> ModuleNode:
        model_name = n._meta.object_name

        proxy_n = n.__class__()
        for f in selections:
            py_name = _MODEL_FIELD_NAME_BY_CAMEL.get(f.name, f.name)
            # pass through non-relational fields
            if f.name in ("__typename", "id") or not f.selections:
                if hasattr(n, py_name):
                    setattr(proxy_n, py_name, getattr(n, py_name))
                continue
            # shortcut for parent (which isn't a real field)
            elif f.name == "parent":
                # find the parent node in its fields (where its value == n.parent_id)
                parent_field = first(
                    (k for k in n._meta.fields if getattr(n, k.column) == n.parent_id)
                )
                # set id and relation field
                setattr(proxy_n, parent_field.attname, n.parent_id)
                setattr(proxy_n, parent_field.name, parent_field.related_model(id=n.parent_id))
                continue

            # relational field
            django_field = n._meta.get_field(py_name)
            # error on invalid relations to models outside the module tree
            if (
                django_field.related_model not in packer.MOT_BY_BASE_MODEL_CLASS
                and django_field.related_model not in ALLOWED_EXTERNAL_RELATIONS
            ):
                raise ValueError(
                    f"invalid relation {django_field.related_model} for {model_name}.{py_name}"
                )
            inner_selections = _inline_fragments(f.selections)
            # for 1:1 relations use id only proxy
            if django_field.one_to_one or django_field.many_to_one:
                if django_field.related_model in ALLOWED_EXTERNAL_RELATIONS:
                    # external rotations are properly queried
                    # this is okay because we only do this once usually (e.g. top-level project)
                    setattr(proxy_n, py_name, getattr(n, py_name))
                else:
                    # assumes { __typename, id } selection only (that's all we know here)
                    assert len(inner_selections) == 2, f"bad {inner_selections} for {django_field}"
                    remote_id = getattr(n, py_name + "_id")
                    remote_stub = django_field.related_model(id=remote_id) if remote_id else None
                    setattr(proxy_n, py_name, remote_stub)
            # for 1:n relations get children
            # for flattened relations get all descendants (of same type)
            elif django_field.one_to_many or django_field.many_to_many:
                related = []
                if (type(n), django_field.related_model) in FLATTENED_RELATIONS:
                    # collect descendants of same type
                    remaining = visited.visited_by_parent.get(n.id, [])
                    while remaining:
                        child = remaining.pop()
                        if type(child) != django_field.related_model:
                            continue
                        related.append(_resolve(child, inner_selections))
                        remaining.extend(visited.visited_by_parent.get(child.id, []))
                else:
                    # collect immediate children only
                    for child in visited.visited_by_parent.get(n.id, []):
                        if type(child) != django_field.related_model:
                            continue  # ignore children of other types
                        related.append(_resolve(child, inner_selections))
                # set children list on proxy to 'cache' it in the Django model
                if not hasattr(proxy_n, "_prefetched_objects_cache"):
                    proxy_n._prefetched_objects_cache = {}
                proxy_n._prefetched_objects_cache[py_name] = StaticPrefetchedQueryset(related)
            else:
                raise ValueError(f"unexpected relation {django_field} for {model_name}.{py_name}")

        return proxy_n

    logger.debug(
        "module.read_node.resolve", id=id, node=node, nodes=len(visited.visited), excluded=excluded
    )
    resolved_node = _resolve(node, root_selections)

    logger.debug(
        "module.read_node.done", id=id, node=node, nodes=len(visited.visited), excluded=excluded
    )

    return resolved_node
