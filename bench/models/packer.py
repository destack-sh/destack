"""
Server-side mapper to translate between language and database models.

'Write' direction is language -> database, 'read' is database -> wire.
"""

from __future__ import annotations

import abc
import typing
from collections import defaultdict
from typing import Collection, Optional, TypeVar
from uuid import UUID

from django.db import transaction
from django.db.models import Model, QuerySet

from bench import models
from bench.language import IssueType, StatementType, TypeHint, TypeTag, wire
from bench.language.const import (
    INTERP_NODE_TYPES,
    IssueKind,
    ModuleNodeType,
    RemoteObjectStatus,
    TriggerType,
)
from bench.language.mutate import MMK, ModuleMutation, MutationBundle
from bench.language.wire import ModuleTree
from bench.opensearch.index import write_session_to_os
from bench.utils.dt import utcnow_with_tz

MNT = ModuleNodeType
ParentsT = set[MNT]
NodeDataT = TypeVar("NodeDataT", bound=wire.NodeData)
NodeT = TypeVar("NodeT", bound=Model)
DataT = TypeVar("DataT")
ModelT = TypeVar("ModelT", bound=Model)


class NodePacker(typing.Generic[NodeDataT, NodeT]):
    """Module node (DB<->wire) packer."""

    def walk(self, nodes: list[NodeT], tree: "PackContext") -> list[QuerySet[Model]]:
        """Walk any descendants of the given nodes (visit or queryset)."""
        return []

    def pack(self, node: NodeT) -> NodeDataT:
        """Pack the node and any relevant normalized related nodes."""
        raise NotImplementedError(f"{self.__class__.__name__} does not support 'pack'")

    def unpack(self, data: NodeDataT, parent: Optional[NodeT]) -> NodeT | list[NodeT]:
        """
        Unpack the node and any relevant normalized related nodes.
        If returning a list, the first item is the main node.
        """
        raise NotImplementedError(f"{self.__class__.__name__} does not support 'unpack'")

    # we don't need an 'unwalk' here because child models are associated automatically


class PackContext(abc.ABC):
    # nothing here yet (if we want a visit(...), consider how it affects filtering below)
    pass


PackFilter = typing.Callable[[QuerySet[ModelT]], QuerySet[ModelT]]


class PackMultiFilter:
    def __init__(self, filters: list[tuple[typing.Type[ModelT], PackFilter]]):
        self.filters: dict[typing.Type[ModelT], list[PackFilter]] = defaultdict(list)
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


DEFAULT_PACK_FILTERS = [
    (models.File, lambda qs: qs.filter(deleted_at__isnull=True)),
    (models.Statement, lambda qs: qs.filter(deleted_at__isnull=True)),
    (models.Field, lambda qs: qs.filter(deleted_at__isnull=True)),
    (models.Tagging, lambda qs: qs.filter(deleted_at__isnull=True)),
    (models.Trigger, lambda qs: qs.filter(deleted_at__isnull=True)),
]
DEFAULT_PACK_FILTER = PackMultiFilter(DEFAULT_PACK_FILTERS)
DEFAULT_EXCLUDED = [models.Record]

# registered packers
# some node models correspond to multiple actual module node / node data types
_node_packers_by_data: dict[typing.Type[NodeDataT], NodePacker] = {}
_node_packers_by_node: dict[typing.Type[NodeT], NodePacker] = {}
BASE_MODEL_CLASS_BY_MNT: dict[MNT, typing.Type[Model]] = {}
MNT_BY_BASE_MODEL_CLASS: dict[typing.Type[Model], MNT] = {}


def node_packer(t: MNT, data_t: typing.Type[NodeDataT], node_t: typing.Type[NodeT]):
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
        if t not in BASE_MODEL_CLASS_BY_MNT:
            BASE_MODEL_CLASS_BY_MNT[t] = node_t
            MNT_BY_BASE_MODEL_CLASS[node_t] = t
        elif not issubclass(node_t, BASE_MODEL_CLASS_BY_MNT[t]):  # type: ignore
            raise ValueError(f"model {node_t} is not a subclass of {BASE_MODEL_CLASS_BY_MNT[t]}")
        return cls

    return decorator


