from __future__ import annotations

import typing
from itertools import groupby
from uuid import UUID

from django.db import transaction
from django.db.models import Q

from bench import models
from bench.language import StatementType, wire
from bench.language.const import InterpScope
from bench.language.mutate import MMT, NON_SEMANTIC_STATEMENT_TYPES, ModuleMutation, MutationBundle
from bench.language.wire import FieldData, FileData, InterpData, StatementData
from bench.models import packer
from bench.models.project import ProjectVersion
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
        wire_file = packer.pack_file_flat(file, module_id=wire_module.id)
        wire_files[file.id] = wire_file
        wire_module.files.append(wire_file)

    # map statements
    statements = project_v.statements.filter(deleted_at=None, commented=False).prefetch_related(
        "fields"
    )
    if exclude_non_semantic:
        statements = statements.exclude(type__in=NON_SEMANTIC_STATEMENT_TYPES)

    for statement in statements:
        wire_statement = packer.pack_statement(
            statement, file_id=statement.file_id, module_id=wire_module.id
        )
        wire_files[statement.file_id].statements.append(wire_statement)

    return wire_module


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
                new_relations = packer.unpack_symbol(model_statement, stmt_data, flat=True)
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
            packer.unpack_field(m.statement_id, typing.cast(FieldData, m.data))
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
                issues.append(packer.unpack_issue(issue_data, project_v.id))
            for resolved_field_data in interp_data.resolved_fields or []:
                resolved_fields.append(
                    packer.unpack_resolved_field(
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
