from typing import Optional
from uuid import UUID

import structlog
from more_itertools import first
from strawberry.relay import GlobalID
from strawberry.types import Info
from strawberry.types.nodes import FragmentSpread, InlineFragment, SelectedField
from strawberry.utils.str_converters import to_camel_case
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import check_module_node_access
from bench.api.utils import ModuleNode
from bench.language.const import MNT
from bench.models import ModuleAccessLevel, packer
from bench.models.packer import MNT_BY_BASE_MODEL_CLASS

logger = structlog.get_logger(__name__)

# this is a hack until we have proper generated module GQL types
GQL_NODE_NAME_BY_MNT = {
    MNT.Module: "ProjectVersion",
    MNT.File: "File",
    MNT.Statement: "Statement",
    MNT.Record: "Record",
    MNT.Field: "Field",
    MNT.Tagging: "Tagging",
    MNT.Trigger: "Trigger",
    MNT.Comment: "Comment",
    MNT.DatasetView: "DatasetView",
    MNT.DatasetViewField: "DatasetViewField",
    MNT.ResolvedField: "ResolvedField",
    MNT.Issue: "Issue",
}
MNT_NAME_BY_GQL_NODE = {v: k for k, v in GQL_NODE_NAME_BY_MNT.items()}
assert len(GQL_NODE_NAME_BY_MNT) == len(MNT), f"missing {set(MNT) - GQL_NODE_NAME_BY_MNT.keys()}"


class StaticPrefetchedQueryset:
    """
    Imitate a django queryset from a list of objects.
    Any further filtering is ignored, we only return the objects we have.
    """

    def __init__(self, objects):
        self._result_cache = objects

    def __str__(self):
        return str(self._result_cache)

    def __repr__(self):
        return f"<{self.__class__.__name__} {self._result_cache}>"

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
    for model in packer.MNT_BY_BASE_MODEL_CLASS.keys():
        for field in model._meta.fields:
            _add_field_name(field.name)
            # and related name if any
            if field.is_relation:
                _add_field_name(field.remote_field.name)
        for field in model._meta.fields_map.values():
            _add_field_name(field.name)
            # add relations to enable lookup of related model by field name
            if field.is_relation:
                camel_name = to_camel_case(field.name).replace("+", "")
                if camel_name:
                    _BASE_MODEL_BY_CAMEL_FIELD[camel_name] = field.related_model
        for field in model._meta.many_to_many:
            _add_field_name(field.name)


_collect_fields()


def _inline_fragments(
    fields: list[SelectedField | FragmentSpread | InlineFragment],
    recursive: bool = False,
) -> list[SelectedField]:
    """Inline any fragment spreads in the fields"""
    inlined = []
    for f in fields:
        is_inner = isinstance(f, (FragmentSpread, InlineFragment))
        if is_inner or f.selections and recursive:
            inlined.extend(_inline_fragments(f.selections, recursive=recursive))
        if not is_inner:
            inlined.append(f)
    return inlined


ALLOWED_EXTERNAL_RELATIONS = {models.Project, models.User}
FLATTENED_RELATIONS = {(models.ProjectVersion, models.File), (models.File, models.Statement)}


def read_module_node_by_id(info: Info, id: GlobalID) -> Optional[ModuleNode] | OperationInfo:
    assert len(info.selected_fields) == 1, "only one root field expected"

    # get root node and check permissions
    qs = models.__dict__[id.type_name].objects.all()
    node = qs.filter(id=id.node_id).first()
    if not node:
        return None
    check_module_node_access(info, node, ModuleAccessLevel.Read)

    return read_module_node(info, node)