def get_node_packer(node: NodeT) -> NodePacker:
    if isinstance(node, models.Statement):
        return _node_packers_by_node[models.Statement]
    else:
        return _node_packers_by_node[type(node)]


def pack_module(
    module: models.ProjectVersion,
    filter: PackFilter = DEFAULT_PACK_FILTER,
    excluded: Collection[type[ModelT]] = DEFAULT_EXCLUDED,
) -> wire.ModuleTreeData:
    """Pack a module (convenience wrapper)"""
    packed = pack_node(module, filter=filter, excluded=excluded)
    tree = wire.ModuleTreeData(
        **packed.roots[0].__dict__, module=packed.roots[0], nodes=packed.nodes_list()
    )
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


class _PackedCopy(typing.NamedTuple):
    roots: list[NodeDataT]
    nodes_by_id: dict[UUID, NodeDataT]
    target_ids: dict[UUID, UUID]
    target_cks: dict[UUID, UUID]
    target_ids_reversed: dict[UUID, UUID]
    target_cks_reversed: dict[UUID, UUID]

    def nodes_list(self):
        return list(self.nodes_by_id.values())


def collect_node(
    *roots: ModelT, filter: PackFilter = DEFAULT_PACK_FILTER, excluded: Collection[ModelT] = None
) -> _VisitedTree:
    """Collect a node and its descendants"""
    visited_by_id: dict[UUID, NodeT] = {}
    visited_by_node_t: dict[typing.Type[NodeT], list[UUID]] = defaultdict(list)
    visited_by_parent: dict[UUID, list[NodeT]] = defaultdict(list)
    ctx = PackContext()

    to_pack: list[ModelT] = [*roots]
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


def pack_node(
    *models: ModelT,
    filter: PackFilter = DEFAULT_PACK_FILTER,
    excluded: Collection[type[ModelT]] = DEFAULT_EXCLUDED,
) -> _Packed:
    """Pack a node and its descendants"""
    visited = collect_node(*models, filter=filter, excluded=excluded)
    nodes = {node.id: pack_node_flat(node) for node in visited.visited.values()}
    roots = [nodes[node.id] for node in visited.roots]

    return _Packed(roots, nodes, visited.visited_by_parent)


def unpack_nodes_tree(
    nodes: list[NodeDataT], parent: Optional[NodeT] = None, pre_unpacked: dict[UUID, NodeT] = None
) -> ModuleTree:
    """Unpack a node and its descendants"""
    data_tree = ModuleTree(nodes)
    unpacked_tree = ModuleTree()
    pre_unpacked = pre_unpacked or {}

    # unpack all nodes top down (breadth first)
    for node in data_tree.walk_bfs():
        packer = _node_packers_by_data[type(node)]
        node_parent = None
        if node.parent_id in pre_unpacked:
            node_parent = pre_unpacked[node.parent_id]
        elif node.parent_id in unpacked_tree.nodes:
            node_parent = unpacked_tree.nodes[node.parent_id]
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
    project_v: models.ProjectVersion, module: ModuleTree, data_nodes: list[NodeDataT]
) -> list[NodeT]:
    """Unpack a list nodes (incl. their ancestors) without DB queries"""
    unpacked_nodes = []
    ancestors_by_id = {project_v.id: project_v}
    for data in data_nodes:
        ancestors = module.get_ancestors(data.parent_id, include_self=True)
        for ancestor in reversed(ancestors):
            if ancestor.id not in ancestors_by_id:
                parent = ancestors_by_id.get(ancestor.parent_id)
                unpacked = unpack_node_flat(ancestor, parent)
                ancestors_by_id[ancestor.id] = unpacked[0]
        nodes = unpack_node_flat(data, ancestors_by_id[data.parent_id])
        unpacked_nodes.extend(nodes)
    return unpacked_nodes


def pack_node_flat(model: ModelT) -> NodeDataT:
    """Pack a node (flat)"""
    packer = get_node_packer(model)
    return packer.pack(model)


