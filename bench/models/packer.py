"""
Server-side mapper to translate between language and database models.

'Write' direction is language -> database, 'read' is database -> wire.
"""

from __future__ import annotations

import abc
from collections import defaultdict
import typing
from dataclasses import asdict
from datetime import datetime
from uuid import UUID
from typing import Optional, TypeVar

import pytz
from django.db.models import Model, QuerySet

from bench import models
from bench.language import StatementType, SymbolType, wire
from bench.language.const import TypeFlag, TypeHint, TypeTag
from bench.language.wire import (
    ModuleObjectType,
    ModuleTree,
)
from bench.runtime.common.type import RunErrorData

MOT = ModuleObjectType
ParentsT = set[MOT]
NodeDataT = TypeVar("NodeDataT", bound=wire.NodeData)
NodeT = TypeVar("NodeT", bound=Model)
DataT = TypeVar("DataT")
ModelT = TypeVar("ModelT", bound=Model)


class DataPacker(typing.Generic[DataT, NodeT]):
    """Generic data packer for non-node data types"""

    def pack(self, model: ModelT) -> DataT:
        raise NotImplementedError

    def unpack(self, data: DataT) -> ModelT:
        raise NotImplementedError


class NodePacker(typing.Generic[NodeDataT, NodeT]):
    def walk(self, nodes: list[NodeT], tree: "PackContext") -> list[QuerySet[Model]]:
        """Walk any descendants of the given nodes (visit or queryset)."""
        return []

    def pack(self, node: NodeT) -> NodeDataT:
        raise NotImplementedError

    def unpack(self, data: NodeDataT, parent: Optional[NodeT]) -> NodeT:
        raise NotImplementedError


class PackContext(abc.ABC):
    def visit(self, model: ModelT, t: MOT) -> None:
        pass


# registered packers, where each MOT may have multiple packers (subtypes) per model type
_node_packers: dict[MOT, dict[typing.Type[NodeDataT], "NodePacker"]] = defaultdict(dict)
MOT_BY_DATA_CLASS: dict[typing.Type[NodeDataT], MOT] = {}


def node_packer(t: MOT, data_t: typing.Type[NodeDataT], node_t: typing.Type[NodeT]):
    """Decorator to register a node packer for a given type"""

    def decorator(cls: "NodePacker"):
        if data_t in _node_packers[t]:
            raise ValueError(
                f"packer for {t} and {data_t} already registered: {_node_packers[t][data_t]}"
            )
        if data_t in MOT_BY_DATA_CLASS:
            raise ValueError(
                f"data class {data_t} already registered for {MOT_BY_DATA_CLASS[data_t]}"
            )
        _node_packers[t][data_t] = cls
        MOT_BY_DATA_CLASS[data_t] = t
        return cls

    return decorator


def pack_node(model: ModelT) -> tuple[NodeDataT, list[NodeDataT]]:
    """Pack a node and its descendants"""
    tree = ModuleTree()
    ctx = PackContext()

    return list(tree.nodes.values())


def unpack_node(data: NodeDataT, parent: Optional[NodeT] = None) -> NodeT:
    """Unpack a node and its descendants"""
    raise NotImplementedError


def pack_node_flat(model: ModelT, mot: ModuleObjectType) -> NodeDataT:
    """Pack a node (flat)"""
    packer = _node_packers[mot][type(model)]
    return packer.pack(model)


@node_packer(MOT.MODULE, wire.ModuleData, models.ProjectVersion)
class ModulePacker(NodePacker[wire.ModuleData, models.ProjectVersion]):
    def walk(self, nodes: list[models.ProjectVersion], tree: PackContext) -> list[QuerySet[Model]]:
        return [models.File.objects.filter(project_version__in=nodes)]

    def pack(self, module: models.ProjectVersion) -> wire.ModuleData:
        return wire.ModuleData(
            id=module.id,
            parent_id=None,
            name=module.name,
            revision=module.revision,
            committed=module.committed,
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
            parent=parent if isinstance(parent, models.File) else None,
            name=data.name,
            revision=data.revision,
        )


