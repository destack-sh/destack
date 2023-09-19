import typing
from uuid import UUID

import structlog

import bench.opensearch.core as os
from bench import language as lang
from bench import models
from bench.language import wire
from bench.language.mutate import MMK, MMT, MNT, ModuleMutation
from bench.language.run import HasRun
from bench.opensearch import mirror
from bench.opensearch.client import os_client
from bench.opensearch.core import IndexType
from bench.opensearch.mapping import map_to_os_field

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
        mirror.Run,  # only the static parts
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
    tokenizers = {
        tokenizer.value: definition for tokenizer, definition in os.CUSTOM_TOKENIZERS.items()
    }
    analyzers = {analyzer.value: definition for analyzer, definition in os.CUSTOM_ANALYZERS.items()}
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
                "analysis": {"tokenizer": tokenizers, "analyzer": analyzers},
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
        # close index
        os_client.indices.close(index=index_name)
        # update mutable settings
        os_client.indices.put_settings(
            index=index_name,
            body={
                "analysis": {"tokenizer": tokenizers, "analyzer": analyzers},
                "mapping": {"total_fields": {"limit": BENCH_MAPPING_TOTAL_FIELDS_LIMIT}},
            },
        )
        os_client.indices.put_mapping(
            index=index_name, body={"dynamic": "strict", "properties": mappings}
        )
        # reopen index
        os_client.indices.open(index=index_name)


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
    MMT.CREATE_FIELD,
    MMT.UPDATE_FIELD,
    MMT.UPDATE_FIELD_TYPE,
    MMT.DELETE_FIELD,
    MMT.TRUNCATE_RESOLVED_FIELDS,
    MMT.CREATE_RESOLVED_FIELD,
}

BENCH_INDEXED_MNTS = (MNT.Record,)
BENCH_INDEXED_MODELS = (models.Record,)


def get_index_for_mnt(mnt: MNT, project_id: UUID) -> str:
    if mnt in BENCH_INDEXED_MNTS:
        return IndexType.BENCH.get_index_name(project_id)
    else:
        return IndexType.GLOBAL.get_index_name()


def write_mutations_to_os(
    project_v: models.ProjectVersion, mutations: list[ModuleMutation], *, wait: bool = False
) -> None:
    """
    Writes/mirrors any relevant mutations to OpenSearch.
    All regular DB mutations come this way.
    """
    global_index = IndexType.GLOBAL.get_index_name()
    bench_index = IndexType.BENCH.get_index_name(project_v.project_id)

    # mut state
    ops: list[dict] = []
    field_mappings_dirty: list[bool] = [False]  # for closure

    def _flush():
        if ops:
            logger.debug(
                "os.write_mutations",
                project_version=project_v,
                index=bench_index,
                mutations=len(mutations),
                operations=len(ops),
            )
            ret = os_client.bulk(ops, refresh="wait_for" if wait else False)
            if ret.get("errors"):
                raise RuntimeError(f"failed to write mutations to OpenSearch: {ret['items'][:5]}")

        if field_mappings_dirty[0]:  # if needed, must happen before any other mutations
            update_dynamic_field_mappings(project_v)

        ops.clear()
        field_mappings_dirty[0] = False

    for m in mutations:
        # mark field mappings as dirty if relevant mutation
        if m.type in OS_SEMANTIC_FIELD_MUTATIONS:
            field_mappings_dirty[0] = True

        index = bench_index if m.type.mnt in BENCH_INDEXED_MNTS else global_index
        if m.type.kind == MMK.TRUNCATE and m.mnt == MNT.Record:
            _flush()  # unfortunately can't be batched with the other operations
            os_client.delete_by_query(
                index=index, body={"query": {"term": {"statement_id": m.statement_id}}}
            )
        elif not mirror.has_mirror(m.thing):
            continue  # ignore
        elif m.type.kind in (MMK.CREATE, MMK.UPDATE) or m.type.is_soft_delete:
            mirrored = mirror.mirror_node(project_v, m.thing)
            mirrored_data = mirrored.to_dict()
            # TODO @Broken?: should OS mutation limit update to changed properties?
            #  (partial update is not supported in index operation)
            ops.append({"index": {"_index": index, "_id": str(m.thing.id)}})
            ops.append(mirrored_data)
        elif m.type.kind == MMK.DELETE:
            ops.append({"delete": {"_index": index, "_id": str(m.thing.id)}})

    _flush()  # flush any remaining mutations


def write_module_to_os(
    project_v: models.ProjectVersion, model_tree: wire.ModuleTree, *, wipe: bool, wait: bool = False
):
    """
    Writes all nodes in the module to OpenSearch.
    If wipe, first delete all module data for that version.
    """
    global_index = IndexType.GLOBAL.get_index_name()
    bench_index = IndexType.BENCH.get_index_name(project_v.project_id)

    ops: list[dict] = []

    if wipe:
        logger.debug("os.wipe_module", project_version=project_v)
        os_client.delete_by_query(
            index=global_index, body={"query": {"term": {"project_version_id": project_v.id}}}
        )
        os_client.delete_by_query(
            index=bench_index,
            body={
                "query": {
                    "bool": {
                        "must": [
                            {"term": {"project_version_id": project_v.id}},
                            {"term": {"_type": "record"}},
                        ]
                    }
                }
            },
        )

    for node in model_tree.walk_bfs():
        if not mirror.has_mirror(node):
            continue
        index = bench_index if isinstance(node, BENCH_INDEXED_MODELS) else global_index
        ops.append({"index": {"_index": index, "_id": str(node.id)}})
        ops.append(mirror.mirror_node(project_v, node).to_dict())

    logger.debug(
        "os.write_module", project_version=project_v, index=bench_index, operations=len(ops)
    )
    ret = os_client.bulk(ops, refresh="wait_for" if wait else False)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write module to OpenSearch: {ret['items'][:5]}")