def unpack_node_flat(data: NodeDataT, parent: Optional[NodeT] = None) -> list[NodeT]:
    """Unpack a node (flat) (can return multiple nodes for normalized/related models)"""
    packer = _node_packers_by_data[type(data)]
    unpacked = packer.unpack(data, parent)
    if not isinstance(unpacked, list):
        unpacked = [unpacked]
    return unpacked


@node_packer(MNT.Module, wire.ModuleData, models.ProjectVersion)
class ModulePacker(NodePacker[wire.ModuleData, models.ProjectVersion]):
    def walk(self, nodes: list[models.ProjectVersion], tree: PackContext) -> list[QuerySet[Model]]:
        return [models.File.objects.filter(project_version__in=nodes)]

    def pack(self, module: models.ProjectVersion) -> wire.ModuleData:
        return wire.ModuleData(
            id=module.id,
            ck=module.project_id,
            name=module.project.path,
            committed=module.committed,
            parent_id=None,
            created_at=module.created_at,
            updated_at=module.updated_at,
            last_edited_at=module.last_edited_at,
            last_changed_at=module.last_changed_at,
            revision=-1,  # no revision for module
        )


@node_packer(MNT.File, wire.FileData, models.File)
class FilePacker(NodePacker[wire.FileData, models.File]):
    def walk(self, nodes: list[models.File], tree: PackContext) -> list[QuerySet[Model]]:
        return [
            models.Statement.objects.filter(file__in=nodes),
            models.Issue.objects.filter(parent_file__in=nodes),
        ]

    def pack(self, file: models.File) -> wire.FileData:
        return wire.FileData(
            id=file.id,
            ck=file.ck,
            parent_id=file.project_version_id if file.parent_id is None else file.parent_id,
            name=file.name,
            revision=file.revision,
            created_at=file.created_at,
            updated_at=file.updated_at,
            last_edited_at=file.last_edited_at,
            last_changed_at=file.last_changed_at,
        )

    def unpack(
        self, data: wire.FileData, parent: models.File | models.ProjectVersion
    ) -> models.File:
        project_version_id = (
            parent.id if isinstance(parent, models.ProjectVersion) else parent.project_version_id
        )
        return models.File(
            id=data.id,
            ck=data.ck,
            project_version_id=project_version_id,
            parent_file_id=parent.id if isinstance(parent, models.File) else None,
            name=data.name,
            revision=data.revision,
        )


