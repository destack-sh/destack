from typing import Optional

import structlog
from strawberry.types import Info
from strawberry.types.nodes import FragmentSpread, SelectedField
from strawberry.utils.str_converters import to_camel_case
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo
from strawberry_django_plus.utils.resolvers import async_safe

from bench import models
from bench.api.auth import check_can_read_project
from bench.api.utils import ModuleNode
from bench.models import packer

logger = structlog.get_logger(__name__)


class IdOnlyProxy:
    def __init__(self, id: GlobalID):
        self.id = id


_MODEL_FIELD_NAME_BY_GQL_NAME: dict[str, str] = {}
_GQL_FIELD_NAME_BY_MODEL_NAME: dict[str, str] = {}


def _add_field_name(name: str):
    gql_name = to_camel_case(name)
    if (
        gql_name in _MODEL_FIELD_NAME_BY_GQL_NAME
        and _MODEL_FIELD_NAME_BY_GQL_NAME[gql_name] != name
    ):
        raise ValueError(
            f"duplicate field name {gql_name} for {name} and {_MODEL_FIELD_NAME_BY_GQL_NAME[gql_name]}"
        )
    _MODEL_FIELD_NAME_BY_GQL_NAME[gql_name] = name
    _GQL_FIELD_NAME_BY_MODEL_NAME[name] = gql_name


def _collect_fields():
    for model in packer.MOT_BY_BASE_MODEL_CLASS.keys():
        for field in model._meta.fields:
            _add_field_name(field.name)
            # and related name if any
            if hasattr(field, "remote_field") and field.remote_field:
                _add_field_name(field.remote_field.name)
        for field_name in model._meta.fields_map.keys():
            _add_field_name(field_name)
        for field in model._meta.many_to_many:
            _add_field_name(field.name)


_collect_fields()


def _inline_fragments(fields: list[SelectedField | FragmentSpread]) -> list[SelectedField]:
    if not any(isinstance(f, FragmentSpread) for f in fields):
        return fields
    inlined = []
    for f in fields:
        if isinstance(f, FragmentSpread):
            inlined.extend(f.selections)
        else:
            inlined.append(f)
    return inlined


@async_safe
def read_module_node(info: Info, id: GlobalID) -> Optional[ModuleNode] | OperationInfo:
    """
    Reads a module node in an optimized way (that assumes tree-shaped retrieval).
    Any nodes not in the tree will be fetched by the strawberry resolver.
    We map all models directly to the graphql type, so we mostly bypass the strawberry resolver.

    TODO @Broken: read module node assumes default filters
    """
    assert len(info.selected_fields) == 1, "only one root field expected"

    qs = models.__dict__[id.type_name].objects.all()
    node = qs.filter(id=id.node_id).first()
    if not node:
        return None
    check_can_read_project(info, node)

    logger.debug("module.read_node", id=id, node=node)
    visited = packer.collect_node(node)  # nocheckin: filter to selected relations

    # map relevant selected fields to the visited nodes
    def _resolve(n: models.ModuleNode, selections: list[SelectedField]) -> ModuleNode:
        model_name = n._meta.object_name
        gql_type = info.schema.get_type_by_name(model_name)

        gql_props = {}
        for f in selections:
            py_name = _MODEL_FIELD_NAME_BY_GQL_NAME.get(f.name, f.name)
            # pass through non-relational fields
            if f.name == "__typename":
                gql_props[f.name] = gql_type.name
                continue
            elif f.name == "id":
                gql_props[f.name] = GlobalID(type_name=gql_type.name, node_id=str(n.id))
                continue
            elif not f.selections:
                gql_props[f.name] = getattr(n, py_name)
                continue
            # shortcut for parent (which isn't a real field)
            elif f.name == "parent":
                # nocheckin: convert this
                gql_props[f.name] = visited.visited.get(n.parent_id, IdOnlyProxy(n.parent_id))
                continue

            # relational field
            django_field = n._meta.get_field(py_name)
            # for 1:1 relations use id only proxy
            inner_selections = _inline_fragments(f.selections)
            if django_field.one_to_one or django_field.many_to_one:
                assert len(inner_selections) == 2, f"unexpected {selections} for {django_field}"
                gql_props[py_name] = IdOnlyProxy(getattr(n, py_name + "_id"))
            # for 1:n relations get children
            elif django_field.one_to_many or django_field.many_to_many:
                # error on invalid relations to fields not in the module tree
                if django_field.related_model not in packer.MOT_BY_BASE_MODEL_CLASS:
                    raise ValueError(
                        f"invalid relation {django_field.related_model} for {model_name}.{py_name}"
                    )
                # resolve children
                resolved_children = []
                for child in visited.visited_by_parent.get(n.id, []):
                    resolved_child = _resolve(child, inner_selections)
                    resolved_children.append(resolved_child)
                gql_props[py_name] = resolved_children
            else:
                raise ValueError(f"unexpected relation {django_field} for {model_name}.{py_name}")

        return gql_type(**gql_props)

    logger.debug("module.read_node.resolve", id=id, node=node, nodes=len(visited.visited))
    root_selections = _inline_fragments(info.selected_fields[0].selections)
    resolved_node = _resolve(node, root_selections)

    logger.debug("module.read_node.done", id=id, node=node, resolved_node=resolved_node)
    return resolved_node
