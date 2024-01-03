"""
Server-side mapper to translate between language and database models.

'Write' direction is language -> database, 'read' is database -> wire.
"""

from __future__ import annotations

import abc
import typing
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
from typing import Collection, Optional, TypeVar
from uuid import UUID

import structlog
from betterproto.lib.google.protobuf import Struct as BetterprotoStruct
from django.db import transaction
from django.db.models import F, Model, QuerySet
from django.db.models.expressions import RawSQL

from bench import models
from bench.language import TypeTag
from bench.language.const import (
    HOST_NODE_TYPES,
    INTERP_NODE_TYPES,
    EditKind,
    NodeType,
    RunStatus,
    TriggerType,
)
from bench.language.module import NODE_CLASS_BY_NODE_TYPE, EditBundle, NodeTree, to_bench_metatype
from bench.proto import wire, wiring
from bench.proto.wire import EditData
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import to_uuid
from bench.utils.utils import flatten

ParentsT = set[NodeType]
NodeDataT = TypeVar("NodeDataT", bound=wire.BaseNodeData)
NodeT = TypeVar("NodeT", bound=Model)
DataT = TypeVar("DataT")
ModelT = TypeVar("ModelT", bound=Model)

logger = structlog.get_logger(__name__)


class NodePacker(typing.Generic[NodeDataT, NodeT]):
    """Module node (DB<->wire) packer."""

    def walk(self, nodes: list[NodeT], tree: "PackContext") -> list[QuerySet[Model]]:
        """Walk any descendants of the given nodes (visit or queryset)."""
        return []

    def pack(self, node: NodeT) -> NodeDataT:
        """Pack the node and any relevant normalized related nodes."""
        raise NotImplementedError(f"{self.__class__.__name__} does not support 'pack'")

    def unpack(self, data: NodeDataT, parent: Optional[NodeT]) -> NodeT:
        """Unpack the node."""
        raise NotImplementedError(f"{self.__class__.__name__} does not support 'unpack'")

    # we don't need an 'unwalk' here because child models are associated automatically


class PackContext(abc.ABC):
    # nothing here yet (if we want a visit(...), consider how it affects filtering below)
    pass


PackFilter = typing.Callable[[QuerySet[ModelT]], QuerySet[ModelT]]


class PackMultiFilter:
    def __init__(self, filters: Collection[tuple[typing.Type[ModelT], PackFilter]]):
        self.filters: dict[typing.Type[ModelT], Collection[PackFilter]] = defaultdict(list)
        for type, filter in filters:
            self.filters[type].append(filter)

    def extend(self, *filters: tuple[typing.Type[ModelT], PackFilter]) -> "PackMultiFilter":
        """Return a new filter with the given filters added."""
        new_filters = [*filters]
        for type, fs in self.filters.items():
            for f in fs:
                new_filters.append((type, f))
        return PackMultiFilter(new_filters)

    def filter(self, type: ModelT, callable: PackFilter):
        self.filters[type].append(callable)

    def __call__(self, qs: QuerySet[Model]) -> QuerySet[Model]:
        applicable_filters = self.filters.get(qs.model, [])
        for filter in applicable_filters:
            qs = filter(qs)
        return qs


NODE_MODELS: tuple[typing.Type[ModelT], ...] = (
    models.File,
    models.Statement,
    models.Field,
    models.Trigger,
    models.Tagging,
)


def get_default_pack_filters(
    deleted_at: Optional[None, datetime, Collection[datetime]]
) -> PackMultiFilter:
    if deleted_at is None:
        filters = tuple(
            (model, lambda qs: qs.filter(deleted_at__isnull=True)) for model in NODE_MODELS
        )
    elif isinstance(deleted_at, datetime):
        filters = tuple(
            (model, lambda qs: qs.filter(deleted_at=deleted_at)) for model in NODE_MODELS
        )
    elif isinstance(deleted_at, list):
        filters = tuple(
            (model, lambda qs: qs.filter(deleted_at__in=deleted_at)) for model in NODE_MODELS
        )
    else:
        raise ValueError(f"invalid deleted_at: {deleted_at}")
    return PackMultiFilter(filters)


DEFAULT_PACK_FILTER = get_default_pack_filters(deleted_at=None)
DEFAULT_EXCLUDED = ()

# registered packers
# some node models correspond to multiple actual module node / node data types
_node_packers_by_data: dict[typing.Type[NodeDataT], NodePacker] = {}
_node_packers_by_node: dict[typing.Type[NodeT], NodePacker] = {}
MODEL_CLASS_BY_NODE_TYPE: dict[NodeType, typing.Type[Model]] = {}
NODE_TYPE_BY_MODEL_CLASS: dict[typing.Type[Model], NodeType] = {}


def node_packer(t: NodeType, data_t: typing.Type[NodeDataT], node_t: typing.Type[NodeT]):
    """Decorator to register a node packer for a given type"""

    def decorator(cls: "NodePacker"):
        if data_t in _node_packers_by_data:
            raise ValueError(
                f"packer for {data_t} already registered: {_node_packers_by_data[data_t]}"
            )
        if node_t in _node_packers_by_node:
            raise ValueError(
                f"packer for {node_t} already registered: {_node_packers_by_node[node_t]}"
            )
        packer = cls()
        _node_packers_by_data[data_t] = packer
        _node_packers_by_node[node_t] = packer
        if t not in MODEL_CLASS_BY_NODE_TYPE:
            MODEL_CLASS_BY_NODE_TYPE[t] = node_t
            NODE_TYPE_BY_MODEL_CLASS[node_t] = t
        elif not issubclass(node_t, MODEL_CLASS_BY_NODE_TYPE[t]):  # type: ignore
            raise ValueError(f"model {node_t} is not a subclass of {MODEL_CLASS_BY_NODE_TYPE[t]}")
        return cls

    return decorator


