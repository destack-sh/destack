from __future__ import annotations

import typing
from uuid import uuid5

from django.db import transaction
from django.db.models import Q

from bench import models
from bench.bench.mutate import MMT, ModuleMutation, MutationBundle
from bench.bench.wire import InterpData
from bench.models import packer
from bench.opensearch.index import write_mutations_to_os


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
            # should probably somehow fit into our other regular packer/mutation system
            file_ids = [m.file_id for m in batch]
            statement_ids = [m.statement_id for m in batch]
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
                        models.ResolvedField(
                            id=uuid5(interp_data.statement_id, str(resolved_field_data.id)),
                            project_version_id=project_v.id,
                            statement_id=interp_data.statement_id,
                            field_id=resolved_field_data.id,
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

    write_mutations_to_os(project_v.project_id, mut.mutations)
