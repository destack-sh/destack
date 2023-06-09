"""
Server-side mapper to translate between language and database models.

'Write' direction is language -> database, 'read' is database -> wire.
"""

from __future__ import annotations

import typing
from dataclasses import asdict
from datetime import datetime
from uuid import UUID, uuid5

import pytz

from bench import models
from bench.language import StatementType, SymbolType, wire
from bench.language.const import StatementModifier, TypeFlag, TypeHint, TypeTag
from bench.language.mutate import NON_SEMANTIC_STATEMENT_TYPES
from bench.language.wire import SYMBOL_DATA_CLASS_BY_TYPE, FieldData, FileData, StatementData
from bench.runtime.common.type import ExecutionFrameData, RunErrorData


def pack_file_nested(file: models.File, exclude_non_semantic: bool = False) -> FileData:
    """Reads a file and its statements (and their contents)."""
    statements = (
        file.statements.filter(deleted_at=None, commented=False)
        .select_related("reference")
        .prefetch_related("fields")
    )
    if exclude_non_semantic:
        statements = statements.exclude(type__in=NON_SEMANTIC_STATEMENT_TYPES)

    wire_file = pack_file_flat(file, module_id=file.project_version_id)
    wire_file.statements = [
        pack_statement(s, file_id=file.id, module_id=file.project_version_id) for s in statements
    ]
    return wire_file


def pack_statement_nested(statement: models.Statement) -> list[StatementData]:
    """Reads a statement and all its children."""
    wire_statements = [
        pack_statement(s, file_id=statement.file_id, module_id=statement.project_version_id)
        for s in statement.descendants
    ]
    return wire_statements


# (all module contents are used for tracking changes)
def pack_flat(
    obj: models.File | models.Statement | models.Field,
) -> wire.FileData | wire.StatementData | wire.FieldData:
    """Read a DB object into a wire object without any children."""
    if isinstance(obj, models.File):
        return pack_file_flat(obj, module_id=obj.project_version_id)
    elif isinstance(obj, models.Statement):
        return pack_statement(obj, file_id=obj.file_id, module_id=obj.project_version_id, flat=True)
    elif isinstance(obj, models.Field):
        return pack_field(obj)
    else:
        raise ValueError(f"unexpected obj: {obj}")


def pack_file_flat(file: models.File, module_id: UUID) -> wire.FileData:
    return wire.FileData(
        id=file.id,
        module_id=module_id,
        path=file.path,
        statements=[],
        revision=file.revision,
    )


def unpack_resolved_field(statement_id: UUID, field: FieldData, module_id: UUID):
    return models.ResolvedField(
        id=uuid5(statement_id, str(field.id)),
        project_version_id=module_id,
        statement_id=statement_id,
        field_id=field.id,
    )


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


def pack_statement(
    statement: models.Statement, file_id: UUID, module_id: UUID, flat: bool = False
) -> wire.StatementData:
    """Reads a database statement into a wire statement."""
    # map reference into wire-able reference (convert module-external ref to statement path)
    data = wire.StatementData(
        id=statement.id,
        module_id=module_id,
        file_id=file_id,
        revision=statement.revision,
        parent_id=statement.parent_id,
        order_key=statement.order_key,
        type=StatementType(statement.type),
        name=statement.name,
        fqn=None,
        text=statement.code if statement.type == StatementType.COMMENT else None,
        symbol_type=SymbolType(statement.symbol_type) if statement.symbol_type else None,
        modifier=StatementModifier(statement.modifier) if statement.modifier else None,
    )
    if statement.type == StatementType.SYMBOL:
        pack_symbol(statement, data, flat=flat)
    return data


def pack_symbol(statement: models.Statement, data: wire.StatementData, flat: bool) -> None:
    """Reads a database statement's symbol into a wire statement."""
    data.description = statement.description

    data_cls = SYMBOL_DATA_CLASS_BY_TYPE[statement.symbol_type]
    fields = None
    if issubclass(data_cls, wire.HasTypeData) and not flat:
        fields = [pack_field(node) for node in statement.fields.filter(deleted_at=None)]

    if statement.symbol_type == SymbolType.TYPE:
        data.symbol = wire.TypeData(
            tag=TypeTag(statement.root_type_tag),
            flags=TypeFlag(statement.root_type_flags or 0),
            fields=fields,
        )
    elif statement.symbol_type == SymbolType.TASK:
        data.symbol = wire.TaskData(fields=fields)
    elif statement.symbol_type == SymbolType.EXPECTATION:
        data.symbol = wire.ExpectationData()
    elif statement.symbol_type == SymbolType.CODE:
        data.symbol = wire.CodeData(fields=fields, lang=statement.lang, code=statement.code)
    elif statement.symbol_type == SymbolType.REQUIREMENT:
        data.symbol = wire.RequirementData(
            reference_module=wire.ModuleReference(id=statement.reference_project_version_id),
        )
    elif statement.symbol_type == SymbolType.MODEL:
        data.symbol = wire.ModelData(
            external_name=statement.external_name,
        )
    elif statement.symbol_type == SymbolType.VALUE:
        data.symbol = wire.ValueData(
            tag=TypeTag(statement.root_type_tag),
            flags=TypeFlag(statement.root_type_flags),
            fields=fields,
            value=statement.value,
        )
    elif statement.symbol_type == SymbolType.DATASET:
        data.symbol = wire.DatasetData(fields=fields, records=None, length=None)