def get_node_packer(node: NodeT) -> NodePacker:
    if isinstance(node, models.Statement):
        return _node_packers_by_node[models.Statement]
    else:
        return _node_packers_by_node[type(node)]


def pack_module_host(
    module: models.BenchVersion,
    filter: PackFilter = DEFAULT_PACK_FILTER,
    excluded: Collection[NodeType] = DEFAULT_EXCLUDED,
) -> wire.ModuleTreeData:
    """Pack a module (convenience wrapper)"""
    packed = pack_node_host(module, filter=filter, excluded=excluded)
    wrapped_nodes = [wiring.wrap_some_node(n) for n in packed.nodes_by_id.values()]
    tree = wire.ModuleTreeData(module=packed.roots[0], nodes=wrapped_nodes)
    return tree


class _VisitedTree(typing.NamedTuple):
    roots: list[NodeT]
    visited: dict[UUID, NodeT]
    visited_by_parent: dict[Optional[UUID], list[NodeT]]


class _Packed(typing.NamedTuple):
    roots: list[NodeDataT]
    nodes_by_id: dict[UUID, NodeDataT]
    visited_by_parent: dict[Optional[UUID], list[NodeT]]

    def nodes_list(self):
        return list(self.nodes_by_id.values())


@dataclass
class _PackedCopy:
    roots: list[NodeDataT]
    nodes_by_id: dict[UUID, NodeDataT]
    target_ids: dict[UUID, UUID]
    target_cks: dict[UUID, UUID]
    target_ids_reversed: dict[UUID, UUID]
    target_cks_reversed: dict[UUID, UUID]

    def nodes_list(self):
        return list(self.nodes_by_id.values())


def collect_node_host(
    *roots: ModelT,
    filter: PackFilter = DEFAULT_PACK_FILTER,
    excluded: Collection[NodeType] = None,
    recurse_flat_root: bool = True,
) -> _VisitedTree:
    """
    Collect a node and its host descendants.
    If the roots are at a flattened level (e.g. file), we also collect their descendants.
    """
    assert all(isinstance(n, NodeType) for n in excluded or ()), f"invalid excluded: {excluded}"
    excluded = {
        MODEL_CLASS_BY_NODE_TYPE[t] for t in excluded if t in MODEL_CLASS_BY_NODE_TYPE or ()
    }
    visited_by_id: dict[UUID, NodeT] = {}
    visited_by_node_t: dict[typing.Type[NodeT], list[UUID]] = defaultdict(list)
    visited_by_parent: dict[UUID, list[NodeT]] = defaultdict(list)
    ctx = PackContext()
    to_pack: list[ModelT] = [*roots]

    if recurse_flat_root:
        # collect descendants at the root level
        for root in roots:
            root_node_type = NODE_TYPE_BY_MODEL_CLASS[type(root)]
            if (
                root_node_type not in (NodeType.FILE, NodeType.STATEMENT)
                or excluded
                and type(root) in excluded
            ):
                continue
            # queryset for recursive parent_<node_type>_id descendants
            query = """
            WITH RECURSIVE descendants(id, parent_{type}_id) AS (
                SELECT id, parent_{type}_id
                FROM bench_{type}
                WHERE id = ANY(%s)
                UNION ALL
                SELECT bench_{type}.id, bench_{type}.parent_{type}_id
                FROM bench_{type}
                INNER JOIN descendants ON descendants.id = bench_{type}.parent_{type}_id
            )
            SELECT DISTINCT id
             FROM descendants
            """.format(
                type=root_node_type.value.lower()
            )
            qs = MODEL_CLASS_BY_NODE_TYPE[root_node_type]._base_manager
            qs = qs.filter(id__in=RawSQL(query, ([root.id],)))
            qs = filter(qs)
            to_pack.extend(qs)

    while to_pack:
        # assemble different packers and nodes by type
        packers: dict[NodePacker, list[ModelT]] = defaultdict(list)
        for node in to_pack:
            packer = get_node_packer(node)
            packers[packer].append(node)

        # walk each packer
        # merge querysets of the same model
        querysets: dict[typing.Type[ModelT], QuerySet[ModelT]] = {}
        for packer, nodes in packers.items():
            for qs in packer.walk(nodes, ctx):
                if excluded and qs.model in excluded:
                    continue
                qs = filter(qs)
                if visited_by_node_t[qs.model]:
                    qs = qs.exclude(id__in=visited_by_node_t[qs.model])
                existing_qs = querysets.get(qs.model)
                # skip if existing queryset is the same, otherwise union
                if existing_qs is None:
                    querysets[qs.model] = qs
                elif existing_qs.query != qs.query:
                    querysets[qs.model] = querysets[qs.model].union(qs)
            for node in nodes:
                visited_by_id[node.id] = node
                visited_by_node_t[type(node)].append(node.id)
                visited_by_parent[node.parent_id].append(node)

        # get the next set of nodes to pack
        to_pack = []
        for qs in querysets.values():
            to_pack.extend(qs)

    roots = [visited_by_id[node.id] for node in roots]
    return _VisitedTree(roots, visited_by_id, visited_by_parent)


