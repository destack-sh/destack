from __future__ import annotations

import typing

from django.db import transaction
from django.db.models import Q

from bench import models
from bench.language import wire
from bench.language.const import InterpScope
from bench.language.mutate import MMT, ModuleMutation, MutationBundle
from bench.language.wire import InterpData
from bench.models import packer
from bench.models.project import ProjectVersion


def read_packed_module(
    project_v: ProjectVersion, exclude_non_semantic: bool = False
) -> wire.ModuleData:
    raise NotImplementedError

    # wire_module = wire.ModuleData(
    #     id=project_v.id, name=project_v.project.path, files=[], committed=project_v.committed
    # )
    # wire_files: dict[UUID, wire.FileData] = {}
    #
    # # map files
    # for file in project_v.files.filter(deleted_at=None).all():
    #     wire_file = packer.pack_file_flat(file, module_id=wire_module.id)
    #     wire_files[file.id] = wire_file
    #     wire_module.files.append(wire_file)
    #
    # # map statements
    # statements = project_v.statements.filter(deleted_at=None, commented=False).prefetch_related(
    #     "fields"
    # )
    # if exclude_non_semantic:
    #     statements = statements.exclude(type__in=NON_SEMANTIC_STATEMENT_TYPES)
    #
    # for statement in statements:
    #     wire_statement = packer.pack_statement(
    #         statement, file_id=statement.file_id, module_id=wire_module.id
    #     )
    #     wire_files[statement.file_id].statements.append(wire_statement)
    #
    # return wire_module


@transaction.atomic(savepoint=False)
def write_mutations(project_v: models.ProjectVersion, mutations: list[ModuleMutation]):
    """
    Writes a series of module mutations to the database.
    Currently only interp and record mutations are supported.
    """

    mut = MutationBundle(mutations)

    for mmt, batch in mut.batched():
        # DB
        if mmt == MMT.UPDATE_INTERP:
            # delete and re-create interp data
            file_ids = [
                m.file_id for m in mut[MMT.UPDATE_INTERP] if m.data.scope == InterpScope.FILE
            ]
            statement_ids = [
                m.statement_id
                for m in mut[MMT.UPDATE_INTERP]
                if m.data.scope == InterpScope.STATEMENT
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
                            interp_data.parent_id, resolved_field_data, project_v.id
                        )
                    )
            models.Issue.objects.bulk_create(issues)
            models.ResolvedField.objects.bulk_create(resolved_fields)
        # Opensearch mutations
        # nocheckin: implement opensearch mutations
        elif mmt == MMT.CREATE_RECORD:
            raise NotImplementedError
        elif mmt == MMT.UPDATE_RECORD:
            raise NotImplementedError
        elif mmt == MMT.DELETE_RECORD:
            raise NotImplementedError
        elif mmt == MMT.TRUNCATE_RECORDS:
            raise NotImplementedError
