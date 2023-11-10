import typing
from uuid import UUID

import structlog

from bench import models
from bench.language import libs, wire
from bench.language.const import INTERP_NODE_TYPES
from bench.language.edit import MEK, MNT, EditData
from bench.models import packer
from bench.models.packer import collect_node
from bench.search import core as os
from bench.search import mirror
from bench.search.client import os_client
from bench.search.core import IndexType
from bench.search.crud import (
    BENCH_LOCAL_MNTS,
    DOCUMENTS_BY_INDEX,
    SEARCH_SEMANTIC_EDIT_TYPES,
    update_field_mappings,
)

DEFAULT_FIELDS = {
    os.TYPE_DISCRIMINATOR_KEY: os.TYPE_DISCRIMINATOR_FIELD,
}

GLOBAL_INDEX_SHARDS = 5
GLOBAL_INDEX_REPLICAS = 1

BENCH_INDEX_SHARDS = 1
BENCH_INDEX_REPLICAS = 0
BENCH_MAPPING_TOTAL_FIELDS_LIMIT = 10000  # TODO @Performance: reconsider OS mapping limit

logger = structlog.get_logger(__name__)


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


def create_global_search_index(upsert: bool = False) -> None:
    _create_index(
        os.GLOBAL_INDEX_NAME,
        shards=GLOBAL_INDEX_SHARDS,
        replicas=GLOBAL_INDEX_REPLICAS,
        documents=DOCUMENTS_BY_INDEX[IndexType.GLOBAL],
        upsert=upsert,
    )


def create_local_search_index(project_id: UUID, name: str, *, upsert: bool) -> None:
    _create_index(
        name,
        shards=BENCH_INDEX_SHARDS,
        replicas=BENCH_INDEX_REPLICAS,
        documents=DOCUMENTS_BY_INDEX[IndexType.LOCAL],
        upsert=upsert,
    )


BENCH_LOCAL_MODELS = (models.Record,)


def write_edits_to_os(
    project_v: models.ProjectVersion, edit: list[EditData], *, refresh: bool = False
) -> None:
    """
    Writes/mirrors any relevant edit to OpenSearch.
    All regular DB edit come this way.
    """
    project: models.Project = project_v.project
    if not edit:
        if refresh:
            # just refresh the index
            os_client.indices.refresh(index=project.os_name)
        return  # nothing to do

    # mut state
    ops: list[dict] = []
    field_mappings_dirty: list[bool] = [False]  # for closure

    def _flush():
        if field_mappings_dirty[0]:  # if needed, must happen before any other edit
            update_field_mappings_from_db(project_v)

        if ops:
            logger.debug(
                "os.write_edits",
                project_version=project_v,
                index=project.os_name,
                edit=len(edit),
                operations=len(ops),
            )
            # TODO @Performance: consider bulking OS refreshes in edit somehow
            ret = os_client.bulk(ops, refresh="" if refresh else False)
            if ret.get("errors"):
                bad_items = [i for i in ret["items"] if i.get("index", {}).get("error")]
                raise RuntimeError(f"failed to write edit to OpenSearch: {bad_items[:5]}")

        ops.clear()
        field_mappings_dirty[0] = False

    for e in edit:
        # mark field mappings as dirty if relevant mutation
        if e.type in SEARCH_SEMANTIC_EDIT_TYPES:
            field_mappings_dirty[0] = True

        index = project.os_name if e.type.mnt in BENCH_LOCAL_MNTS else os.GLOBAL_INDEX_NAME
        if e.type.kind == MEK.TRUNCATE and e.mnt == MNT.RECORD:
            _flush()  # unfortunately can't be batched with the other operations
            os_client.delete_by_query(
                index=index, body={"query": {"term": {"statement_key": e.node.key}}}
            )
        elif not mirror.has_mirror(e.thing):
            continue  # ignore
        elif e.type.kind in (MEK.CREATE, MEK.UPDATE) or e.type.is_soft_delete:
            mirrored = mirror.mirror_node(project_v, e.thing)
            mirrored_data = mirrored.to_dict()
            # TODO @Robustness: limit OS edit to changed properties?
            #  (partial update is not supported in index operation)
            ops.append({"index": {"_index": index, "_id": str(e.thing.id)}})
            ops.append(mirrored_data)
        elif e.type.kind == MEK.DELETE:
            ops.append({"delete": {"_index": index, "_id": str(e.thing.id)}})

    _flush()  # flush any remaining edit


def update_field_mappings_from_db(project_v: models.ProjectVersion) -> None:
    from bench.models import packer

    logger.info("os.update_mappings", project_version=project_v)
    source = packer.pack_module(
        project_v, excluded=[models.Record, models.Trigger, models.ResolvedField, models.Issue]
    )
    module = wire.unpack_module(source.nodes, exclude=INTERP_NODE_TYPES, session=None)
    for dependency in libs.DEFAULT_MODULES.values():
        module.add_dependency(dependency)
    module.add_builtin(libs.symbolx_lib.files.get("builtins"))
    module._interp_rec()

    update_field_mappings(project_v.project.os_name, module)


