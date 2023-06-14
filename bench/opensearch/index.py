from uuid import UUID

import structlog

import bench.opensearch.type as os
from bench import models
from bench.bench.mutate import MOT, ModuleMutation
from bench.opensearch import mirror
from bench.opensearch.client import os_client
from bench.opensearch.type import IndexType

logger = structlog.get_logger(__name__)


class IndexError(ValueError):
    pass


DOCUMENTS_BY_INDEX = {
    IndexType.GLOBAL: [
        mirror.User,
        mirror.Organization,
        mirror.Project,
    ],
    IndexType.PROJECT: [
        mirror.File,
        mirror.Statement,
        mirror.Field,
        mirror.Tile,
        mirror.Comment,
    ],
    IndexType.DATASETS: [
        mirror.Record,  # only the static parts
    ],
    IndexType.SESSIONS: [
        mirror.Session,
        mirror.Execution,  # only the static parts
        mirror.LogEntry,
    ],
}


def _collect_fields(doc_classes: list[os.Document]) -> dict[str, os.Field]:
    fields = {}
    for doc_class in doc_classes:
        for field_name, field in doc_class.fields.items():
            existing_field = fields.get(field_name)
            if existing_field is not None and existing_field != field:
                raise ValueError(
                    f"field {field_name} defined twice with different types: {existing_field} != {field}"
                )
            fields[field_name] = field
    return fields


def create_index(index: IndexType, project_id: UUID) -> None:
    index_name = index.get_index_name(project_id)

    doc_classes = DOCUMENTS_BY_INDEX[index]
    fields = _collect_fields(doc_classes)
    mappings = {field_name: field.to_dict() for field_name, field in fields.items()}
    analyzers = {analyzer.value: definition for analyzer, definition in os.ANALYZERS.items()}

    os_client.indices.create(
        index=index_name,
        body={
            "settings": {
                "index": {"number_of_shards": 1, "number_of_replicas": 0, "knn": True},
                "analysis": {"analyzer": analyzers},
            },
            "mappings": {"dynamic": "strict", "properties": mappings},
        },
    )


def write_mutations_to_os(project_v: models.ProjectVersion, mutations: list[ModuleMutation]):
    """
    Writes any relevant mutations to OpenSearch.
    All regular DB mutations come this way (records are not stored in the DB).
    For now assumes that there is only one index per type per project/scope.
    """
    os_operations = []
    for m in mutations:
        if m.mot == MOT.RECORD:
            pass  # nocheckin: index

    if os_operations:
        os_client.bulk(os_operations)


def create_record(project_v: models.ProjectVersion, record: mirror.Record):
    index_name = IndexType.DATASETS.get_index_name(project_v.project_id)
    os_client.create(index=index_name, id=record.id, body=record.to_dict())


def update_record(project_v: models.ProjectVersion, record: mirror.Record.Partial) -> mirror.Record:
    raise NotImplementedError  # nocheckin: index


def delete_record(project_v: models.ProjectVersion, record_id: UUID) -> None:
    raise NotImplementedError  # nocheckin: index


def batch_update_records(
    project_v: models.ProjectVersion, records: list[mirror.Record.Partial]
) -> list[mirror.Record]:
    raise NotImplementedError  # nocheckin: index