@node_packer(MOT.STATEMENT, wire.StatementData, models.Statement)
class StatementPacker(NodePacker[wire.StatementData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: PackContext) -> list[QuerySet[Model]]:
        for node in nodes:
            if node.symbol_type is not None:
                tree.visit(node, MOT.SYMBOL)
        return []

    def pack(self, statement: models.Statement) -> wire.StatementData:
        return wire.StatementData(
            id=statement.id,
            revision=statement.revision,
            parent_id=statement.parent_id if statement.parent_id else statement.file_id,
            order_key=statement.order_key,
            type=StatementType(statement.type),
            name=statement.name,
            text=statement.text,
            symbol_type=SymbolType(statement.symbol_type) if statement.symbol_type else None,
        )

    def unpack(
        self, data: wire.StatementData, parent: models.File | models.Statement
    ) -> models.Statement:
        return models.Statement(
            id=data.id,
            revision=data.revision,
            parent_id=parent.id if isinstance(parent, models.File) else parent.id,
            file_id=parent.id if isinstance(parent, models.File) else parent.file_id,
            order_key=data.order_key,
            type=data.type.value,
            name=data.name,
            text=data.text,
            symbol_type=data.symbol_type.value if data.symbol_type else None,
        )


@node_packer(MOT.SYMBOL, wire.TypeData, models.Statement)
class TypePacker(NodePacker[wire.TypeData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: PackContext) -> list[QuerySet[Model]]:
        return [models.Field.objects.filter(statement__in=nodes)]

    def pack(self, statement: models.Statement) -> wire.TypeData:
        return wire.TypeData(
            id=statement.id,
            parent_id=statement.id,
            tag=TypeTag(statement.root_type_tag),
            flags=TypeFlag(statement.root_type_flags or 0),
        )

    def unpack(self, data: wire.TypeData, parent: models.Statement) -> models.Statement:
        parent.root_type_tag = data.tag.value
        parent.root_type_flags = data.flags.value
        return parent


@node_packer(MOT.SYMBOL, wire.TaskData, models.Statement)
class TaskPacker(NodePacker[wire.TaskData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: PackContext) -> list[QuerySet[Model]]:
        return [models.Field.objects.filter(statement__in=nodes)]

    def pack(self, statement: models.Statement) -> wire.TaskData:
        return wire.TaskData(
            id=statement.id,
            parent_id=statement.id,
            description=statement.description,
            modifier=statement.modifier,
        )

    def unpack(self, data: wire.TaskData, parent: models.Statement) -> models.Statement:
        parent.description = data.description
        parent.modifier = data.modifier
        return parent


@node_packer(MOT.SYMBOL, wire.ExpectationData, models.Statement)
class ExpectationPacker(NodePacker[wire.ExpectationData, models.Statement]):
    def pack(self, statement: models.Statement) -> wire.ExpectationData:
        return wire.ExpectationData(
            id=statement.id,
            parent_id=statement.id,
            modifier=statement.modifier,
            description=statement.description,
        )

    def unpack(self, data: wire.ExpectationData, parent: models.Statement) -> models.Statement:
        parent.description = data.description
        parent.modifier = data.modifier
        return parent


@node_packer(MOT.SYMBOL, wire.CodeData, models.Statement)
class CodePacker(NodePacker[wire.CodeData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: PackContext) -> list[QuerySet[Model]]:
        return [models.Field.objects.filter(statement__in=nodes)]

    def pack(self, statement: models.Statement) -> wire.CodeData:
        return wire.CodeData(
            id=statement.id,
            parent_id=statement.id,
            modifier=statement.modifier,
            language=statement.lang,
            code=statement.code,
        )

    def unpack(self, data: wire.CodeData, parent: models.Statement) -> models.Statement:
        parent.modifier = data.modifier
        parent.lang = data.language
        parent.code = data.code
        return parent


@node_packer(MOT.SYMBOL, wire.ModelData, models.Statement)
class ModelPacker(NodePacker[wire.ModelData, models.Statement]):
    def pack(self, statement: models.Statement) -> wire.ModelData:
        return wire.ModelData(
            id=statement.id, parent_id=statement.id, external_name=statement.external_name
        )

    def unpack(self, data: wire.ModelData, parent: models.Statement) -> models.Statement:
        parent.external_name = data.external_name
        return parent


@node_packer(MOT.SYMBOL, wire.RequirementData, models.Statement)
class RequirementPacker(NodePacker[wire.RequirementData, models.Statement]):
    def pack(self, statement: models.Statement) -> wire.RequirementData:
        return wire.RequirementData(
            id=statement.id,
            parent_id=statement.id,
            reference_module=wire.ModuleReference(id=statement.reference_project_id)
            if statement.reference_project_id
            else None,
        )

    def unpack(self, data: wire.RequirementData, parent: models.Statement) -> models.Statement:
        parent.reference_project_id = data.reference_module.id if data.reference_module else None
        return parent


@node_packer(MOT.SYMBOL, wire.ValueData, models.Statement)
class ValuePacker(NodePacker[wire.ValueData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: PackContext) -> list[QuerySet[Model]]:
        return [models.Field.objects.filter(statement__in=nodes)]

    def pack(self, statement: models.Statement) -> wire.ValueData:
        return wire.ValueData(
            id=statement.id,
            parent_id=statement.id,
            modifier=statement.modifier,
            value=statement.value,
        )

    def unpack(self, data: wire.ValueData, parent: models.Statement) -> models.Statement:
        parent.modifier = data.modifier
        parent.value = data.value
        return parent


@node_packer(MOT.SYMBOL, wire.DatasetData, models.Statement)
class DatasetPacker(NodePacker[wire.DatasetData, models.Statement]):
    def walk(self, nodes: list[models.Statement], tree: PackContext) -> list[QuerySet[Model]]:
        return [models.Field.objects.filter(statement__in=nodes)]

    def pack(self, statement: models.Statement) -> wire.DatasetData:
        return wire.DatasetData(
            id=statement.dataset.id,
            parent_id=statement.id,
            modifier=statement.modifier,
            versioned=statement.dataset.versioned,
        )

    def unpack(self, data: wire.DatasetData, parent: models.Statement) -> models.Dataset:
        parent.modifier = data.modifier
        parent.dataset = models.Dataset(
            id=data.id,
            versioned=data.versioned,
            statement=parent,
        )
        return parent.dataset


@node_packer(MOT.FIELD, wire.FieldData, models.Field)
class FieldPacker(NodePacker[wire.FieldData, models.Field]):
    def pack(self, node: models.Field) -> wire.FieldData:
        return wire.FieldData(
            id=node.id,
            parent_id=node.statement_id,
            revision=node.revision,
            name=node.name,
            tag=TypeTag(node.tag),
            hint=TypeHint(node.hint) if node.hint else None,
            key=node.key,
            order_key=node.order_key,
            description=node.description,
            flags=node.flags,
            reference_id=node.reference_id,
            metadata=node.metadata,
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


_data_packers: dict[typing.Type[DataT], "DataPacker"] = {}


def data_packer(data_t: typing.Type[DataT]):
    """Decorator to register a data packer for a given type"""

    def decorator(cls: "DataPacker"):
        if data_t in _data_packers:
            raise ValueError(f"packer for {data_t} already registered: {_data_packers[data_t]}")
        _data_packers[data_t] = cls
        return cls

    return decorator


def pack_data(model: ModelT) -> DataT:
    """Pack any non-node data type"""
    packer = _data_packers[type(model)]
    return packer.pack(model)


def unpack_data(data: DataT) -> ModelT:
    """Unpack any non-node data type"""
    packer = _data_packers[type(data)]
    return packer.unpack(data)


def unpack_issue(issue: wire.IssueData, module_id: UUID):
    return models.Issue(
        id=issue.id,
        project_version_id=module_id,
        scope=issue.scope.value,
        kind=issue.kind.value,
        type=issue.type.value,
        message=issue.message,
        file_id=issue.file_id,
        statement_id=issue.statement_id,
    )


@data_packer(wire.ExecutionFrameData)
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