@node_packer(MNT.Statement, wire.StatementData, models.Statement)
class StatementPacker(NodePacker[wire.StatementData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: "PackContext") -> list[QuerySet[Model]]:
        return [
            models.Issue.objects.filter(parent_statement__in=nodes),
            models.ResolvedField.objects.filter(statement__in=nodes),
            models.Field.objects.filter(statement__in=nodes),
            models.Tagging.objects.filter(statement__in=nodes),
            models.Trigger.objects.filter(statement__in=nodes),
            models.Record.objects.filter(statement__in=nodes),
        ]

    def pack(self, statement: models.Statement) -> wire.StatementData:
        return wire.StatementData(
            id=statement.id,
            ck=statement.ck,
            parent_id=statement.parent_id,
            order_key=statement.order_key,
            type=StatementType(statement.type),
            name=statement.name,
            heading_level=statement.heading_level,
            text=statement.text,
            flags=statement.flags,
            tag=statement.tag,
            key=statement.key,
            code=statement.code,
            value=statement.value,
            versioned=False,  # not stored yet
            reference_ck=statement.reference_ck,
            revision=statement.revision,
            created_at=statement.created_at,
            updated_at=statement.updated_at,
            last_edited_at=statement.last_edited_at,
            last_changed_at=statement.last_changed_at,
        )

    def unpack(
        self, data: wire.StatementData, parent: models.File | models.Statement
    ) -> models.Statement:
        return models.Statement(
            id=data.id,
            ck=data.ck,
            project_version_id=parent.project_version_id,
            parent_statement_id=parent.id if isinstance(parent, models.Statement) else None,
            file_id=parent.id if isinstance(parent, models.File) else parent.file_id,
            revision=data.revision,
            order_key=data.order_key,
            type=data.type.value,
            name=data.name,
            heading_level=data.heading_level,
            text=data.text,
            flags=data.flags,
            tag=data.tag,
            key=data.key,
            code=data.code,
            value=data.value,
            reference_ck=data.reference_ck,
            created_at=data.created_at,
            updated_at=data.updated_at,
            last_edited_at=data.last_edited_at,
            last_changed_at=data.last_changed_at,
        )


@node_packer(MNT.Field, wire.FieldData, models.Field)
class FieldPacker(NodePacker[wire.FieldData, models.Field]):
    def pack(self, field: models.Field) -> wire.FieldData:
        return wire.FieldData(
            id=field.id,
            ck=field.ck,
            parent_id=field.statement_id,
            name=field.name,
            tag=TypeTag(field.tag),
            hint=TypeHint(field.hint) if field.hint else None,
            key=field.key,
            order_key=field.order_key,
            text=field.text,
            flags=field.flags,
            reference_ck=field.reference_ck,
            metadata=field.metadata,
            revision=field.revision,
            created_at=field.created_at,
            updated_at=field.updated_at,
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
            tag=data.tag.value,
            hint=data.hint.value if data.hint else None,
            text=data.text,
            flags=data.flags,
            reference_ck=data.reference_ck,
            metadata=data.metadata,
        )


@node_packer(MNT.Trigger, wire.TriggerData, models.Trigger)
class TriggerPacker(NodePacker[wire.TriggerData, models.Trigger]):
    def pack(self, trigger: models.Trigger) -> wire.TriggerData:
        return wire.TriggerData(
            id=trigger.id,
            ck=trigger.ck,
            parent_id=trigger.statement_id,
            type=trigger.type,
            active=trigger.active,
            mapping=trigger.mapping,
            schedule_type=trigger.schedule_type,
            timezone=trigger.timezone,
            interval=trigger.interval,
            cron=trigger.cron,
            runnable_ck=trigger.runnable_ck,
            scope_ck=trigger.scope_ck,
            revision=trigger.revision,
            created_at=trigger.created_at,
            updated_at=trigger.updated_at,
            last_edited_at=trigger.last_edited_at,
            last_changed_at=trigger.last_changed_at,
        )

    def unpack(self, data: wire.TriggerData, parent: models.Statement) -> models.Trigger:
        return models.Trigger(
            id=data.id,
            ck=data.ck,
            statement_id=data.parent_id,
            type=data.type,
            active=data.active,
            mapping=data.mapping,
            schedule_type=data.schedule_type,
            timezone=data.timezone,
            interval=data.interval,
            cron=data.cron,
            runnable_ck=data.runnable_ck,
            scope_ck=data.scope_ck,
        )


@node_packer(MNT.Tagging, wire.TaggingData, models.Tagging)
class TaggingPacker(NodePacker[wire.TaggingData, models.Tagging]):
    def pack(self, tagging: models.Tagging) -> wire.TaggingData:
        return wire.TaggingData(
            id=tagging.id,
            ck=tagging.ck,
            parent_id=tagging.statement_id,
            key=tagging.key,
            reference_ck=tagging.reference_ck,
            metadata=tagging.metadata,
            revision=tagging.revision,
            created_at=tagging.created_at,
            updated_at=tagging.updated_at,
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
            metadata=data.metadata,
        )


@node_packer(MNT.Record, wire.RecordData, models.Record)
class RecordPacker(NodePacker[wire.RecordData, models.Record]):
    def pack(self, record: models.Record) -> wire.RecordData:
        return wire.RecordData(
            id=record.id,
            ck=record.ck,
            parent_id=record.statement_id,
            value=record.value,
            revision=record.revision,
            created_at=record.created_at,
            updated_at=record.updated_at,
            last_edited_at=record.last_edited_at,
            last_changed_at=record.last_edited_at,
        )

    def unpack(self, data: wire.RecordData, parent: models.Statement) -> models.Record:
        return models.Record(
            id=data.id,
            ck=data.ck,
            statement_id=parent.id,
            statement_ck=parent.ck,
            value=data.value,
            revision=data.revision,
            created_at=data.created_at,
            updated_at=data.updated_at,
            last_edited_at=data.last_edited_at,
        )


# interp module data


@node_packer(MNT.Issue, wire.IssueData, models.Issue)
class IssuePacker(NodePacker[wire.IssueData, models.Issue]):
    def pack(self, issue: models.Issue) -> wire.IssueData:
        return wire.IssueData(
            id=issue.id,
            ck=issue.ck,
            parent_id=issue.parent_statement_id or issue.parent_file_id or issue.project_version_id,
            kind=IssueKind(issue.kind),
            type=IssueType(issue.type),
            message=issue.message,
        )

    def unpack(
        self, data: wire.IssueData, parent: models.Statement | models.File | models.ProjectVersion
    ) -> models.Issue:
        if isinstance(parent, models.Statement):
            project_version_id = parent.project_version_id
            parent_statement_id = parent.id
            parent_file_id = None
        elif isinstance(parent, models.File):
            project_version_id = parent.project_version_id
            parent_statement_id = None
            parent_file_id = parent.id
        elif isinstance(parent, models.ProjectVersion):
            project_version_id = parent.id
            parent_statement_id = None
            parent_file_id = None
        else:
            raise ValueError(f"{data} has unexpected parent type: {parent} ({parent.id})")
        return models.Issue(
            id=data.id,
            ck=data.ck,
            project_version_id=project_version_id,
            parent_file_id=parent_file_id,
            parent_statement_id=parent_statement_id,
            kind=data.kind.value,
            type=data.type.value,
            message=data.message,
        )


@node_packer(MNT.ResolvedField, wire.ResolvedFieldData, models.ResolvedField)
class ResolvedFieldPacker(NodePacker[wire.ResolvedFieldData, models.ResolvedField]):
    def pack(self, resolved_field: models.ResolvedField) -> wire.ResolvedFieldData:
        return wire.ResolvedFieldData(
            id=resolved_field.id,
            ck=resolved_field.ck,
            parent_id=resolved_field.statement_id,
            field_ck=resolved_field.field_ck,
        )

    def unpack(
        self, data: wire.ResolvedFieldData, parent: models.Statement
    ) -> models.ResolvedField:
        return models.ResolvedField(
            id=data.id,
            ck=data.ck,
            project_version_id=parent.project_version_id,
            statement_id=parent.id,
            field_ck=data.field_ck,
        )


# detached module data


class DataPacker(typing.Generic[DataT, NodeT]):
    """Generic data packer for non-node data types"""

    def pack(self, model: ModelT) -> DataT:
        raise NotImplementedError(f"{self.__class__.__name__} does not support pack")

    def unpack(self, data: DataT) -> ModelT:
        raise NotImplementedError(f"{self.__class__.__name__} does not support unpack")


_data_packers_by_data: dict[typing.Type[DataT], "DataPacker"] = {}
_data_packers_by_model: dict[typing.Type[ModelT], "DataPacker"] = {}


def data_packer(data_t: typing.Type[DataT], model_t: typing.Type[ModelT]):
    """Decorator to register a data packer for a given type"""

    def decorator(cls: "DataPacker"):
        if data_t in _data_packers_by_data:
            raise ValueError(
                f"packer for {data_t} already registered: {_data_packers_by_data[data_t]}"
            )
        if model_t and model_t in _data_packers_by_model:
            raise ValueError(
                f"packer for {model_t} already registered: {_data_packers_by_model[model_t]}"
            )
        packer = cls()
        _data_packers_by_data[data_t] = packer
        if model_t:
            _data_packers_by_model[model_t] = packer
        return cls

    return decorator


def get_data_packer(data_t: typing.Type[DataT]) -> "DataPacker":
    """Get the data packer for a given data type"""
    return _data_packers_by_data[data_t]


def pack_data(model: ModelT) -> DataT:
    """Pack any non-node data type"""
    packer = _data_packers_by_model[type(model)]
    return packer.pack(model)


def unpack_data(data: DataT) -> ModelT:
    """Unpack any non-node data type"""
    packer = _data_packers_by_data[type(data)]
    return packer.unpack(data)


@data_packer(wire.RemoteObjectData, models.RemoteObject)
class RemoteObjectPacker(DataPacker[wire.RemoteObjectData, models.RemoteObject]):
    def pack(self, data: models.RemoteObject) -> wire.RemoteObjectData:
        return wire.RemoteObjectData(
            id=data.id,
            sha512=data.sha512,
            content_length=data.content_length,
            content_type=data.content_type,
            name=data.name,
            status=RemoteObjectStatus(data.status),
        )

    def unpack(self, data: wire.RemoteObjectData) -> models.RemoteObject:
        return models.RemoteObject(
            id=data.id,
            sha512=data.sha512,
            content_length=data.content_length,
            content_type=data.content_type,
            name=data.name,
            status=data.status.value,
        )


@data_packer(wire.SecretData, models.Secret)
class SecretPacker(DataPacker[wire.SecretData, models.Secret]):
    def pack(self, data: models.Secret) -> wire.SecretData:
        return wire.SecretData(
            id=data.id,
            sha512=data.sha512,
            value=data.value,
        )

    def unpack(self, data: wire.SecretData) -> models.Secret:
        return models.Secret(
            id=data.id,
            sha512=data.sha512,
            value=data.value,
        )


@data_packer(wire.SessionData, models.Session)
class SessionPacker(DataPacker[wire.SessionData, models.Session]):
    def pack(self, data: models.Session) -> wire.SessionData:
        return wire.SessionData(
            id=data.id,
            module_id=data.project_version_id,
            opened_at=data.opened_at,
            closed_at=data.closed_at,
            metadata=data.metadata,
            trigger_id=data.trigger_id,
            trigger_type=data.trigger_type,
        )

    def unpack(self, data: wire.SessionData) -> models.Session:
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
            project_version_id=data.module_id,
            opened_at=data.opened_at,
            closed_at=data.closed_at,
            metadata=data.metadata,
            trigger_type=data.trigger_type,
            trigger_access_token_id=access_token_id,
            trigger_user_id=user_id,
            trigger_id=trigger_id,
        )


