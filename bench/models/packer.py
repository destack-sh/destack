"""
Server-side mapper to translate between language and database models.

'Write' direction is language -> database, 'read' is database -> wire.
"""

from __future__ import annotations

import abc
import typing
from collections import OrderedDict, defaultdict
from dataclasses import asdict
from datetime import datetime
from typing import Optional, TypeVar
from uuid import UUID, uuid5

import pytz
from django.db import transaction
from django.db.models import Model, QuerySet

import bench.bench.core
from bench import models
from bench.bench import StatementType, TypeHint, TypeTag, wire
from bench.bench.const import DatasetBackend, RemoteObjectStatus, TypeFlag
from bench.bench.core import InterpScope, ModuleObjectType
from bench.bench.execution import RunErrorData
from bench.bench.issue import IssueKind, IssueType
from bench.bench.mutate import MMK, ModuleMutation, MutationBundle
from bench.bench.wire import ModuleTree

MOT = ModuleObjectType
ParentsT = set[MOT]
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
        raise NotImplementedError

    def unpack(self, data: NodeDataT, parent: Optional[NodeT]) -> NodeT | list[NodeT]:
        """
        Unpack the node and any relevant normalized related nodes.
        If returning a list, the first item is the main node.
        """
        raise NotImplementedError

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
    (models.Statement, lambda qs: qs.filter(deleted_at__isnull=True, commented=False)),
    (models.Field, lambda qs: qs.filter(deleted_at__isnull=True)),
]
DEFAULT_PACK_FILTER = PackMultiFilter(DEFAULT_PACK_FILTERS)

# registered packers
# some node models correspond to multiple actual module node / node data types
_node_packers_by_data: dict[typing.Type[NodeDataT], NodePacker] = {}
_node_packers_by_node: dict[tuple[typing.Type[NodeT], Optional[str]], NodePacker] = {}
BASE_MODEL_CLASS_BY_MOT: dict[MOT, typing.Type[Model]] = {}


def node_packer(
    t: MOT,
    data_t: typing.Type[NodeDataT],
    node_t: typing.Type[NodeT],
    subtype: Optional[str] = None,
):
    """Decorator to register a node packer for a given type"""

    def decorator(cls: "NodePacker"):
        if data_t in _node_packers_by_data:
            raise ValueError(
                f"packer for {data_t} already registered: {_node_packers_by_data[data_t]}"
            )
        if (node_t, subtype) in _node_packers_by_node:
            raise ValueError(
                f"packer for {node_t} already registered: {_node_packers_by_node[(node_t, subtype)]}"
            )
        packer = cls()
        _node_packers_by_data[data_t] = packer
        _node_packers_by_node[(node_t, subtype)] = packer
        if t not in BASE_MODEL_CLASS_BY_MOT:
            BASE_MODEL_CLASS_BY_MOT[t] = node_t
        elif not issubclass(node_t, BASE_MODEL_CLASS_BY_MOT[t]):  # type: ignore
            raise ValueError(f"model {node_t} is not a subclass of {BASE_MODEL_CLASS_BY_MOT[t]}")
        return cls

    return decorator


def get_node_packer(node: NodeT) -> NodePacker:
    if isinstance(node, models.Statement):
        return _node_packers_by_node[(models.Statement, node.type)]
    else:
        return _node_packers_by_node[(type(node), None)]


def pack_module(
    module: models.ProjectVersion, filter: PackFilter = DEFAULT_PACK_FILTER
) -> wire.ModuleTreeData:
    """Pack a module (convenience wrapper)"""
    packed = pack_node(module, filter=filter)
    tree = wire.ModuleTreeData(
        **packed.roots[0].__dict__, module=packed.roots[0], nodes=packed.nodes_list()
    )
    return tree


class Packed(typing.NamedTuple):
    roots: list[NodeDataT]
    nodes: dict[UUID, NodeDataT]
    visited: dict[UUID, NodeT]

    def nodes_list(self):
        return list(self.nodes.values())


