import typing
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
        mirror.ProjectVersion,
        mirror.File,
        mirror.Statement,
        mirror.Field,
        mirror.Tile,
        mirror.Comment,
    ],
    IndexType.BENCH: [
        mirror.Record,  # only the static parts
        mirror.Session,
        mirror.Execution,  # only the static parts
        mirror.LogEntry,
    ],
}

GLOBAL_INDEX_SHARDS = 5
GLOBAL_INDEX_REPLICAS = 1

BENCH_INDEX_SHARDS = 1
BENCH_INDEX_REPLICAS = 0
BENCH_MAPPING_TOTAL_FIELDS_LIMIT = 10000  # TODO @Performance: reconsider OS mapping limit

DEFAULT_FIELDS = {
    os.TYPE_DISCRIMINATOR_KEY: os.TYPE_DISCRIMINATOR_FIELD,
}


def _collect_fields(doc_classes: list[os.Document]) -> dict[str, os.Field]:
    fields = {}
    for doc_class in doc_classes:
        for field_name, field in doc_class.__fields__.items():
            existing_field = fields.get(field_name)
            if existing_field is not None and existing_field != field:
                raise ValueError(
                    f"field {field_name} defined twice with different types: {existing_field} != {field}"
                )
            fields[field_name] = field
    return fields


def _create_index(
    index_name: str,
    *,
    shards: int,
    replicas: int,
    documents: list[typing.Type[os.Document]],
) -> None:
    fields = {**DEFAULT_FIELDS, **_collect_fields(documents)}
    mappings = {field_name: field.to_dict() for field_name, field in fields.items()}
    analyzers = {analyzer.value: definition for analyzer, definition in os.ANALYZERS.items()}
    os_client.indices.create(
        index=index_name,
        body={
            "settings": {
                "index": {"number_of_shards": shards, "number_of_replicas": replicas, "knn": True},
                "analysis": {"analyzer": analyzers},
                "mapping": {"total_fields": {"limit": BENCH_MAPPING_TOTAL_FIELDS_LIMIT}},
            },
            "mappings": {"dynamic": "strict", "properties": mappings},
        },
    )


def create_global_index(name: str = None) -> None:
    _create_index(
        name or IndexType.GLOBAL.get_index_name(),
        shards=GLOBAL_INDEX_SHARDS,
        replicas=GLOBAL_INDEX_REPLICAS,
        documents=DOCUMENTS_BY_INDEX[IndexType.GLOBAL],
    )


def create_bench_index(project_id: UUID, name: str = None) -> None:
    _create_index(
        name or IndexType.BENCH.get_index_name(project_id),
        shards=BENCH_INDEX_SHARDS,
        replicas=BENCH_INDEX_REPLICAS,
        documents=DOCUMENTS_BY_INDEX[IndexType.BENCH],
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
    index_name = IndexType.BENCH.get_index_name(project_v.project_id)
    os_client.create(index=index_name, id=record.id, body=record.to_dict())


def update_record(project_v: models.ProjectVersion, record: mirror.Record.Partial) -> mirror.Record:
    raise NotImplementedError  # nocheckin: index


def delete_record(project_v: models.ProjectVersion, record_id: UUID) -> None:
    raise NotImplementedError  # nocheckin: index


def batch_update_records(
    project_v: models.ProjectVersion, records: list[mirror.Record.Partial]
) -> list[mirror.Record]:
    raise NotImplementedError  # nocheckin: index