def write_module_to_os(
    project_v: models.ProjectVersion,
    nodes: typing.Iterable[models.ModuleNode] | typing.Generator[models.ModuleNode, None, None],
    *,
    wipe: bool,
    wait: bool = False,
):
    """
    Writes all nodes in the module to OpenSearch.
    If wipe, first delete all module data for that version.
    """
    ops: list[dict] = []

    if wipe:
        delete_module_in_os(project_v)

    update_field_mappings_from_db(project_v)  # can we only do this sometimes? when?

    project: models.Project = project_v.project
    for node in nodes:
        if not mirror.has_mirror(node):
            continue
        index = project.os_name if isinstance(node, BENCH_LOCAL_MODELS) else os.GLOBAL_INDEX_NAME
        ops.append({"index": {"_index": index, "_id": str(node.id)}})
        ops.append(mirror.mirror_node(project_v, node).to_dict())

    logger.debug(
        "os.write_module", project_version=project_v, index=project.os_name, operations=len(ops)
    )
    if not ops:
        return
    ret = os_client.bulk(ops, refresh="wait_for" if wait else False)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write module to OpenSearch: {ret['errors'][:5]}")


def write_module_to_os_from_db(project_v: models.ProjectVersion, *, wipe: bool) -> None:
    nodes = collect_node(project_v)
    write_module_to_os(project_v, nodes.visited.values(), wipe=wipe)


def write_runs_to_os(os_names: str | list[str], runs: list[wire.RunData]) -> None:
    """Writes/mirrors runs (from different sessions/projects) to OpenSearch."""

    if not runs:
        return
    if isinstance(os_names, list) and len(os_names) != len(runs):
        raise ValueError(f"len(os_names) != len(runs): {len(os_names)} != {len(runs)}")
    ops: list[dict] = []
    for i, run in enumerate(runs):
        index = os_names[i] if isinstance(os_names, list) else os_names
        ops.append({"index": {"_index": index, "_id": str(run.id)}})
        ops.append(mirror.unpack_node_flat(None, run, None).to_dict())
    logger.debug("os.write_runs", operations=len(ops))
    ret = os_client.bulk(ops)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write runs to OpenSearch: {ret['items'][:5]}")


def write_session_to_os(
    project_v: models.ProjectVersion,
    session: typing.Optional[models.Session],
    runs: list[wire.RunData],
    logs: list[wire.LogEntryData],
) -> None:
    """Writes/mirrors a session to OpenSearch."""

    os_name = project_v.project.os_name
    ops: list[dict] = []
    if session:
        ops.append({"index": {"_index": os_name, "_id": str(session.id)}})
        ops.append(mirror.mirror_node(project_v, session).to_dict())
    for run in runs:
        ops.append({"index": {"_index": os_name, "_id": str(run.id)}})
        ops.append(mirror.unpack_node_flat(project_v, run, None).to_dict())
    for log in logs:
        ops.append({"index": {"_index": os_name, "_id": str(log.id)}})
        ops.append(mirror.unpack_node_flat(project_v, log, None).to_dict())

    logger.debug("os.write_session", project_version=project_v, index=os_name, operations=len(ops))
    ret = os_client.bulk(ops)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write session to OpenSearch: {ret['errors'][:5]}")


def write_sessions_to_os(project_v: models.ProjectVersion) -> None:
    """Writes/mirrors all sessions and runs to OpenSearch."""
    bench_index = project_v.project.os_name
    ops: list[dict] = []
    for session in project_v.sessions.all():
        ops.append({"index": {"_index": bench_index, "_id": str(session.id)}})
        ops.append(mirror.mirror_node(project_v, session).to_dict())
    for run in project_v.runs.all():
        ops.append({"index": {"_index": bench_index, "_id": str(run.id)}})
        run = packer.pack_data(run)
        ops.append(mirror.unpack_node_flat(project_v, run, None).to_dict())
    logger.debug(
        "os.write_sessions", project_version=project_v, index=bench_index, operations=len(ops)
    )
    ret = os_client.bulk(ops)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write sessions to OpenSearch: {ret['errors'][:5]}")


def delete_module_in_os(project_v: models.ProjectVersion):
    logger.debug("os.delete", project_version=project_v)
    os_client.delete_by_query(
        index=os.GLOBAL_INDEX_NAME,
        body={"query": {"term": {"project_version_id": project_v.id}}},
    )
    os_client.delete_by_query(
        index=project_v.project.os_name,
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