def write_session_to_os(
    project_v: models.ProjectVersion,
    session: typing.Optional[models.Session],
    runs: list[wire.RunData],
    logs: list[wire.LogEntryData],
) -> None:
    """Writes/mirrors a session to OpenSearch."""

    bench_index = IndexType.BENCH.get_index_name(project_v.project_id)
    ops: list[dict] = []
    if session:
        ops.append({"index": {"_index": bench_index, "_id": str(session.id)}})
        ops.append(mirror.mirror_node(project_v, session).to_dict())
    for run in runs:
        ops.append({"index": {"_index": bench_index, "_id": str(run.id)}})
        ops.append(mirror.unpack_node_flat(project_v, run, None).to_dict())
    for log in logs:
        ops.append({"index": {"_index": bench_index, "_id": str(log.id)}})
        ops.append(mirror.unpack_node_flat(project_v, log, None).to_dict())

    logger.debug(
        "os.write_session", project_version=project_v, index=bench_index, operations=len(ops)
    )
    ret = os_client.bulk(ops)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write session to OpenSearch: {ret['items'][:5]}")


def write_runs_to_os(runs: list[wire.RunData]) -> None:
    """Writes/mirrors runs (from different sessions/projects) to OpenSearch."""

    if not runs:
        return
    ops: list[dict] = []

    for run in runs:
        index_name = IndexType.BENCH.get_index_name(run.project_id)
        ops.append({"index": {"_index": index_name, "_id": str(run.id)}})
        ops.append(mirror.unpack_node_flat(None, run, None).to_dict())

    logger.debug("os.write_runs", operations=len(ops))
    ret = os_client.bulk(ops)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write runs to OpenSearch: {ret['items'][:5]}")


def update_dynamic_field_mappings(project_v: models.ProjectVersion) -> None:
    """
    Updates *all* dynamic OpenSearch field mappings for a module
    TODO @Performance: update OS field mappings more efficiently on field mutations
      (especially for library/dependency mappings)
    """
    from bench.language import libs
    from bench.models import packer

    logger.info("os.update_mappings", project_version=project_v)
    source = packer.pack_module(
        project_v, excluded=[models.Record, models.Trigger, models.ResolvedField, models.Issue]
    )
    module = wire.unpack_module(source, session=None)
    for dependency in libs.DEFAULT_MODULES.values():
        module.add_dependency(dependency)
    module.add_builtin(libs.symbolx_lib.get_file("builtins"))
    module.index()
    module.interp()

    value_mappings: dict[str, os.Field] = {}
    inputs_mappings: dict[str, os.Field] = {}
    outputs_mappings: dict[str, os.Field] = {}
    metadata_mappings: dict[str, os.Field] = {}

    # get library mappings
    for lib in libs.DEFAULT_MODULES.values():
        for statement in lib._nodes_by_id.values():
            if isinstance(statement, HasRun):
                for field in statement.inputs:
                    inputs_mappings[field.typed_key] = map_to_os_field(field)
                for field in statement.outputs:
                    outputs_mappings[field.typed_key] = map_to_os_field(field)
    # ensure library vectors are not indexed (would be pointless waste of resources)
    for field in (*inputs_mappings.values(), *outputs_mappings.values()):
        for f in field.walk():
            if f.type == os.FieldType.KNN_VECTOR:
                f.index = False

    # and 'static' metadata mappings (hard-coded)
    for metadata_type in (libs.symbolx_lib.lookup_or_error(".reflect.RunMetadata"),):
        for field in metadata_type.resolved_fields:
            metadata_mappings[field.typed_key] = map_to_os_field(field)

    # add dynamic user mappings
    for statement in module._nodes_by_id.values():
        if not isinstance(statement, lang.HasFields) or statement.self_errors:
            continue  # ignore symbols with issues
        elif isinstance(statement, lang.Dataset):
            # all fields go into Record.data ('data' is a "dynamic" object)
            for field in statement.resolved_fields:
                value_mappings[field.typed_key] = map_to_os_field(field)
        elif isinstance(statement, lang.HasRun):
            # inputs into Execution.inputs, outputs into Execution.outputs
            for field in statement.inputs:
                inputs_mappings[field.typed_key] = map_to_os_field(field)
            for field in statement.outputs:
                outputs_mappings[field.typed_key] = map_to_os_field(field)

    logger.info(
        "os.update_mappings.done",
        project_version=project_v,
        value_mappings=len(value_mappings),
        inputs_mappings=len(inputs_mappings),
        outputs_mappings=len(outputs_mappings),
        metadata_mappings=len(metadata_mappings),
    )
    mappings = {}
    for key, sub_mappings in (
        ("value", value_mappings),
        ("inputs", inputs_mappings),
        ("outputs", outputs_mappings),
        ("metadata", metadata_mappings),
    ):
        sub_mappings = {k: v.to_dict() for (k, v) in sub_mappings.items()}
        mappings[key] = {"type": "object", "dynamic": "strict", "properties": sub_mappings}

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
