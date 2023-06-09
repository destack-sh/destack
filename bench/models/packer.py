"""
Server-side mapper to translate between language and database models.

'Write' direction is language -> database, 'read' is database -> wire.
"""

from __future__ import annotations

import typing
from dataclasses import asdict
from datetime import datetime
from itertools import groupby
from uuid import UUID, uuid5

import pytz
from django.db import transaction
from django.db.models import Q

from bench import models
from bench.language import StatementType, SymbolType, wire
from bench.language.const import InterpScope, StatementModifier, TypeFlag, TypeHint, TypeTag
from bench.language.mutate import MMT, NON_SEMANTIC_STATEMENT_TYPES, ModuleMutation, MutationBundle
from bench.language.wire import (
    SYMBOL_DATA_CLASS_BY_TYPE,
    FieldData,
    FileData,
    InterpData,
    StatementData,
)
from bench.models.project import ProjectVersion
from bench.runtime.common.type import ExecutionFrameData, RunErrorData
from bench.utils.fractional import generate_n_keys_between


def read_packed_module(
    project_v: ProjectVersion, exclude_non_semantic: bool = False
) -> wire.ModuleData:
    """Reads the DB module."""
    wire_module = wire.ModuleData(
        id=project_v.id, name=project_v.project.path, files=[], committed=project_v.committed
    )
    wire_files: dict[UUID, wire.FileData] = {}

    # map files
    for file in project_v.files.filter(deleted_at=None).all():
        wire_file = pack_file_flat(file, module_id=wire_module.id)
        wire_files[file.id] = wire_file
        wire_module.files.append(wire_file)

    # map statements
    statements = project_v.statements.filter(deleted_at=None, commented=False).prefetch_related(
        "fields"
    )
    if exclude_non_semantic:
        statements = statements.exclude(type__in=NON_SEMANTIC_STATEMENT_TYPES)

    for statement in statements:
        wire_statement = pack_statement(
            statement, file_id=statement.file_id, module_id=wire_module.id
        )
        wire_files[statement.file_id].statements.append(wire_statement)

    return wire_module


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


