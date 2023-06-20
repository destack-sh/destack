import typing
from uuid import UUID, uuid4

import structlog

import bench.opensearch.type as os
from bench import models
from bench.bench import type as lang
from bench.bench import wire
from bench.bench.dataset import MAX_VERSIONED_RECORDS_TOTAL
from bench.bench.mutate import MMK, MMT, MOT, ModuleMutation
from bench.opensearch import mirror
from bench.opensearch.client import os_client
from bench.opensearch.mapping import map_to_os_field
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
    upsert: bool = False,
) -> None:
    fields = {**DEFAULT_FIELDS, **_collect_fields(documents)}
    mappings = {field_name: field.to_dict() for field_name, field in fields.items()}
    analyzers = {analyzer.value: definition for analyzer, definition in os.ANALYZERS.items()}
    logger.info(
        "os.create_index",
        index_name=index_name,
        shards=shards,
        replicas=replicas,
        fields=list(fields.keys()),
    )
    result = os_client.indices.create(
        index=index_name,
        body={
            "settings": {
                "index": {"number_of_shards": shards, "number_of_replicas": replicas, "knn": True},
                "analysis": {"analyzer": analyzers},
                "mapping": {"total_fields": {"limit": BENCH_MAPPING_TOTAL_FIELDS_LIMIT}},
            },
            "mappings": {"dynamic": "strict", "properties": mappings},
        },
        ignore=400 if upsert else 0,
    )
    # upsert if index already exists
    if result.get("acknowledged") is not True:
        if not upsert:
            raise IndexError(f"failed to create index {index_name}: {result}")
        logger.info("os.create_index.upsert", index_name=index_name)
        # update mutable settings
        os_client.indices.put_settings(
            index=index_name,
            body={"mapping": {"total_fields": {"limit": BENCH_MAPPING_TOTAL_FIELDS_LIMIT}}},
        )
        os_client.indices.put_mapping(
            index=index_name, body={"dynamic": "strict", "properties": mappings}
        )


def create_global_index(name: str = None, upsert: bool = False) -> None:
    _create_index(
        name or IndexType.GLOBAL.get_index_name(),
        shards=GLOBAL_INDEX_SHARDS,
        replicas=GLOBAL_INDEX_REPLICAS,
        documents=DOCUMENTS_BY_INDEX[IndexType.GLOBAL],
        upsert=upsert,
    )


def create_bench_index(project_id: UUID, name: str = None, upsert: bool = False) -> None:
    _create_index(
        name or IndexType.BENCH.get_index_name(project_id),
        shards=BENCH_INDEX_SHARDS,
        replicas=BENCH_INDEX_REPLICAS,
        documents=DOCUMENTS_BY_INDEX[IndexType.BENCH],
        upsert=upsert,
    )


OS_SEMANTIC_FIELD_MUTATIONS = {
    MMT.TRUNCATE_FIELDS,
    MMT.CREATE_FIELD,
    MMT.UPDATE_FIELD,
    MMT.UPDATE_FIELD_TYPE,
    MMT.DELETE_FIELD,
    MMT.TRUNCATE_RESOLVED_FIELDS,
    MMT.CREATE_RESOLVED_FIELD,
}


def write_mutations_to_os(
    project_v: models.ProjectVersion, mutations: list[ModuleMutation]
) -> None:
    """
    Writes/mirrors any relevant mutations to OpenSearch.
    All regular DB mutations come this way (records are not stored in the DB).
    """
    os_operations: list[dict] = []
    global_index_name = IndexType.GLOBAL.get_index_name()
    bench_index_name = IndexType.BENCH.get_index_name(project_v.project_id)
    mappings_dirty = False
    for m in mutations:
        # index directly as primary or secondary store
        if m.mot == MOT.RECORD or mirror.has_mirror(m.thing):
            index_name = bench_index_name if m.mot == MOT.RECORD else global_index_name
            if m.type.kind in (MMK.CREATE, MMK.UPDATE) or m.type.is_soft:
                mirrored = mirror.mirror_node(project_v, m.thing)
                op = (
                    {"index": {"_index": index_name, "_id": str(m.thing.id)}},
                    mirrored.to_dict(),
                )
                os_operations.extend(op)
            elif m.type.kind == MMK.DELETE:
                op = {"delete": {"_index": index_name, "_id": str(m.thing.id)}}
                os_operations.append(op)
        # mark field mappings as dirty if relevant
        if m.type in OS_SEMANTIC_FIELD_MUTATIONS:
            mappings_dirty = True

    if mappings_dirty:
        update_dynamic_field_mappings(project_v)

    if os_operations:
        logger.debug(
            "os.write_mutations",
            project_version=project_v,
            index=bench_index_name,
            mutations=len(mutations),
            operations=len(os_operations),
        )
        os_client.bulk(os_operations)