def pack_node_host(
    *models: ModelT,
    filter: PackFilter = DEFAULT_PACK_FILTER,
    excluded: Collection[NodeType] = DEFAULT_EXCLUDED,
) -> _Packed:
    """Pack a node and its host descendants"""
    visited = collect_node_host(*models, filter=filter, excluded=excluded)
    nodes = {node.id: pack_node_flat(node) for node in visited.visited.values()}
    roots = [nodes[node.id] for node in visited.roots]

    return _Packed(roots, nodes, visited.visited_by_parent)


def unpack_nodes_tree(
    nodes: list[NodeDataT], parent: Optional[NodeT] = None, pre_unpacked: dict[UUID, NodeT] = None
) -> NodeTree:
    """Unpack a node and its descendants"""
    data_tree = NodeTree(nodes)
    unpacked_tree = NodeTree()
    pre_unpacked = pre_unpacked or {}

    # unpack all nodes top down (breadth first)
    for node in data_tree.walk_bfs():
        packer = _node_packers_by_data[type(node)]
        node_parent = None
        if node.parent_id in pre_unpacked:
            node_parent = pre_unpacked[node.parent_id]
        elif node.parent_id in unpacked_tree.nodes_by_id:
            node_parent = unpacked_tree.nodes_by_id[node.parent_id]
        elif parent is not None:
            node_parent = parent

        if node.id in pre_unpacked:
            unpacked = pre_unpacked[node.id]
        else:
            if node_parent is None and node.parent_id is not None:
                raise ValueError(f"parent node for {node.parent_id} not found for {node}")
            unpacked = packer.unpack(node, node_parent)
        if isinstance(unpacked, list):
            for unpacked_node in unpacked:
                unpacked_tree.add(unpacked_node)
        else:
            unpacked_tree.add(unpacked)

    return unpacked_tree


def unpack_nodes(
    bench_v: models.BenchVersion, module: NodeTree, nodes: Collection[NodeDataT]
) -> list[NodeT]:
    """Unpack a list nodes (incl. their ancestors) without DB queries"""
    unpacked_nodes = []
    ancestors_by_id: dict[UUID, ModelT] = {bench_v.id: bench_v}
    for node_data in nodes:
        # runs aren't technically detached but sessions (their parents) are
        metatype = to_bench_metatype(node_data.metatype)
        detached = NODE_CLASS_BY_NODE_TYPE[metatype].__is_detached__ or metatype == NodeType.RUN
        # node may be detached or ancestor may already be unpacked
        node_parent_id = to_uuid(node_data.parent_id)
        if not detached and node_parent_id not in ancestors_by_id:
            ancestors = module.get_ancestors(to_uuid(node_parent_id), include_self=True)
            for ancestor in reversed(ancestors):
                ancestor_id = to_uuid(ancestor.id)
                if ancestor_id not in ancestors_by_id:
                    parent = ancestors_by_id.get(to_uuid(ancestor.parent_id))
                    unpacked = unpack_node_flat(ancestor, parent)
                    ancestors_by_id[ancestor_id] = unpacked
        node_parent = ancestors_by_id[node_parent_id] if not detached else None
        node_model = unpack_node_flat(node_data, node_parent)
        unpacked_nodes.append(node_model)
        ancestors_by_id[to_uuid(node_data.id)] = node_model
    return unpacked_nodes


def pack_node_flat(model: ModelT) -> NodeDataT:
    """Pack a node (flat)"""
    packer = get_node_packer(model)
    return packer.pack(model)


def unpack_node_flat(data: NodeDataT, parent: Optional[NodeT] = None) -> NodeT:
    """Unpack a node (flat) (can return multiple nodes for normalized/related models)"""
    packer = _node_packers_by_data[type(data)]
    return packer.unpack(data, parent)


@node_packer(NodeType.MODULE, wire.ModuleData, models.BenchVersion)
class ModulePacker(NodePacker[wire.ModuleData, models.BenchVersion]):
    def walk(self, nodes: list[models.BenchVersion], tree: PackContext) -> list[QuerySet[Model]]:
        return [models.File._base_manager.filter(bench_version__in=nodes)]

    def pack(self, module: models.BenchVersion) -> wire.ModuleData:
        return wire.ModuleData(
            metatype=wire.BenchType.MODULE,
            id=str(module.id),
            ck=str(module.bench_id),
            name=module.parent_bench.path,
            committed=module.committed,
            parent_id=None,
            created_at=module.created_at,
            updated_at=module.updated_at,
            deleted_at=module.deleted_at,
            last_edited_at=module.last_edited_at,
            last_changed_at=module.last_changed_at,
            revision=-1,  # no revision for module?
        )