def unpack_symbol(
    statement: models.Statement, data: wire.StatementData, flat: bool
) -> list[typing.Any]:
    """Writes a wire statement's symbol into DB models."""
    relations = []
    statement.modifier = data.modifier.value if data.modifier else None
    statement.description = data.description  # every symbol has a description
    if isinstance(data.symbol, wire.HasTypeData):
        statement.root_type_tag = data.symbol.tag.value if data.symbol.tag else None
        statement.root_type_flags = data.symbol.flags
        if not flat:
            fields = [unpack_field(statement.id, node) for node in data.symbol.fields]
            relations.extend(fields)
    if isinstance(data.symbol, wire.CodeData):
        statement.lang = data.symbol.lang
        statement.code = data.symbol.code
    if isinstance(data.symbol, wire.ModelData):
        statement.external_name = data.symbol.external_name
    if isinstance(data.symbol, wire.RequirementData):
        statement.reference_project_version_id = data.symbol.reference_module.id
    if isinstance(data.symbol, wire.ValueData):
        statement.value = data.symbol.value
    if isinstance(data.symbol, wire.DatasetData):
        raise NotImplementedError  # what do?
    return relations


def pack_field(node: models.Field) -> wire.FieldData:
    return wire.FieldData(
        id=node.id,
        revision=node.revision,
        name=node.name,
        tag=TypeTag(node.tag),
        hint=TypeHint(node.hint) if node.hint else None,
        statement_id=node.statement_id,
        key=node.key,
        order_key=node.order_key,
        description=node.description,
        flags=node.flags,
        reference_id=node.reference_id,
    )


def unpack_field(statement_id: UUID, node: wire.FieldData) -> models.Field:
    return models.Field(
        id=node.id,
        statement_id=statement_id,
        key=node.key,
        order_key=node.order_key,
        name=node.name,
        tag=node.tag.value,
        hint=node.hint.value if node.hint else None,
        description=node.description,
        flags=node.flags,
        reference_id=node.reference_id,
    )


def pack_execution_frame(frame: ExecutionFrameData) -> models.Execution:
    if frame.error:
        status = models.ExecutionStatus.Failed
    elif frame.exited_at:
        status = models.ExecutionStatus.Completed
    elif frame.queue_position:
        status = models.ExecutionStatus.Queued
    else:
        status = models.ExecutionStatus.Running
    # additional context
    user_id = frame.trigger_id if frame.trigger_type == models.ExecutionTriggerType.UI else None
    access_token_id = (
        frame.trigger_id if frame.trigger_type == models.ExecutionTriggerType.API else None
    )
    return models.Execution(
        id=frame.id,
        project_id=frame.project_id,
        project_version_id=frame.module_id,
        status=status,
        root_id=frame.root_id,
        parent_id=frame.parent_id,
        runnable_id=frame.runnable_id,
        created_at=frame.entered_at,  # not sure what to pass since it's not in DB, not frame
        updated_at=datetime.utcnow().replace(tzinfo=pytz.utc),
        started_at=frame.entered_at,
        terminated_at=frame.exited_at,
        cached_generated_at=frame.cached_generated_at,
        cached_duration=frame.cached_duration,
        inputs=frame.inputs,
        outputs=frame.outputs,
        error=asdict(frame.error) if frame.error else None,
        # additional context
        tracing_level=frame.tracing_level,
        worker_id=frame.worker_id,
        trigger_type=frame.trigger_type,
        user_id=user_id,
        access_token_id=access_token_id,
    )


def unpack_execution_frame(frame: models.Execution) -> ExecutionFrameData:
    return ExecutionFrameData(
        id=frame.id,
        project_id=frame.project_id,
        module_id=frame.project_version_id,
        root_id=frame.root_id,
        parent_id=frame.parent_id,
        runnable_id=frame.runnable_id,
        entered_at=frame.started_at,
        exited_at=frame.terminated_at,
        cached_generated_at=frame.cached_generated_at,
        cached_duration=frame.cached_duration,
        queue_position=None,
        inputs=frame.inputs,
        outputs=frame.outputs,
        error=RunErrorData.from_dict(frame.error) if frame.error else None,
        # additional context
        tracing_level=frame.tracing_level,
        worker_id=frame.worker_id,
        trigger_type=frame.trigger_type,
        trigger_id=frame.user_id or frame.access_token_id,
    )
