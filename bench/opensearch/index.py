from uuid import UUID

import structlog

import bench.opensearch.type as os
from bench.bench.mutate import ModuleMutation
from bench.opensearch import mapping, mirror
from bench.opensearch.client import os_client
from bench.opensearch.type import IndexType

logger = structlog.get_logger(__name__)

DOCUMENTS_BY_INDEX = {
    IndexType.GLOBAL: [
        mirror.Owner,
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
        mapping.Record,  # only the static parts
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
        for field_name, field in doc_class.fields().items():
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

    os_client.indices.create(
        index=index_name,
        body={
            "settings": {"index": {"number_of_shards": 1, "number_of_replicas": 0}},
            "mappings": {"dynamic": "strict", "properties": mappings},
        },
    )


def write_mutations_to_os(project_id: UUID, mutations: list[ModuleMutation]):
    """Writes any relevant mutations to OpenSearch."""
    pass