@node_packer(NodeType.FILE, wire.FileData, models.File)
class FilePacker(NodePacker[wire.FileData, models.File]):
    def walk(self, nodes: list[models.File], tree: PackContext) -> list[QuerySet[Model]]:
        return [
            models.Statement._base_manager.filter(file__in=nodes),
            models.Issue._base_manager.filter(parent_file__in=nodes),
        ]

    def pack(self, file: models.File) -> wire.FileData:
        return wire.FileData(
            metatype=wire.BenchType.FILE,
            id=str(file.id),
            ck=str(file.ck),
            parent_id=str(file.bench_version_id) if file.parent_id is not None else file.parent_id,
            name=file.name,
            revision=file.revision,
            created_at=file.created_at,
            updated_at=file.updated_at,
            deleted_at=file.deleted_at,
            last_edited_at=file.last_edited_at,
            last_changed_at=file.last_changed_at,
        )

    def unpack(self, data: wire.FileData, parent: models.File | models.BenchVersion) -> models.File:
        bench_version_id = (
            parent.id if isinstance(parent, models.BenchVersion) else parent.bench_version_id
        )
        return models.File(
            id=data.id,
            ck=data.ck,
            bench_version_id=bench_version_id,
            parent_file_id=parent.id if isinstance(parent, models.File) else None,
            name=data.name,
            revision=data.revision,
            deleted_at=data.deleted_at,
        )