def pack_node(*models: ModelT, filter: PackFilter = DEFAULT_PACK_FILTER) -> Packed:
    """Pack a node and its descendants"""
    visited: dict[UUID, NodeT] = {}
    packed: dict[UUID, NodeDataT] = OrderedDict()
    packed_by_node_t: dict[typing.Type[NodeT], list[UUID]] = defaultdict(list)
    ctx = PackContext()

    to_pack: list[ModelT] = [*models]
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
                qs = filter(qs)
                if packed_by_node_t[qs.model]:
                    qs = qs.exclude(id__in=packed_by_node_t[qs.model])
                existing_qs = querysets.get(qs.model)
                # skip if existing queryset is the same, otherwise union
                if existing_qs is None:
                    querysets[qs.model] = qs
                elif existing_qs.query != qs.query:
                    querysets[qs.model] = querysets[qs.model].union(qs)
            for node in nodes:
                visited[node.id] = node
                packed[node.id] = packer.pack(node)
                packed_by_node_t[type(node)].append(node.id)

        # get the next set of nodes to pack
        to_pack = []
        for qs in querysets.values():
            to_pack.extend(qs)

    roots = [packed[node.id] for node in models]
    return Packed(roots, packed, visited)


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


@node_packer(MOT.MODULE, wire.ModuleData, models.ProjectVersion)
class ModulePacker(NodePacker[wire.ModuleData, models.ProjectVersion]):
    def walk(self, nodes: list[models.ProjectVersion], tree: PackContext) -> list[QuerySet[Model]]:
        return [models.File.objects.filter(project_version__in=nodes)]

    def pack(self, module: models.ProjectVersion) -> wire.ModuleData:
        return wire.ModuleData(
            id=module.id,
            name=module.project.path,
            committed=module.committed,
            parent_id=None,
            created_at=module.created_at,
            updated_at=module.updated_at,
            last_edited_at=module.last_edited_at,
            last_changed_at=module.last_changed_at,
            revision=-1,  # no revision for module
        )

    def unpack(
        self, data: wire.ModuleData, parent: Optional[models.ProjectVersion]
    ) -> models.ProjectVersion:
        raise NotImplementedError


@node_packer(MOT.FILE, wire.FileData, models.File)
class FilePacker(NodePacker[wire.FileData, models.File]):
    def walk(self, nodes: list[models.File], tree: PackContext) -> list[QuerySet[Model]]:
        return [models.Statement.objects.filter(file__in=nodes)]

    def pack(self, file: models.File) -> wire.FileData:
        return wire.FileData(
            id=file.id,
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
            project_version_id=project_version_id,
            parent_file_id=parent.id if isinstance(parent, models.File) else None,
            name=data.name,
            revision=data.revision,
        )


@node_packer(MOT.STATEMENT, wire.StatementData, models.Statement)
class StatementPacker(NodePacker[wire.StatementData, models.Statement]):
    def pack(self, statement: models.Statement) -> wire.StatementData:
        return wire.StatementData(
            id=statement.id,
            parent_id=statement.parent_id if statement.parent_id else statement.file_id,
            order_key=statement.order_key,
            type=StatementType(statement.type),
            name=statement.name,
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
            project_version_id=parent.project_version_id,
            parent_statement_id=parent.id if isinstance(parent, models.Statement) else None,
            file_id=parent.id if isinstance(parent, models.File) else parent.file_id,
            revision=data.revision,
            order_key=data.order_key,
            type=data.type.value,
            name=data.name,
        )


@node_packer(MOT.STATEMENT, wire.BlankData, models.Statement, StatementType.BLANK)
class BlankPacker(StatementPacker, NodePacker[wire.BlankData, models.Statement]):
    def pack(self, statement: models.Statement) -> wire.BlankData:
        statement_data = super().pack(statement)
        return wire.BlankData(**statement_data.__dict__)

    def unpack(
        self, data: wire.BlankData, parent: models.File | models.Statement
    ) -> models.Statement:
        return super().unpack(data, parent)