@data_packer(wire.RunData, models.Run)
class RunPacker(DataPacker[wire.RunData, models.Run]):
    def pack(self, model: models.Run) -> wire.RunData:
        return wire.RunData(
            id=model.id,
            project_id=model.project_id,
            worker_node_id=model.worker_node_id,
            worker_process_id=model.worker_process_id,
            module_id=model.project_version_id,
            session_id=model.session_id,
            trigger_type=model.trigger_type,
            trigger_id=model.trigger_id,
            root_id=model.root_id,
            parent_id=model.parent_id,
            runnable_id=model.runnable_id,
            runnable_ck=model.runnable_ck,
            runnable_type=StatementType(model.runnable_type) if model.runnable_type else None,
            created_at=model.created_at,
            updated_at=model.updated_at,
            scheduled_at=model.scheduled_at,
            started_at=model.started_at,
            terminated_at=model.terminated_at,
            status=model.status,
            inputs=model.inputs,
            outputs=model.outputs,
            metadata=model.metadata,
            error=wire.RunErrorData.from_dict(model.error) if model.error else None,
        )

    def unpack(self, data: wire.RunData) -> models.Run:
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
            project_id=data.project_id,
            project_version_id=data.module_id,
            worker_node_id=data.worker_node_id,
            worker_process_id=data.worker_process_id,
            session_id=data.session_id,
            trigger_type=data.trigger_type,
            trigger_access_token_id=access_token_id,
            trigger_user_id=user_id,
            trigger_id=trigger_id,
            root_id=data.root_id,
            parent_id=data.parent_id,
            runnable_id=data.runnable_id,
            runnable_ck=data.runnable_ck,
            runnable_type=data.runnable_type.value,
            created_at=data.created_at,
            updated_at=utcnow_with_tz(),
            scheduled_at=data.scheduled_at,
            started_at=data.started_at,
            terminated_at=data.terminated_at,
            status=data.status,
            inputs=data.inputs,
            outputs=data.outputs,
            metadata=data.metadata,
            error=data.error.to_dict() if data.error else None,
        )