@node_packer(NodeType.STATEMENT, wire.StatementData, models.Statement)
class StatementPacker(NodePacker[wire.StatementData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: "PackContext") -> list[QuerySet[Model]]:
        return [
            models.Issue._base_manager.filter(parent_statement__in=nodes),
            models.ResolvedField._base_manager.filter(statement__in=nodes),
            models.Field._base_manager.filter(statement__in=nodes),
            models.Tagging._base_manager.filter(statement__in=nodes),
            models.Trigger._base_manager.filter(statement__in=nodes),
        ]

    def pack(self, statement: models.Statement) -> wire.StatementData:
        return wire.StatementData(
            metatype=wire.BenchType.STATEMENT,
            id=str(statement.id),
            ck=str(statement.ck),
            parent_id=str(statement.parent_id),
            order_key=statement.order_key,
            type=wire.StatementType[statement.type.upper()],
            name=statement.name,
            heading_level=statement.heading_level,
            text=statement.text,
            key=statement.key,
            code=statement.code,
            value=BetterprotoStruct.from_dict(statement.value) if statement.value else None,
            versioned=statement.versioned,
            revision=statement.revision,
            created_at=statement.created_at,
            updated_at=statement.updated_at,
            deleted_at=statement.deleted_at,
            last_edited_at=statement.last_edited_at,
            last_changed_at=statement.last_changed_at,
        )

    def unpack(
        self, data: wire.StatementData, parent: models.File | models.Statement
    ) -> models.Statement:
        return models.Statement(
            id=data.id,
            ck=data.ck,
            bench_version_id=parent.bench_version_id,
            parent_statement_id=parent.id if isinstance(parent, models.Statement) else None,
            file_id=parent.id if isinstance(parent, models.File) else parent.file_id,
            revision=data.revision,
            order_key=data.order_key,
            type=data.type.name,
            name=data.name,
            heading_level=data.heading_level,
            versioned=data.versioned,
            text=data.text,
            key=data.key,
            code=data.code,
            value=data.value.to_dict() if data.value else None,
            created_at=data.created_at,
            updated_at=data.updated_at,
            deleted_at=data.deleted_at,
            last_edited_at=data.last_edited_at,
            last_changed_at=data.last_changed_at,
        )


@node_packer(NodeType.FIELD, wire.FieldData, models.Field)
class FieldPacker(NodePacker[wire.FieldData, models.Field]):
    def pack(self, field: models.Field) -> wire.FieldData:
        return wire.FieldData(
            metatype=wire.BenchType.FIELD,
            id=str(field.id),
            ck=str(field.ck),
            parent_id=str(field.statement_id),
            name=field.name,
            tag=wire.TypeTag[TypeTag(field.tag).name],
            hint=wire.TypeHint[field.hint.upper()] if field.hint else None,
            key=field.key,
            order_key=field.order_key,
            text=field.text,
            flags=field.flags,
            reference_ck=str(field.reference_ck) if field.reference_ck else None,
            value=field.value.to_dict() if field.value else None,
            revision=field.revision,
            created_at=field.created_at,
            updated_at=field.updated_at,
            deleted_at=field.deleted_at,
            last_edited_at=field.last_edited_at,
            last_changed_at=field.last_changed_at,
        )

    def unpack(self, data: wire.FieldData, parent: models.Statement) -> models.Field:
        return models.Field(
            id=data.id,
            ck=data.ck,
            statement_id=data.parent_id,
            key=data.key,
            order_key=data.order_key,
            name=data.name,
            tag=data.tag.name,
            hint=data.hint.name if data.hint else None,
            text=data.text,
            flags=data.flags,
            reference_ck=data.reference_ck,
            value=data.value,
            deleted_at=data.deleted_at,
        )


@node_packer(NodeType.TRIGGER, wire.TriggerData, models.Trigger)
class TriggerPacker(NodePacker[wire.TriggerData, models.Trigger]):
    def pack(self, trigger: models.Trigger) -> wire.TriggerData:
        return wire.TriggerData(
            metatype=wire.BenchType.TRIGGER,
            id=str(trigger.id),
            ck=str(trigger.ck),
            parent_id=str(trigger.statement_id),
            type=wire.TriggerType[trigger.type.upper()],
            active=trigger.active,
            schedule_type=trigger.schedule_type,
            timezone=trigger.timezone,
            interval=trigger.interval,
            cron=trigger.cron,
            revision=trigger.revision,
            created_at=trigger.created_at,
            updated_at=trigger.updated_at,
            deleted_at=trigger.deleted_at,
            last_edited_at=trigger.last_edited_at,
            last_changed_at=trigger.last_changed_at,
        )

    def unpack(self, data: wire.TriggerData, parent: models.Statement) -> models.Trigger:
        return models.Trigger(
            id=data.id,
            ck=data.ck,
            statement_id=data.parent_id,
            type=data.type.name,
            active=data.active,
            schedule_type=data.schedule_type,
            timezone=data.timezone,
            interval=data.interval,
            cron=data.cron,
            deleted_at=data.deleted_at,
        )


@node_packer(NodeType.TAGGING, wire.TaggingData, models.Tagging)
class TaggingPacker(NodePacker[wire.TaggingData, models.Tagging]):
    def pack(self, tagging: models.Tagging) -> wire.TaggingData:
        return wire.TaggingData(
            metatype=wire.BenchType.TAGGING,
            id=str(tagging.id),
            ck=str(tagging.ck),
            parent_id=str(tagging.statement_id),
            key=tagging.key,
            reference_ck=str(tagging.reference_ck) if tagging.reference_ck else None,
            value=tagging.value.to_dict() if tagging.value else None,
            revision=tagging.revision,
            created_at=tagging.created_at,
            updated_at=tagging.updated_at,
            deleted_at=tagging.deleted_at,
            last_edited_at=tagging.last_edited_at,
            last_changed_at=tagging.last_changed_at,
        )

    def unpack(self, data: wire.TaggingData, parent: models.Statement) -> models.Tagging:
        return models.Tagging(
            id=data.id,
            ck=data.ck,
            statement_id=data.parent_id,
            key=data.key,
            reference_ck=data.reference_ck,
            value=data.value,
            deleted_at=data.deleted_at,
        )


# interp module data


@node_packer(NodeType.ISSUE, wire.IssueData, models.Issue)
class IssuePacker(NodePacker[wire.IssueData, models.Issue]):
    def pack(self, issue: models.Issue) -> wire.IssueData:
        return wire.IssueData(
            metatype=wire.BenchType.ISSUE,
            id=str(issue.id),
            ck=str(issue.ck),
            parent_id=str(issue.parent_statement_id)
            or issue.parent_file_id
            or issue.bench_version_id,
            kind=wire.IssueKind[issue.kind],
            type=wire.IssueType[issue.type],
            message=issue.message,
        )

    def unpack(
        self, data: wire.IssueData, parent: models.Statement | models.File | models.BenchVersion
    ) -> models.Issue:
        if isinstance(parent, models.Statement):
            bench_version_id = parent.bench_version_id
            parent_statement_id = parent.id
            parent_file_id = None
        elif isinstance(parent, models.File):
            bench_version_id = parent.bench_version_id
            parent_statement_id = None
            parent_file_id = parent.id
        elif isinstance(parent, models.BenchVersion):
            bench_version_id = parent.id
            parent_statement_id = None
            parent_file_id = None
        else:
            raise ValueError(f"{data} has unexpected parent type: {parent} ({parent.id})")
        return models.Issue(
            id=data.id,
            ck=data.ck,
            bench_version_id=bench_version_id,
            parent_file_id=parent_file_id,
            parent_statement_id=parent_statement_id,
            kind=data.kind.name,
            type=data.type.name,
            message=data.message,
        )


@node_packer(NodeType.RESOLVED_FIELD, wire.ResolvedFieldData, models.ResolvedField)
class ResolvedFieldPacker(NodePacker[wire.ResolvedFieldData, models.ResolvedField]):
    def pack(self, resolved_field: models.ResolvedField) -> wire.ResolvedFieldData:
        return wire.ResolvedFieldData(
            metatype=wire.BenchType.RESOLVED_FIELD,
            id=str(resolved_field.id),
            ck=str(resolved_field.ck),
            parent_id=str(resolved_field.statement_id),
            order_key=resolved_field.order_key,
            field_ck=str(resolved_field.field_ck),
        )

    def unpack(
        self, data: wire.ResolvedFieldData, parent: models.Statement
    ) -> models.ResolvedField:
        return models.ResolvedField(
            id=data.id,
            ck=data.ck,
            bench_version_id=parent.bench_version_id,
            statement_id=parent.id,
            order_key=data.order_key,
            field_ck=data.field_ck,
        )


# detached module data


class StructPacker(typing.Generic[DataT, NodeT]):
    """Generic struct packer for non-node data types"""

    def pack(self, model: ModelT) -> DataT:
        raise NotImplementedError(f"{self.__class__.__name__} does not support pack")

    def unpack(self, data: DataT) -> ModelT:
        raise NotImplementedError(f"{self.__class__.__name__} does not support unpack")


_struct_packers_by_data: dict[typing.Type[DataT], "StructPacker"] = {}
_struct_packers_by_model: dict[typing.Type[ModelT], "StructPacker"] = {}


def struct_packer(data_t: typing.Type[DataT], model_t: typing.Type[ModelT]):
    """Decorator to register a struct packer for a given type"""

    def decorator(cls: "StructPacker"):
        if data_t in _struct_packers_by_data:
            raise ValueError(
                f"packer for {data_t} already registered: {_struct_packers_by_data[data_t]}"
            )
        if model_t and model_t in _struct_packers_by_model:
            raise ValueError(
                f"packer for {model_t} already registered: {_struct_packers_by_model[model_t]}"
            )
        packer = cls()
        _struct_packers_by_data[data_t] = packer
        if model_t:
            _struct_packers_by_model[model_t] = packer
        return cls

    return decorator


def get_struct_packer(data_t: typing.Type[DataT]) -> "StructPacker":
    """Get the struct packer for a given data type"""
    return _struct_packers_by_data[data_t]


def pack_struct(model: ModelT) -> DataT:
    """Pack any non-node data type"""
    packer = _struct_packers_by_model[type(model)]
    return packer.pack(model)


def unpack_struct(data: DataT) -> ModelT:
    """Unpack any non-node data type"""
    packer = _struct_packers_by_data[type(data)]
    return packer.unpack(data)


@node_packer(NodeType.BLOB, wire.BlobData, models.Blob)
class BlobPacker(NodePacker[wire.BlobData, models.Blob]):
    def pack(self, data: models.Blob) -> wire.BlobData:
        return wire.BlobData(
            metatype=wire.BenchType.BLOB,
            id=str(data.id),
            ck=str(data.ck),
            created_at=data.created_at,
            updated_at=data.updated_at,
            deleted_at=data.deleted_at,
            last_changed_at=data.last_changed_at,
            last_edited_at=data.last_edited_at,
            revision=data.revision,
            sha512=data.sha512,
            content_length=data.content_length,
            content_type=data.content_type,
            name=data.name,
            status=wire.BlobStatus[data.status],
        )

    def unpack(self, data: wire.BlobData, parent: None) -> models.Blob:
        return models.Blob(
            id=data.id,
            ck=data.ck,
            created_at=data.created_at,
            updated_at=data.updated_at,
            deleted_at=data.deleted_at,
            last_changed_at=data.last_changed_at,
            last_edited_at=data.last_edited_at,
            revision=data.revision,
            sha512=data.sha512,
            content_length=data.content_length,
            content_type=data.content_type,
            name=data.name,
            status=data.status.name,
        )


@node_packer(NodeType.SECRET, wire.SecretData, models.Secret)
class SecretPacker(NodePacker[wire.SecretData, models.Secret]):
    def pack(self, data: models.Secret) -> wire.SecretData:
        return wire.SecretData(
            metatype=wire.BenchType.SECRET,
            id=str(data.id),
            ck=str(data.ck),
            created_at=data.created_at,
            updated_at=data.updated_at,
            deleted_at=data.deleted_at,
            last_changed_at=data.last_changed_at,
            last_edited_at=data.last_edited_at,
            revision=data.revision,
            sha512=data.sha512,
            value=data.value,
            parent_id=data.parent_id,
        )

    def unpack(self, data: wire.SecretData, parent: None) -> models.Secret:
        return models.Secret(
            id=data.id,
            ck=data.ck,
            created_at=data.created_at,
            updated_at=data.updated_at,
            deleted_at=data.deleted_at,
            last_changed_at=data.last_changed_at,
            last_edited_at=data.last_edited_at,
            revision=data.revision,
            sha512=data.sha512,
            value=data.value,
        )


@node_packer(NodeType.SESSION, wire.SessionData, models.Session)
class SessionPacker(NodePacker[wire.SessionData, models.Session]):
    def pack(self, data: models.Session) -> wire.SessionData:
        return wire.SessionData(
            metatype=wire.BenchType.SESSION,
            id=str(data.id),
            ck=str(data.ck),
            created_at=data.created_at,
            updated_at=data.updated_at,
            deleted_at=data.deleted_at,
            last_changed_at=data.last_changed_at,
            last_edited_at=data.last_edited_at,
            revision=data.revision,
            bench_id=data.bench_id,
            opened_at=data.opened_at,
            closed_at=data.closed_at,
            trigger_id=data.trigger_id,
            trigger_type=data.trigger_type,
            parent_id=data.parent_id,
        )

    def unpack(self, data: wire.SessionData, parent: None) -> models.Session:
        user_id = None
        access_token_id = None
        trigger_id = None
        if data.trigger_type == TriggerType.API:
            access_token_id = data.trigger_id
        elif data.trigger_type == TriggerType.USER:
            user_id = data.trigger_id
        elif data.trigger_type == TriggerType.TIME:
            trigger_id = data.trigger_id
        return models.Session(
            id=data.id,
            ck=data.ck,
            created_at=data.created_at,
            updated_at=data.updated_at,
            deleted_at=data.deleted_at,
            last_changed_at=data.last_changed_at,
            last_edited_at=data.last_edited_at,
            revision=data.revision,
            opened_at=data.opened_at,
            closed_at=data.closed_at,
            bench_id=data.bench_id,
            trigger_type=data.trigger_type,
            trigger_access_token_id=access_token_id,
            trigger_user_id=user_id,
            trigger_id=trigger_id,
        )


@node_packer(NodeType.RUN, wire.RunData, models.Run)
class RunPacker(NodePacker[wire.RunData, models.Run]):
    def pack(self, model: models.Run) -> wire.RunData:
        return wire.RunData(
            metatype=wire.BenchType.RUN,
            id=str(model.id),
            ck=str(model.ck),
            bench_id=str(model.bench_id) if model.bench_id else None,
            worker_node_id=model.worker_node_id,
            worker_process_id=model.worker_process_id,
            session_id=str(model.session_id) if model.session_id else None,
            trigger_type=model.trigger_type,
            trigger_id=str(model.trigger_id) if model.trigger_id else None,
            root_id=str(model.root_id) if model.root_id else None,
            parent_id=str(model.parent_id) if model.parent_id else None,
            statement_ck=str(model.statement_ck) if model.statement_ck else None,
            statement_path=model.statement_path,
            created_at=model.created_at,
            updated_at=model.updated_at,
            deleted_at=model.deleted_at,
            last_changed_at=model.last_changed_at,
            last_edited_at=model.last_edited_at,
            revision=model.revision,
            scheduled_at=model.scheduled_at,
            started_at=model.started_at,
            terminated_at=model.terminated_at,
            status=wire.RunStatus[model.status.upper()],
            inputs=BetterprotoStruct.from_dict(model.inputs) if model.inputs else None,
            outputs=BetterprotoStruct.from_dict(model.outputs) if model.outputs else None,
            error=wire.RunErrorData.from_dict(model.error) if model.error else None,
            value=BetterprotoStruct.from_dict(model.value) if model.value else None,
            access_level=model.access_level,
        )

    def unpack(self, data: wire.RunData, parent: models.Run | models.Session) -> models.Run:
        # additional context
        user_id = None
        access_token_id = None
        trigger_id = None
        if data.trigger_type == TriggerType.API:
            access_token_id = data.trigger_id
        elif data.trigger_type == TriggerType.USER:
            user_id = data.trigger_id
        elif data.trigger_type == TriggerType.TIME:
            trigger_id = data.trigger_id
        return models.Run(
            id=data.id,
            ck=data.ck,
            bench_id=data.bench_id,
            parent_run_id=data.parent_id if data.root_id else None,  # hack hack
            worker_node_id=data.worker_node_id,
            worker_process_id=data.worker_process_id,
            session_id=data.session_id,
            trigger_type=data.trigger_type,
            trigger_access_token_id=access_token_id,
            trigger_user_id=user_id,
            trigger_id=trigger_id,
            root_id=data.root_id,
            statement_ck=data.statement_ck,
            statement_path=data.statement_path,
            created_at=data.created_at,
            updated_at=data.updated_at,
            deleted_at=data.deleted_at,
            last_changed_at=data.last_changed_at,
            last_edited_at=data.last_edited_at,
            revision=data.revision,
            scheduled_at=data.scheduled_at,
            started_at=data.started_at,
            terminated_at=data.terminated_at,
            status=RunStatus[data.status.name],
            inputs=data.inputs.to_dict() if data.inputs else None,
            outputs=data.outputs.to_dict() if data.outputs else None,
            error=data.error.to_dict() if data.error else None,
            value=data.value.to_dict() if data.value else None,
            access_level=data.access_level.value if data.access_level else None,
        )


@struct_packer(wire.WorkerSetData, models.WorkerSet)
class WorkerSetPacker(StructPacker[wire.WorkerSetData, models.WorkerSet]):
    def pack(self, model: models.WorkerSet) -> wire.WorkerSetData:
        return wire.WorkerSetData(
            metatype=wire.BenchType.WORKER_SET,
            id=str(model.id),
            bench_id=str(model.bench_id),
            region=wire.ProjectRegion[model.region.upper()],
            profile=wire.WorkerProfile[model.profile.upper()],
            sleeping=model.sleeping,
            status=wire.WorkerSetStatus[model.status.upper()],
            desired_replicas=model.desired_replicas,
            target_replicas=model.target_replicas,
            available_replicas=model.available_replicas,
            ready_replicas=model.ready_replicas,
            created_at=model.created_at,
            updated_at=model.updated_at,
            last_active_at=model.last_active_at,
        )

    def unpack(self, data: wire.WorkerSetData) -> models.WorkerSet:
        return models.WorkerSet(
            id=data.id,
            bench_id=data.bench_id,
            region=data.region.name,
            profile=data.profile.name,
            sleeping=data.sleeping,
            status=data.status.name,
            desired_replicas=data.desired_replicas,
            target_replicas=data.target_replicas,
            available_replicas=data.available_replicas,
            ready_replicas=data.ready_replicas,
            created_at=data.created_at,
            updated_at=data.updated_at,
            last_active_at=data.last_active_at,
        )


REMAP_PROPERTIES: dict[tuple[NodeType, str], list[str]] = {
    (NodeType.FILE, "parent_id"): ["parent_file_id", "bench_version_id"],
    (NodeType.STATEMENT, "parent_id"): ["parent_statement_id", "file_id", "bench_version_id"],
    (NodeType.FIELD, "parent_id"): ["statement_id", "bench_version_id"],
    (NodeType.TRIGGER, "parent_id"): ["statement_id", "bench_version_id"],
    (NodeType.TAGGING, "parent_id"): ["statement_id", "bench_version_id"],
}


@transaction.atomic
def write_host_db_edits(
    bench_v: models.BenchVersion,
    source: NodeTree,
    edits: list[EditData],
    *,
    validate: bool = True,
    raise_on_apply_error: bool = True,
) -> list[NodeDataT]:
    """
    Writes module edits to the database. Returns the changed nodes (that still exist in the DB).
    """

    edits = EditBundle(edits)
    now = utcnow_with_tz()
    edited_nodes: list[NodeDataT] = []

    for (edit_kind, node_type, scope_type), edit_batch in edits.batched_apply(
        source, raise_on_error=raise_on_apply_error
    ):
        if node_type in (NodeType.RECORD,):  # can't do local edits in host..
            raise RuntimeError(f"unexpected host edit {edit_batch}: {edit_batch!r}")
        if edit_kind == EditKind.TRUNCATE:
            # remove children of a certain type by scope
            # this is a bit unwieldy...
            statement_ids = [e.statement_id for e in edit_batch if e.statement_id is not None]
            file_ids = [e.file_id for e in edit_batch if e.file_id is not None]
            model_cls = MODEL_CLASS_BY_NODE_TYPE[scope_type]
            if statement_ids:
                if hasattr(model_cls, "statement"):
                    model_cls._base_manager.filter(statement_id__in=statement_ids).delete()
                else:
                    model_cls._base_manager.filter(parent_statement_id__in=statement_ids).delete()
            elif file_ids:
                if hasattr(model_cls, "file"):
                    model_cls._base_manager.filter(file_id__in=file_ids).delete()
                else:
                    model_cls._base_manager.filter(parent_file_id__in=file_ids).delete()
            else:
                model_cls._base_manager.filter(bench_version_id=bench_v.id).delete()

        elif edit_kind == EditKind.CREATE:
            # create nodes
            nodes = unpack_nodes(bench_v, source, [e.node for e in edit_batch])
            model_cls = MODEL_CLASS_BY_NODE_TYPE[node_type]
            model_cls.objects.bulk_create(nodes)
            # reload nodes (e.g., for revisions)
            nodes = model_cls._base_manager.filter(id__in=[e.node.id for e in edit_batch])
            for e, node in zip(edit_batch, nodes):
                e.node = pack_node_flat(node)
                e.thing = node  # keep node model for downstream indexing in opensearch
            edited_nodes.extend(e.node for e in edit_batch)

        elif edit_kind in (
            EditKind.UPDATE,
            EditKind.MOVE,
            EditKind.SOFT_DELETE,
            EditKind.RESTORE,
        ):
            # update nodes in place
            nodes = unpack_nodes(bench_v, source, [e.node for e in edit_batch])
            model_cls = MODEL_CLASS_BY_NODE_TYPE[node_type]
            if edit_kind == EditKind.SOFT_DELETE:
                for node in nodes:
                    node.deleted_at = node.deleted_at or now
                cru_properties = ["deleted_at"]
            elif edit_kind == EditKind.RESTORE:
                for node in nodes:
                    node.deleted_at = None
                cru_properties = ["deleted_at"]
            else:
                for node in nodes:
                    node.updated_at = now
                    node.revision = F("revision") + 1
                cru_properties = ["updated_at", "revision"]
            # different properties may be updated, so group by properties
            nodes_by_props: dict[str, list[NodeT]] = defaultdict(list)
            for e, node in zip(edit_batch, nodes):
                properties = flatten(
                    *(REMAP_PROPERTIES.get((node_type, p), [p]) for p in e.properties or ())
                )
                properties = ";".join(properties)
                nodes_by_props[properties].append(node)
            # batch update
            for properties, nodes in nodes_by_props.items():
                properties = [p for p in properties.split(";") if p]
                # need to remap properties since edit data uses language names (see :Edit)
                properties = wire.remap_properties(node_type, properties)
                # validate changed properties (records have no validation)
                if validate and node_type != NodeType.RECORD:
                    unchanged_properties = [
                        f.name for f in model_cls._meta.fields if f.name not in properties
                    ]
                    for node in nodes:
                        node.clean_fields(exclude=unchanged_properties)
                num_updated = model_cls._base_manager.bulk_update(
                    nodes, [*properties, *cru_properties]
                )
                if num_updated != len(nodes):
                    raise ValueError(
                        f"failed to update {len(nodes)} {model_cls} ({properties}, got {num_updated})"
                    )
            # reload nodes (e.g., for revisions)
            nodes = model_cls._base_manager.filter(id__in=[e.node.id for e in edit_batch])
            for e, node in zip(edit_batch, nodes):
                e.node = pack_node_flat(node)
                e.thing = node  # keep node model for downstream indexing in opensearch
            edited_nodes.extend(e.node for e in edit_batch)

        elif edit_kind == EditKind.DELETE:
            model_cls = MODEL_CLASS_BY_NODE_TYPE[node_type]
            model_cls._base_manager.filter(id__in=[e.node.id for e in edit_batch]).delete()

    return edited_nodes


INTERP_MODEL_TYPES = tuple(MODEL_CLASS_BY_NODE_TYPE[node_type] for node_type in INTERP_NODE_TYPES)
HOST_MODEL_TYPES = tuple(
    MODEL_CLASS_BY_NODE_TYPE[node_type]
    for node_type in HOST_NODE_TYPES
    if node_type in MODEL_CLASS_BY_NODE_TYPE
)
