from __future__ import annotations

from django.db import transaction

from bench import models
from bench.bench.mutate import MMK, ModuleMutation, MutationBundle
from bench.models.packer import BASE_MODEL_CLASS_BY_MOT
from bench.opensearch.index import write_mutations_to_os


@transaction.atomic(savepoint=False)
def write_mutations(project_v: models.ProjectVersion, mutations: list[ModuleMutation]):
    """
    Writes a series of module mutations to the database.
    Currently only interp and record mutations are supported.
    """

    mut = MutationBundle(mutations)

    for mmt, batch in mut.batched():
        if mmt.kind == MMK.TRUNCATE:
            model_cls = BASE_MODEL_CLASS_BY_MOT[mmt.mot]
            statement_ids = [m.statement_id for m in batch if m.statement_id is not None]
            file_ids = [m.file_id for m in batch if m.file_id is not None]
            if statement_ids:
                model_cls.objects.filter(id__in=statement_ids).delete()
            elif file_ids:
                model_cls.objects.filter(id__in=file_ids).delete()
            else:
                model_cls.objects.filter(project_version_id=project_v.id).delete()
        elif mmt.kind == MMK.CREATE:
            raise NotImplementedError
        elif mmt.kind == MMK.UPDATE:
            raise NotImplementedError
        elif mmt.kind == MMK.DELETE:
            raise NotImplementedError

    write_mutations_to_os(project_v.project_id, mut.mutations)