def read_module_node(
    info: Info, node: ModuleNode, root_fragment: Optional[InlineFragment] = None
) -> ModuleNode:
    """
    Reads a module node in an optimized way (that assumes tree-shaped retrieval).
    Any nodes not in the tree will be fetched by the standard strawberry resolver.

    TODO @Broken: read module node assumes default filters (i.e. deleted_at=None)
    """
    logger.debug("module.read_node", node=node)

    # figure out which nodes to query (naively filter by selected fields)
    root_selections = _inline_fragments((root_fragment or info.selected_fields[0]).selections)
    included = {models.ProjectVersion, models.File, models.Statement}
    for field in _inline_fragments(root_selections, recursive=True):
        base_model = _BASE_MODEL_BY_CAMEL_FIELD.get(field.name)
        if base_model and base_model not in included:
            included.add(base_model)
    excluded = MNT_BY_BASE_MODEL_CLASS.keys() - included

    # collect them
    tree = packer.collect_node(node, excluded=excluded)
    logger.debug(
        "module.read_node.resolve",
        id=node.id,
        node=node,
        nodes=len(tree.visited),
        excluded=excluded,
    )
    # 'resolve' them into a proxy models.ModuleNode (with all relevant fields set/cached)
    resolved_node = _resolve_node(node, root_selections, children=tree.visited_by_parent)
    logger.debug(
        "module.read_node.done", id=node.id, node=node, nodes=len(tree.visited), excluded=excluded
    )

    return resolved_node


def _resolve_node(
    n: models.ModuleNode,
    selections: list[SelectedField],
    children: dict[UUID, list[models.ModuleNode]],
) -> ModuleNode:
    """Map relevant selections to the node's fields"""
    model_name = n._meta.object_name

    proxy_n = n.__class__(id=n.id)  # always keep id

    # map selections to module node columns or relations
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
            parent_field = first((k for k in n._meta.fields if getattr(n, k.column) == n.parent_id))
            # set id and relation field
            setattr(proxy_n, parent_field.attname, n.parent_id)
            setattr(proxy_n, parent_field.name, parent_field.related_model(id=n.parent_id))
            continue

        # relational field
        django_field = n._meta.get_field(py_name)
        # error on invalid relations to models outside the module tree
        if (
            django_field.related_model not in packer.MNT_BY_BASE_MODEL_CLASS
            and django_field.related_model not in ALLOWED_EXTERNAL_RELATIONS
        ):
            raise ValueError(
                f"invalid relation {django_field.related_model} for {model_name}.{py_name}"
            )
        inner_selections = _inline_fragments(f.selections)
        # for 1:1 relations use id only proxy
        if django_field.one_to_one or django_field.many_to_one:
            if django_field.related_model in ALLOWED_EXTERNAL_RELATIONS:
                # external relations are properly queried
                # this is okay because we only do this once usually (e.g. top-level project)
                setattr(proxy_n, py_name, getattr(n, py_name))
            else:
                # assumes { __typename, id } selection or similar (that's all we know here)
                assert len(inner_selections) == 2, f"bad 1:1 relation fields {inner_selections}"
                related_id = getattr(n, py_name + "_id")
                related = django_field.related_model(id=related_id) if related_id else None
                setattr(proxy_n, py_name, related)
        # for 1:n relations get children
        elif django_field.one_to_many or django_field.many_to_many:
            related = []
            # for flattened relations get all descendants (of same type)
            if (type(n), django_field.related_model) in FLATTENED_RELATIONS:
                # collect descendants of same type
                remaining = children.get(n.id, [])
                while remaining:
                    child = remaining.pop()
                    if type(child) != django_field.related_model:
                        continue
                    related.append(_resolve_node(child, inner_selections, children))
                    remaining.extend(children.get(child.id, []))
            else:
                # collect immediate children only
                for child in children.get(n.id, []):
                    if type(child) != django_field.related_model:
                        continue  # ignore children of other types
                    related.append(_resolve_node(child, inner_selections, children))
            # set children list on proxy to 'cache' it in the Django model
            if not hasattr(proxy_n, "_prefetched_objects_cache"):
                proxy_n._prefetched_objects_cache = {}
            proxy_n._prefetched_objects_cache[py_name] = StaticPrefetchedQueryset(related)
        else:
            raise ValueError(f"unexpected relation {django_field} for {model_name}.{py_name}")

    return proxy_n