@node_packer(MOT.STATEMENT, wire.TextData, models.Statement, StatementType.TEXT)
class TextPacker(StatementPacker, NodePacker[wire.TextData, models.Statement]):
    def pack(self, statement: models.Statement) -> wire.TextData:
        statement_data = super().pack(statement)
        return wire.TextData(
            **statement_data.__dict__,
            text=statement.text,
        )

    def unpack(
        self, data: wire.TextData, parent: models.File | models.Statement
    ) -> models.Statement:
        statement = super().unpack(data, parent)
        statement.text = data.text
        return statement


@node_packer(MOT.STATEMENT, wire.TypeData, models.Statement, StatementType.TYPE)
class TypePacker(StatementPacker, NodePacker[wire.TypeData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: PackContext) -> list[QuerySet[Model]]:
        return [*super().walk(nodes, tree), models.Field.objects.filter(statement__in=nodes)]

    def pack(self, statement: models.Statement) -> wire.TypeData:
        statement_data = super().pack(statement)
        return wire.TypeData(
            **statement_data.__dict__,
            description=statement.description,
            tag=TypeTag(statement.root_type_tag),
            flags=TypeFlag(statement.root_type_flags or 0),
        )

    def unpack(
        self, data: wire.TypeData, parent: models.File | models.Statement
    ) -> models.Statement:
        statement = super().unpack(data, parent)
        statement.description = data.description
        statement.root_type_tag = data.tag.value
        statement.root_type_flags = data.flags
        return statement


@node_packer(MOT.STATEMENT, wire.TaskData, models.Statement, StatementType.TASK)
class TaskPacker(StatementPacker, NodePacker[wire.TaskData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: PackContext) -> list[QuerySet[Model]]:
        return [*super().walk(nodes, tree), models.Field.objects.filter(statement__in=nodes)]

    def pack(self, statement: models.Statement) -> wire.TaskData:
        statement_data = super().pack(statement)
        return wire.TaskData(
            **statement_data.__dict__,
            description=statement.description,
            modifier=statement.modifier,
        )

    def unpack(
        self, data: wire.TaskData, parent: models.File | models.Statement
    ) -> models.Statement:
        statement = super().unpack(data, parent)
        statement.description = data.description
        statement.modifier = data.modifier
        return statement


@node_packer(MOT.STATEMENT, wire.ExpectationData, models.Statement, StatementType.EXPECTATION)
class ExpectationPacker(StatementPacker, NodePacker[wire.ExpectationData, models.Statement]):
    def pack(self, statement: models.Statement) -> wire.ExpectationData:
        statement_data = super().pack(statement)
        return wire.ExpectationData(
            **statement_data.__dict__,
            modifier=statement.modifier,
            description=statement.description,
            reference_id=statement.reference_id,
        )

    def unpack(
        self, data: wire.ExpectationData, parent: models.File | models.Statement
    ) -> models.Statement:
        statement = super().unpack(data, parent)
        statement.modifier = data.modifier
        statement.description = data.description
        statement.reference_id = data.reference_id
        return parent


@node_packer(MOT.STATEMENT, wire.CodeData, models.Statement, StatementType.CODE)
class CodePacker(StatementPacker, NodePacker[wire.CodeData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: PackContext) -> list[QuerySet[Model]]:
        return [*super().walk(nodes, tree), models.Field.objects.filter(statement__in=nodes)]

    def pack(self, statement: models.Statement) -> wire.CodeData:
        statement_data = super().pack(statement)
        return wire.CodeData(
            **statement_data.__dict__,
            modifier=statement.modifier,
            language=statement.lang,
            code=statement.code,
        )

    def unpack(
        self, data: wire.CodeData, parent: models.File | models.Statement
    ) -> models.Statement:
        statement = super().unpack(data, parent)
        statement.modifier = data.modifier
        statement.lang = data.language
        statement.code = data.code
        return statement


@node_packer(MOT.STATEMENT, wire.ModelData, models.Statement, StatementType.MODEL)
class ModelPacker(StatementPacker, NodePacker[wire.ModelData, models.Statement]):
    def pack(self, statement: models.Statement) -> wire.ModelData:
        statement_data = super().pack(statement)
        return wire.ModelData(
            **statement_data.__dict__,
            id=statement.id,
            parent_id=statement.id,
            external_name=statement.external_name,
        )

    def unpack(
        self, data: wire.ModelData, parent: models.File | models.Statement
    ) -> models.Statement:
        statement = super().unpack(data, parent)
        statement.external_name = data.external_name
        return statement


@node_packer(MOT.STATEMENT, wire.RequirementData, models.Statement, StatementType.REQUIREMENT)
class RequirementPacker(StatementPacker, NodePacker[wire.RequirementData, models.Statement]):
    def pack(self, statement: models.Statement) -> wire.RequirementData:
        statement_data = super().pack(statement)
        reference_module = (
            bench.bench.core.ModuleReference(id=statement.reference_project_id)
            if statement.reference_project_id
            else None
        )
        return wire.RequirementData(**statement_data.__dict__, reference_module=reference_module)

    def unpack(
        self, data: wire.RequirementData, parent: models.File | models.Statement
    ) -> models.Statement:
        statement = super().unpack(data, parent)
        statement.reference_project_id = data.reference_module.id if data.reference_module else None
        return statement


@node_packer(MOT.STATEMENT, wire.ValueData, models.Statement, StatementType.VALUE)
class ValuePacker(StatementPacker, NodePacker[wire.ValueData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: PackContext) -> list[QuerySet[Model]]:
        return [*super().walk(nodes, tree), models.Field.objects.filter(statement__in=nodes)]

    def pack(self, statement: models.Statement) -> wire.ValueData:
        statement_data = super().pack(statement)
        return wire.ValueData(
            **statement_data.__dict__,
            description=statement.description,
            modifier=statement.modifier,
            value=statement.value or {},
        )

    def unpack(
        self, data: wire.ValueData, parent: models.File | models.Statement
    ) -> models.Statement:
        statement = super().unpack(data, parent)
        statement.description = data.description
        statement.modifier = data.modifier
        statement.value = data.value
        return statement


@node_packer(MOT.STATEMENT, wire.DatasetData, models.Statement, StatementType.DATASET)
class DatasetPacker(StatementPacker, NodePacker[wire.DatasetData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: PackContext) -> list[QuerySet[Model]]:
        return [*super().walk(nodes, tree), models.Field.objects.filter(statement__in=nodes)]

    def pack(self, statement: models.Statement) -> wire.DatasetData:
        statement_data = super().pack(statement)
        return wire.DatasetData(
            **statement_data.__dict__,
            description=statement.description,
            modifier=statement.modifier,
            versioned=statement.dataset.versioned,
            backend=DatasetBackend(statement.dataset.backend),
            backend_id=statement.dataset.backend_id,
        )

    def unpack(
        self, data: wire.DatasetData, parent: models.File | models.Statement
    ) -> list[models.Statement | models.Dataset]:
        statement = super().unpack(data, parent)
        statement.description = data.description
        statement.modifier = data.modifier
        statement.dataset = models.Dataset(
            id=uuid5(statement.id, "dataset"),
            versioned=data.versioned,
            statement=statement,
            backend=data.backend.value,
            backend_id=data.backend_id,
        )
        return [statement, statement.dataset]


@node_packer(MOT.FIELD, wire.FieldData, models.Field)
class FieldPacker(StatementPacker, NodePacker[wire.FieldData, models.Field]):
    def pack(self, node: models.Field) -> wire.FieldData:
        return wire.FieldData(
            id=node.id,
            parent_id=node.statement_id,
            name=node.name,
            tag=TypeTag(node.tag),
            hint=TypeHint(node.hint) if node.hint else None,
            key=node.key,
            order_key=node.order_key,
            description=node.description,
            flags=node.flags,
            reference_id=node.reference_id,
            metadata=node.metadata,
            revision=node.revision,
            created_at=node.created_at,
            updated_at=node.updated_at,
            last_edited_at=node.last_edited_at,
            last_changed_at=node.last_changed_at,
        )

    def unpack(self, data: wire.FieldData, parent: models.Statement) -> models.Field:
        return models.Field(
            id=data.id,
            statement_id=data.parent_id,
            key=data.key,
            order_key=data.order_key,
            name=data.name,
            tag=data.tag.value,
            hint=data.hint.value if data.hint else None,
            description=data.description,
            flags=data.flags,
            reference_id=data.reference_id,
            metadata=data.metadata,
        )


# interp module data


@node_packer(MOT.ISSUE, wire.IssueData, models.Issue)
class IssuePacker(NodePacker[wire.IssueData, models.Issue]):
    def pack(self, issue: models.Issue) -> wire.IssueData:
        return wire.IssueData(
            id=issue.id,
            parent_id=issue.statemen_id or issue.file_id or issue.project_version_id,
            scope=InterpScope(issue.scope),
            kind=IssueKind(issue.kind),
            type=IssueType(issue.type),
            message=issue.message,
        )

    def unpack(
        self, data: wire.IssueData, parent: models.Statement | models.File | models.ProjectVersion
    ) -> models.Issue:
        if isinstance(parent, models.Statement):
            project_version_id = parent.project_version_id
            statement_id = parent.id
            file_id = parent.file_id
        elif isinstance(parent, models.File):
            project_version_id = parent.project_version_id
            statement_id = None
            file_id = parent.id
        elif isinstance(parent, models.ProjectVersion):
            project_version_id = parent.id
            statement_id = None
            file_id = None
        else:
            raise ValueError(f"unexpected parent type: {parent}")
        return models.Issue(
            id=data.id,
            project_version_id=project_version_id,
            file_id=file_id,
            statement_id=statement_id,
            scope=data.scope.value,
            kind=data.kind.value,
            type=data.type.value,
            message=data.message,
        )


@node_packer(MOT.RESOLVED_FIELD, wire.ResolvedFieldData, models.ResolvedField)
class ResolvedFieldPacker(NodePacker[wire.ResolvedFieldData, models.ResolvedField]):
    def pack(self, resolved_field: models.ResolvedField) -> wire.ResolvedFieldData:
        return wire.ResolvedFieldData(
            id=resolved_field.id,
            parent_id=resolved_field.statement_id,
            field_id=resolved_field.field_id,
        )

    def unpack(
        self, data: wire.ResolvedFieldData, parent: models.Statement
    ) -> models.ResolvedField:
        return models.ResolvedField(
            id=data.id,
            project_version_id=parent.project_version_id,
            statement_id=parent.id,
            field_id=data.field_id,
        )


# not module data


class DataPacker(typing.Generic[DataT, NodeT]):
    """Generic data packer for non-node data types"""

    def pack(self, model: ModelT) -> DataT:
        raise NotImplementedError

    def unpack(self, data: DataT) -> ModelT:
        raise NotImplementedError


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


@data_packer(wire.ExecutionFrameData, models.Execution)
class ExecutionFramePacker(DataPacker[wire.ExecutionFrameData, models.Execution]):
    def pack(self, data: models.Execution) -> wire.ExecutionFrameData:
        return wire.ExecutionFrameData(
            id=data.id,
            project_id=data.project_id,
            module_id=data.project_version_id,
            root_id=data.root_id,
            parent_id=data.parent_id,
            runnable_id=data.runnable_id,
            entered_at=data.started_at,
            exited_at=data.terminated_at,
            cached_generated_at=data.cached_generated_at,
            cached_duration=data.cached_duration,
            queue_position=None,
            inputs=data.inputs,
            outputs=data.outputs,
            error=RunErrorData.from_dict(data.error) if data.error else None,
            # additional context
            tracing_level=data.tracing_level,
            worker_id=data.worker_id,
            trigger_type=data.trigger_type,
            trigger_id=data.user_id or data.access_token_id,
        )

    def unpack(self, data: wire.ExecutionFrameData) -> models.Execution:
        if data.error:
            status = models.ExecutionStatus.Failed
        elif data.exited_at:
            status = models.ExecutionStatus.Completed
        elif data.queue_position:
            status = models.ExecutionStatus.Queued
        else:
            status = models.ExecutionStatus.Running
        # additional context
        user_id = data.trigger_id if data.trigger_type == models.ExecutionTriggerType.UI else None
        access_token_id = (
            data.trigger_id if data.trigger_type == models.ExecutionTriggerType.API else None
        )
        return models.Execution(
            id=data.id,
            project_id=data.project_id,
            project_version_id=data.module_id,
            status=status,
            root_id=data.root_id,
            parent_id=data.parent_id,
            runnable_id=data.runnable_id,
            created_at=data.entered_at,  # not sure what to pass since it's not in DB, not frame
            updated_at=datetime.utcnow().replace(tzinfo=pytz.utc),
            started_at=data.entered_at,
            terminated_at=data.exited_at,
            cached_generated_at=data.cached_generated_at,
            cached_duration=data.cached_duration,
            inputs=data.inputs,
            outputs=data.outputs,
            error=asdict(data.error) if data.error else None,
            # additional context
            tracing_level=data.tracing_level,
            worker_id=data.worker_id,
            trigger_type=data.trigger_type,
            user_id=user_id,
            access_token_id=access_token_id,
        )


@transaction.atomic(savepoint=False)
def write_mutations(
    project_v: models.ProjectVersion,
    module: ModuleTree,
    mutations: list[ModuleMutation],
    wait_for_os: bool,
):
    """
    Writes a series of module mutations to the database.
    Currently only interp and record mutations are supported.
    """
    from bench.opensearch.index import write_mutations_to_os

    mut = MutationBundle(mutations)

    for mmt, batch in mut.batch():
        if mmt.mot == MOT.RECORD:
            continue  # stored in OpenSearch only (see below)
        elif mmt.kind == MMK.TRUNCATE:
            # remove descendants of a certain type by scope
            statement_ids = [m.statement_id for m in batch if m.statement_id is not None]
            file_ids = [m.file_id for m in batch if m.file_id is not None]
            model_cls = BASE_MODEL_CLASS_BY_MOT[mmt.mot]
            if statement_ids:
                model_cls.objects.filter(statement_id__in=statement_ids).delete()
            elif file_ids:
                model_cls.objects.filter(file_id__in=file_ids).delete()
            else:
                model_cls.objects.filter(project_version_id=project_v.id).delete()
        elif mmt.kind in (MMK.CREATE, MMK.UPDATE):
            # create or update nodes in place
            # (first assemble ancestor models - no queries, just unpacking)
            nodes = unpack_nodes(project_v, module, [m.data for m in batch])
            model_cls = BASE_MODEL_CLASS_BY_MOT[mmt.mot]
            # note: this probably doesn't work yet fully, just a placeholder until we need it proper
            if mmt.kind == MMK.CREATE:
                model_cls.objects.bulk_create(nodes)
            else:
                # can probably optimize this (e.g. group by updated properties)
                for m, node in zip(batch, nodes):
                    node._state.adding = False  # ensure update
                    node.save(force_update=True, update_fields=m.properties)
            for m, node in zip(batch, nodes):
                m.thing = node  # keep node model for downstream indexing in opensearch
        elif mmt.kind == MMK.DELETE:
            model_cls = BASE_MODEL_CLASS_BY_MOT[mmt.mot]
            model_cls.objects.filter(id__in=[m.data.id for m in batch]).delete()

    write_mutations_to_os(project_v, mut.mutations, wait_for_os)