@data_packer(wire.WorkerSetData, models.WorkerSet)
class WorkerSetPacker(DataPacker[wire.WorkerSetData, models.WorkerSet]):
    def pack(self, model: models.WorkerSet) -> wire.WorkerSetData:
        return wire.WorkerSetData(
            id=model.id,
            project_id=model.project_id,
            region=model.region,
            profile=model.profile,
            sleeping=model.sleeping,
            status=model.status,
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
            project_id=data.project_id,
            region=data.region,
            profile=data.profile,
            sleeping=data.sleeping,
            status=data.status,
            desired_replicas=data.desired_replicas,
            target_replicas=data.target_replicas,
            available_replicas=data.available_replicas,
            ready_replicas=data.ready_replicas,
            created_at=data.created_at,
            updated_at=data.updated_at,
            last_active_at=data.last_active_at,
        )


@transaction.atomic(savepoint=False)
def write_mutations(
    project_v: models.ProjectVersion,
    module: ModuleTree,
    mutations: list[ModuleMutation],
    wait_for_os: bool,
):
    """
    Writes a series of module mutations to the database AND mutates the given module.
    Currently only interp and record mutations are supported.
    """
    from bench.opensearch.index import write_mutations_to_os

    mut = MutationBundle(mutations)
    module_data = pack_node_flat(project_v)

    for mmt, batch in mut.batched_apply(module, module_data):
        if mmt.kind == MMK.TRUNCATE:
            # remove descendants of a certain type by scope
            statement_ids = [m.statement_id for m in batch if m.statement_id is not None]
            file_ids = [m.file_id for m in batch if m.file_id is not None]
            model_cls = BASE_MODEL_CLASS_BY_MNT[mmt.mnt]
            if statement_ids:
                if hasattr(model_cls, "statement"):
                    model_cls.objects.filter(statement_id__in=statement_ids).delete()
                else:
                    model_cls.objects.filter(parent_statement_id__in=statement_ids).delete()
            elif file_ids:
                if hasattr(model_cls, "file"):
                    model_cls.objects.filter(file_id__in=file_ids).delete()
                else:
                    model_cls.objects.filter(parent_file_id__in=file_ids).delete()
            else:
                model_cls.objects.filter(project_version_id=project_v.id).delete()
        elif mmt.kind in (MMK.CREATE, MMK.UPDATE):
            # create or update nodes in place
            # (first assemble ancestor models - no queries, just unpacking)
            nodes = unpack_nodes(project_v, module, [m.data for m in batch])
            model_cls = BASE_MODEL_CLASS_BY_MNT[mmt.mnt]
            if mmt.kind == MMK.CREATE:
                model_cls.objects.bulk_create(nodes)
            else:  # MMK.UPDATE
                # should probably optimize this (i.e. compile into single query)
                for m, node in zip(batch, nodes):
                    node._state.adding = False  # ensure update
                    node.save(force_update=True, update_fields=m.properties)
            for m, node in zip(batch, nodes):
                m.thing = node  # keep node model for downstream indexing in opensearch
        elif mmt.kind == MMK.DELETE:
            model_cls = BASE_MODEL_CLASS_BY_MNT[mmt.mnt]
            model_cls.objects.filter(id__in=[m.data.id for m in batch]).delete()

    write_mutations_to_os(project_v, mut.mutations, wait=wait_for_os)


@transaction.atomic(savepoint=False)
def write_session(
    project_v: models.ProjectVersion,
    session: Optional[wire.SessionData],
    runs: list[wire.RunData],
    logs: list[wire.LogEntryData],
):
    """
    Writes a session and relevant runs and logs to the database.
    """
    if session:
        session = unpack_data(session)
        models.Session.objects.bulk_create(
            [session],
            update_conflicts=True,
            unique_fields=["id"],
            update_fields=["updated_at", "opened_at", "closed_at", "metadata"],
        )
    runs_models = [unpack_data(r) for r in runs]
    models.Run.objects.bulk_create(
        runs_models,
        update_conflicts=True,
        unique_fields=["id"],
        update_fields=[
            "status",
            "updated_at",
            "started_at",
            "terminated_at",
            "inputs",
            "outputs",
            "error",
            "metadata",
        ],
    )

    write_session_to_os(project_v, session, runs, logs)


INTERP_MODEL_TYPES = tuple(BASE_MODEL_CLASS_BY_MNT[mnt] for mnt in INTERP_NODE_TYPES)