@transaction.atomic
def write_mutations(project_v: models.ProjectVersion, mutations: list[ModuleMutation]):
    """
    Writes mutations to the DB and Opensearch.
    TODO @Broken: mutations should consider ordering :OrderedMutations
    """

    mut = MutationBundle(mutations)

    # DB mutations

    # first process deletes
    if mut[MMT.DELETE_FIELD]:  # batch delete since no dependent models
        field_ids = [m.data.id for m in mut[MMT.DELETE_FIELD]]
        models.Field.objects.filter(id__in=field_ids).delete()
    if mut[MMT.DELETE_STATEMENT]:
        statement_ids = [m.statement_id for m in mut[MMT.DELETE_STATEMENT]]
        for statement in models.Statement.objects.filter(id__in=statement_ids):
            statement.delete()
    if mut[MMT.DELETE_FILE]:
        file_ids = [m.file_id for m in mut[MMT.DELETE_FILE]]
        for file in models.File.objects.filter(id__in=file_ids):
            file.delete()

    # then process creates
    if mut[MMT.CREATE_FILE]:
        for create in mut[MMT.CREATE_FILE]:
            file_data = typing.cast(FileData, create.data)
            # remove extension from file path (assumed to be .x, but not stored)
            path = file_data.path
            if "." in path:
                path = file_data.path.rsplit(".", 1)[0]
            file = project_v.create_file_from_path(path, exists_ok=True, id=file_data.id)
            if file.id != file_data.id:
                # delete old file
                file.delete()
                file.id = file_data.id
            file.save()
    model_contents_relations: list[typing.Any] = []
    if mut[MMT.CREATE_STATEMENT]:
        statements = [typing.cast(StatementData, c.data) for c in mut[MMT.CREATE_STATEMENT]]
        statements_ids = {stmt_data.id for stmt_data in statements}
        model_statements: dict[UUID, models.Statement] = {}
        # assign temporary parent, reference and order keys to statements within the batch
        temp_order_keys = generate_n_keys_between(None, None, len(statements))
        for ok, stmt_data in zip(temp_order_keys, statements):
            external_parent = (
                stmt_data.parent_id is not None and stmt_data.parent_id not in statements_ids
            )
            model_statement = models.Statement(
                id=stmt_data.id,
                project_version=project_v,
                file_id=stmt_data.file_id,
                parent_id=stmt_data.parent_id if external_parent else None,
                order_key=stmt_data.order_key if external_parent else ok,
                type=stmt_data.type.value,
                name=stmt_data.name,
                #  :StatementCodeTextReuse
                code=stmt_data.text if stmt_data.type == StatementType.COMMENT else None,
                symbol_type=stmt_data.symbol_type,
            )
            model_statements[stmt_data.id] = model_statement
            if stmt_data.type == StatementType.SYMBOL:
                new_relations = unpack_symbol(model_statement, stmt_data, flat=True)
                model_contents_relations.extend(new_relations)
        # create statements
        models.Statement.objects.bulk_create(model_statements.values())
        # map actual order key, parent and references if not external
        dirty_statements = []
        for stmt_data in statements:
            model_statement = model_statements[stmt_data.id]
            dirty = model_statement.parent_id != stmt_data.parent_id
            if dirty:
                model_statement.order_key = stmt_data.order_key
                model_statement.parent_id = stmt_data.parent_id
                dirty_statements.append(model_statement)
        models.Statement.objects.bulk_update(dirty_statements, ["order_key", "parent"])
    for m in mut[MMT.CREATE_FIELD]:
        model_contents_relations.append(
            unpack_field(m.statement_id, typing.cast(FieldData, m.data))
        )
    # create content relations
    for relation_cls, relations in groupby(model_contents_relations, key=type):
        relation_cls.objects.bulk_create(relations)

    # then process updates

    if mut[MMT.UPDATE_STATEMENT]:
        raise NotImplementedError(mut[MMT.UPDATE_STATEMENT])
    if mut[MMT.UPDATE_FIELD]:
        raise NotImplementedError(mut[MMT.UPDATE_FIELD])
    if mut[MMT.UPDATE_INTERP]:
        # delete and re-create interp data
        file_ids = [m.file_id for m in mut[MMT.UPDATE_INTERP] if m.data.scope == InterpScope.FILE]
        statement_ids = [
            m.statement_id for m in mut[MMT.UPDATE_INTERP] if m.data.scope == InterpScope.STATEMENT
        ]
        models.ResolvedField.objects.filter(statement_id__in=statement_ids).delete()
        models.Issue.objects.filter(
            Q(statement_id__in=statement_ids) | Q(file_id__in=file_ids)
        ).delete()
        # create new interp data
        issues = []
        resolved_fields = []
        for m in mut[MMT.UPDATE_INTERP]:
            interp_data = typing.cast(InterpData, m.data)
            for issue_data in interp_data.issues or []:
                issues.append(unpack_issue(issue_data, project_v.id))
            for resolved_field_data in interp_data.resolved_fields or []:
                resolved_fields.append(
                    unpack_resolved_field(
                        interp_data.statement_id, resolved_field_data, project_v.id
                    )
                )
        models.Issue.objects.bulk_create(issues)
        models.ResolvedField.objects.bulk_create(resolved_fields)

    # Opensearch mutations
    # nocheckin: implement these

    if mut[MMT.DELETE_RECORD]:  # batch delete since no dependent models
        # dataset_ids = [m.data.id for m in mut[MMT.DELETE_RECORD]]
        # models.Record.objects.filter(id__in=dataset_ids).delete()
        raise NotImplementedError
    if mut[MMT.TRUNCATE_RECORDS]:
        # statement_ids = [m.statement_id for m in mut[MMT.TRUNCATE_RECORDS]]
        # models.Record.objects.filter(statement_id__in=statement_ids).delete()
        raise NotImplementedError
    for m in mut[MMT.CREATE_RECORD]:
        # model_contents_relations.append(
        #     unpack_record(m.statement_id, typing.cast(RecordData, m.data))
        # )
        raise NotImplementedError
    if mut[MMT.UPDATE_RECORD]:
        # records = [unpack_record(m.statement_id, m.data) for m in mut[MMT.UPDATE_RECORD]]
        # models.Record.objects.bulk_update(records, ["order_key", "revision", "data"])
        raise NotImplementedError


def write_files(
    project_v: models.ProjectVersion,
    files: list[wire.FileData],
    delete_files: set[UUID] = None,
    overwrite: bool = False,
):
    if delete_files:
        for file in models.File.objects.filter(id__in=delete_files):
            file.delete()
    if overwrite:
        file_ids = [file_data.id for file_data in files]
        for file in project_v.files.filter(id__in=file_ids):
            file.delete()

    model_files: dict[UUID, models.File] = {}
    # create files
    for file_data in files:
        # remove extension from file path (assumed to be .x, but not stored)
        path = file_data.path
        if "." in path:
            path = file_data.path.rsplit(".", 1)[0]
        file = project_v.create_file_from_path(path, exists_ok=True, id=file_data.id)
        if file.id != file_data.id:
            # delete old file
            file.delete()
            file.id = file_data.id
        model_files[file_data.id] = file
        file.save()
    return model_files


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