def update_dynamic_field_mappings(project_v: models.ProjectVersion) -> None:
    """
    Updates *all* dynamic OpenSearch field mappings for a module
    TODO @Performance: update OS field mappings more efficiently on field mutations
    """
    from bench.models import packer

    logger.info("os.update_mappings", project_version=project_v)
    source = packer.pack_module(project_v)
    module = wire.unpack_module(source, session=None)
    module.index()
    module.interp()

    data_mappings = {}
    inputs_mappings = {}
    outputs_mappings = {}
    for symbol in module.symbols_by_id.values():
        if symbol.errors:
            continue  # ignore symbols with issues
        elif isinstance(symbol, lang.Dataset):
            # all fields go into Record.data ('data' is a "dynamic" object)
            for field in symbol.resolved_fields:
                data_mappings[field.typed_key] = map_to_os_field(field).to_dict()
        elif isinstance(symbol, (lang.Task, lang.Code)):
            # inputs into Execution.inputs, outputs into Execution.outputs
            for field in symbol.inputs:
                inputs_mappings[field.typed_key] = map_to_os_field(field).to_dict()
            for field in symbol.outputs:
                outputs_mappings[field.typed_key] = map_to_os_field(field).to_dict()

    logger.info(
        "os.update_mappings.done",
        project_version=project_v,
        data_mappings=len(data_mappings),
        inputs_mappings=len(inputs_mappings),
        outputs_mappings=len(outputs_mappings),
    )
    mappings = {
        "data": {"type": "object", "dynamic": "strict", "properties": data_mappings},
        "inputs": {"type": "object", "dynamic": "strict", "properties": inputs_mappings},
        "outputs": {"type": "object", "dynamic": "strict", "properties": outputs_mappings},
    }
    index_name = IndexType.BENCH.get_index_name(project_v.project_id)
    os_client.indices.put_mapping(index=index_name, body={"properties": mappings})


def create_record(project_v: models.ProjectVersion, record: mirror.Record):
    index_name = IndexType.BENCH.get_index_name(project_v.project_id)
    os_record = os_client.create(index=index_name, id=record.id, body=record.to_dict())
    record.revision = os_record["_version"]
    return record


def update_record(
    project_v: models.ProjectVersion, record: mirror.Record.Partial
) -> mirror.Record.Partial:
    index_name = IndexType.BENCH.get_index_name(project_v.project_id)
    os_record = os_client.update(
        index=index_name,
        id=record.id,
        body={"doc": record.to_dict()},
    )
    record.revision = os_record["_version"]
    return record


def delete_record(project_v: models.ProjectVersion, record_id: UUID) -> None:
    index_name = IndexType.BENCH.get_index_name(project_v.project_id)
    os_client.delete(index=index_name, id=record_id)


def batch_update_records(
    project_v: models.ProjectVersion, records: list[mirror.Record.Partial]
) -> list[mirror.Record.Partial]:
    index_name = IndexType.BENCH.get_index_name(project_v.project_id)
    os_operations = []
    for record in records:
        os_operations.append({"update": {"_index": index_name, "_id": str(record.id)}})
        os_operations.append({"doc": record.to_dict()})
    os_records = os_client.bulk(os_operations)
    for i, os_record in enumerate(os_records["items"]):
        records[i].revision = os_record["update"]["_version"]
    return records


def batch_duplicate_records(
    source_project_v: models.ProjectVersion,
    target_project_v: models.ProjectVersion,
    new_dataset_ids: dict[str, str],
    batch_size: int = 512,
):
    """
    Duplicates all documents in the given datasets with new target dataset ids.
    Assigns new records ids on the way.
    TODO @Performance @Robustness: move batch duplicate to a background job
    """
    index_name = IndexType.BENCH.get_index_name(source_project_v.project_id)

    query = {
        # dataset_id must be in new_dataset_ids.keys(), deleted_at must not exist
        "query": {
            "bool": {
                "must": [
                    {"terms": {"dataset_id": list(new_dataset_ids.keys())}},
                    {"bool": {"must_not": {"exists": {"field": "deleted_at"}}}},
                ]
            }
        },
    }
    num_total_documents = os_client.count(index=index_name, body=query)["count"]
    if num_total_documents > MAX_VERSIONED_RECORDS_TOTAL:
        raise ValueError(
            f"{source_project_v} has {num_total_documents} documents (limit={MAX_VERSIONED_RECORDS_TOTAL})"
        )
    if num_total_documents == 0:
        logger.info(
            "os.batch_duplicate_records.skip",
            source=source_project_v,
            num_total_documents=num_total_documents,
        )
        return

    log = logger.bind(
        source=source_project_v,
        target=target_project_v,
        new_dataset_ids=new_dataset_ids,
        batch_size=batch_size,
        total_documents=num_total_documents,
    )
    log.info("os.batch_duplicate_records.start")

    # create a PIT to read from
    pit = os_client.create_point_in_time(index=index_name, keep_alive="2m")
    query["pit"] = {"id": pit["pit_id"], "keep_alive": "2m"}

    num_duplicated = 0
    while True:
        response = os_client.search(body=query, sort=["_doc"], size=batch_size)
        hits = response["hits"]["hits"]
        if not hits:
            break

        log.debug("os.batch_duplicate_records.batch", cumulative=num_duplicated, current=len(hits))
        os_operations = []
        for hit in hits:
            document = hit["_source"]
            dataset_id = document["dataset_id"]
            target_dataset_id = new_dataset_ids[dataset_id]
            document["dataset_id"] = target_dataset_id
            os_operations.append({"index": {"_index": index_name, "_id": str(uuid4())}})
            os_operations.append(document)
        num_duplicated += len(hits)

        os_client.bulk(os_operations)

        last_hit = hits[-1]
        last_sort_values = last_hit["sort"]
        query["search_after"] = last_sort_values

    # TODO @Cleanup: delete PIT after use (delete_point_in_time doesn't work?)
    log.info("os.batch_duplicate_records.done")
