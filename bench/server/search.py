from typing import Generator, Iterable, Optional, Type

import structlog
from asgiref.sync import sync_to_async

from bench import models
from bench.language import Module, wire
from bench.language.const import LOCAL_NODE_TYPES
from bench.language.edit import MEK, EditData
from bench.models.packer import HOST_MODEL_TYPES, collect_node
from bench.search import core as os
from bench.search import mirror
from bench.search.client import get_os_errors, os_client, os_client_sync
from bench.search.core import IndexType
from bench.search.engine import DOCUMENTS_BY_INDEX, update_os_schema

logger = structlog.get_logger(__name__)

DEFAULT_FIELDS = {
    os.TYPE_DISCRIMINATOR_KEY: os.TYPE_DISCRIMINATOR_FIELD,
}

GLOBAL_INDEX_SHARDS = 5
GLOBAL_INDEX_REPLICAS = 1
GLOBAL_READ_ONLY_ROLE = "global-ro"

BENCH_INDEX_SHARDS = 1
BENCH_INDEX_REPLICAS = 0
BENCH_MAPPING_TOTAL_FIELDS_LIMIT = 10000  # TODO @Performance: reconsider OS mapping limit


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
    documents: list[Type[os.Document]],
    upsert: bool = False,
) -> None:
    fields = {**DEFAULT_FIELDS, **_collect_fields(documents)}
    mappings = {field_name: field.to_dict() for field_name, field in fields.items()}
    tokenizers = {
        tokenizer.value: definition for tokenizer, definition in os.CUSTOM_TOKENIZERS.items()
    }
    analyzers = {analyzer.value: definition for analyzer, definition in os.CUSTOM_ANALYZERS.items()}

    log = logger.bind(
        index_name=index_name,
        shards=shards,
        replicas=replicas,
        fields=list(fields.keys()),
    )
    log.info("os.create_index")
    result = os_client_sync.indices.create(
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
        os_client_sync.indices.close(index=index_name)
        # update mutable settings
        os_client_sync.indices.put_settings(
            index=index_name,
            body={
                "analysis": {"tokenizer": tokenizers, "analyzer": analyzers},
                "mapping": {"total_fields": {"limit": BENCH_MAPPING_TOTAL_FIELDS_LIMIT}},
            },
        )
        os_client_sync.indices.put_mapping(
            index=index_name, body={"dynamic": "strict", "properties": mappings}
        )
        # reopen index
        os_client_sync.indices.open(index=index_name)
    log.info("os.create_index.done")


def create_global_os_index(upsert: bool = False) -> None:
    logger.info("os.create_global_index")
    _create_index(
        os.GLOBAL_INDEX_NAME,
        shards=GLOBAL_INDEX_SHARDS,
        replicas=GLOBAL_INDEX_REPLICAS,
        documents=DOCUMENTS_BY_INDEX[IndexType.GLOBAL],
        upsert=upsert,
    )


def create_global_os_role() -> None:
    # creates a global role (that doesn't do anything yet)
    # every user has this role to read public indices
    rep = os_client_sync.security.get_role(role=GLOBAL_READ_ONLY_ROLE, ignore=404)
    if rep.get("status") == "NOT_FOUND":
        rep = os_client_sync.security.create_role(
            role=GLOBAL_READ_ONLY_ROLE,
            body={
                "cluster_permissions": [],
                "index_permissions": [],
                "tenant_permissions": [],
            },
        )
        if rep.get("error"):
            raise RuntimeError(f"failed to create global-ro role: {rep['error']}")
        logger.info("os.create_global_role", name=GLOBAL_READ_ONLY_ROLE, rep=rep)
    else:
        logger.info("os.global_role_exists", name=GLOBAL_READ_ONLY_ROLE, rep=rep)


def create_local_os_index(project: models.Project, *, upsert: bool) -> None:
    """
    Creates the OpenSearch index and corresponding roles/user for a project.
    """
    log = logger.bind(project=project, os_name=project.os_name)
    log.info("os.create_local_index")
    # create index
    _create_index(
        index_name=project.os_name,
        shards=BENCH_INDEX_SHARDS,
        replicas=BENCH_INDEX_REPLICAS,
        documents=DOCUMENTS_BY_INDEX[IndexType.LOCAL],
        upsert=upsert,
    )

    # get global read only role to modify
    rep = os_client_sync.security.get_role(role=GLOBAL_READ_ONLY_ROLE, ignore=404)
    role = rep.get(GLOBAL_READ_ONLY_ROLE)
    assert role is not None, f"failed to get role {GLOBAL_READ_ONLY_ROLE}: {rep}"
    if project.visibility == models.ProjectVisibility.PUBLIC:
        # TODO @Robustness: fix race condition between read and patch global role
        #  (unfortunately the 'add' op doesn't seem to be actually additive?)
        # grant read access to global read only role
        log.info("os.grant_global_read_access")
        rep = os_client_sync.security.patch_role(
            role=GLOBAL_READ_ONLY_ROLE,
            body=[
                {
                    "op": "add",
                    "path": "/index_permissions",
                    "value": [
                        *(
                            r
                            for r in role["index_permissions"]
                            if r["index_patterns"] != [project.os_name]
                        ),
                        {
                            "index_patterns": [project.os_name],
                            "fls": [],
                            "masked_fields": [],
                            "allowed_actions": ["read"],
                        },
                    ],
                }
            ],
        )
        if rep.get("error"):
            raise RuntimeError(f"failed to grant read access to global-ro: {rep['error']}")
        log.info("os.grant_global_read_access.done")
    else:
        # revoke read access from global read only role (if exists)
        log.info("os.revoke_global_read_access")

        # find index permission for this project
        permission_idx = -1
        for i, index_permission in enumerate(role["index_permissions"]):
            if index_permission["index_patterns"] == [project.os_name]:
                permission_idx = i
                break
        if permission_idx >= 0:
            rep = os_client_sync.security.patch_role(
                role=GLOBAL_READ_ONLY_ROLE,
                body={"op": "remove", "path": f"/index_permissions/{permission_idx}"},
            )
            if rep.get("error"):
                raise RuntimeError(f"failed to revoke read access from global-ro: {rep['error']}")
            log.info("os.revoke_global_read_access.done")
        else:
            log.info("os.revoke_global_read_access.not_found")

    # create write access role for project owner
    log.info("os.create_owner_role")
    owner_role_name = f"{project.os_name}-rw"
    rep = os_client_sync.security.get_role(role=owner_role_name, ignore=404)
    owner_role = rep.get(owner_role_name)
    if owner_role is not None:
        os_client_sync.security.delete_role(role=owner_role_name)
    rep = os_client_sync.security.create_role(
        role=owner_role_name,
        body={
            "cluster_permissions": [
                # this is required for all bulk indexing
                #  (the actual permission is checked per index@)
                "indices:data/write/bulk",
            ],
            "index_permissions": [
                {
                    "index_patterns": [project.os_name],
                    "fls": [],
                    "masked_fields": [],
                    "allowed_actions": ["*"],
                }
            ],
        },
    )
    if rep.get("error"):
        raise RuntimeError(f"failed to create role {owner_role_name}: {rep['error']}")
    log.info("os.create_owner_role.done")

    # create user with those roles
    log.info("os.create_user")
    rep = os_client_sync.security.get_user(username=project.os_username, ignore=404)
    user = rep.get(project.os_username)
    if user is not None:
        os_client_sync.security.delete_user(username=project.os_username)
    rep = os_client_sync.security.create_user(
        username=project.os_username,
        body={
            "password": project.os_password,
            "opendistro_security_roles": [owner_role_name, GLOBAL_READ_ONLY_ROLE],
        },
    )
    if rep.get("error"):
        raise RuntimeError(f"failed to create user {project.os_username}: {rep['error']}")
    log.info("os.create_user.done", username=project.os_username)


async def write_edits_to_os(
    module: Module, edits: list[EditData], *, refresh: bool = False
) -> None:
    """
    Writes/mirrors any relevant edit to OpenSearch.
    All regular DB edit come this way.
    """
    log = logger.bind(module=module, edits=edits)
    log.debug("os.write_edits")
    if not edits:
        if refresh:
            # just refresh the index
            os_client_sync.indices.refresh(index=module.os_name)
        return  # nothing to do

    # mut state
    ops: list[dict] = []

    async def _flush():
        if ops:
            log.debug("os.write_edits.flush", operations=len(ops))
            ret = await os_client.bulk(ops, refresh="" if refresh else False)
            if ret.get("errors"):
                raise RuntimeError(f"failed to write edit to OpenSearch: {get_os_errors(ret)}")

        ops.clear()

    for edit in edits:
        index = module.os_name if edit.type.mnt in LOCAL_NODE_TYPES else os.GLOBAL_INDEX_NAME
        node = edit.thing or edit.node  # local nodes don't have a model thing, only data node
        if not mirror.has_mirror(node):
            continue  # ignore
        elif edit.type.kind in (MEK.CREATE, MEK.UPDATE, MEK.MOVE, MEK.SOFT_DELETE, MEK.RESTORE):
            if isinstance(node, models.ModuleNode):
                mirrored_data = mirror.mirror_node(module, node).to_dict()
            elif isinstance(node, wire.NodeData):
                parent = module.resolve(node.parent_id)
                mirrored_data = mirror.unpack_node_flat(module, node, parent).to_dict()
            else:
                raise ValueError(f"unexpected node type: {node!r}")
            ops.append({"index": {"_index": index, "_id": str(node.id)}})
            ops.append(mirrored_data)
        elif edit.type.kind == MEK.DELETE:
            ops.append({"delete": {"_index": index, "_id": str(node.id)}})
        else:
            raise ValueError(f"unexpected edit type: {edit!r}")

    await _flush()  # flush all remaining edits


async def update_os_schema_from_db(
    project_v: models.ProjectVersion, dynamic: str = "strict"
) -> None:
    from bench.server.runtime import interp_module

    logger.info("os.update_mappings", project_version=project_v)
    module, project = await interp_module(project_v.id)
    await update_os_schema(project_v.project.os_name, module, dynamic=dynamic)


async def write_module_to_os(
    project_v: models.ProjectVersion,
    nodes: Iterable[models.ModuleNode] | Generator[models.ModuleNode, None, None],
    *,
    wipe: bool,
    update_schema: bool = True,
    wait: bool = False,
):
    """
    Writes all nodes in the module to OpenSearch.
    If wipe, first delete all module data for that version.
    """
    ops: list[dict] = []

    if wipe:
        await delete_module_in_os(project_v)

    if update_schema:
        await update_os_schema_from_db(project_v)  # can we only do this sometimes? when?

    project: models.Project = project_v.project
    for node in nodes:
        if not mirror.has_mirror(node):
            continue
        if type(node) in HOST_MODEL_TYPES:
            index = os.GLOBAL_INDEX_NAME
        else:
            index = project.os_name
        ops.append({"index": {"_index": index, "_id": str(node.id)}})
        ops.append(mirror.mirror_node(project_v, node).to_dict())

    logger.debug(
        "os.write_module", project_version=project_v, index=project.os_name, operations=len(ops)
    )
    if not ops:
        return
    ret = await os_client.bulk(ops, refresh="wait_for" if wait else False)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write module to OpenSearch: {get_os_errors(ret)}")


async def write_module_to_os_from_db(
    project_v: models.ProjectVersion, *, wipe: bool, update_mappings: bool
) -> None:
    nodes = await sync_to_async(collect_node)(project_v)
    await write_module_to_os(
        project_v, nodes.visited.values(), wipe=wipe, update_schema=update_mappings
    )


def disable_os_strict_mapping(index_name: str) -> None:
    logger.info("os.disable_strict_dynamic_mapping", index=index_name)
    rep = os_client_sync.indices.put_mapping(index=index_name, body={"dynamic": "false"})
    if rep.get("error"):
        raise RuntimeError(f"failed to disable strict dynamic mapping: {rep['error']}")


def enable_os_strict_mapping(index_name: str) -> None:
    logger.info("os.enable_strict_dynamic_mapping", index=index_name)
    rep = os_client_sync.indices.put_mapping(index=index_name, body={"dynamic": "strict"})
    if rep.get("error"):
        raise RuntimeError(f"failed to enable strict dynamic mapping: {rep['error']}")


def write_runs_to_os(
    project_vs: models.ProjectVersion | list[models.ProjectVersion], runs: list[wire.RunData]
) -> None:
    """Writes/mirrors runs (from different sessions/projects) to OpenSearch."""

    if not runs:
        return
    if isinstance(project_vs, list) and len(project_vs) != len(runs):
        raise ValueError(f"len(project_vs) != len(runs): {len(project_vs)} != {len(runs)}")
    ops: list[dict] = []
    for i, run in enumerate(runs):
        project_v = project_vs[i] if isinstance(project_vs, list) else project_vs
        ops.append({"index": {"_index": project_v.project.os_name, "_id": str(run.id)}})
        ops.append(mirror.unpack_node_flat(project_v, run, None).to_dict())
    logger.debug("os.write_runs", operations=len(ops))
    ret = os_client_sync.bulk(ops)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write runs to OpenSearch: {get_os_errors(ret)}")


def write_session_to_os(
    project_v: models.ProjectVersion, session: Optional[models.Session], runs: list[wire.RunData]
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
    logger.debug("os.write_session", project_version=project_v, index=os_name, operations=len(ops))
    ret = os_client_sync.bulk(ops)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write session to OpenSearch: {get_os_errors(ret)}")


async def write_sessions_to_os_from_db(project_v: models.ProjectVersion) -> None:
    """Writes/mirrors all sessions and runs to OpenSearch."""
    from bench.models import packer

    os_name = project_v.project.os_name
    ops: list[dict] = []
    for session in project_v.sessions.all():
        ops.append({"index": {"_index": os_name, "_id": str(session.id)}})
        ops.append(mirror.mirror_node(project_v, session).to_dict())
    for run in project_v.runs.all():
        ops.append({"index": {"_index": os_name, "_id": str(run.id)}})
        run = packer.pack_data(run)
        ops.append(mirror.unpack_node_flat(project_v, run, None).to_dict())
    logger.debug("os.write_sessions", project_version=project_v, index=os_name, operations=len(ops))
    if ops:
        ret = await os_client.bulk(ops)
        if ret.get("errors"):
            raise RuntimeError(f"failed to write sessions to OpenSearch: {get_os_errors(ret)}")


async def delete_module_in_os(project_v: models.ProjectVersion):
    logger.debug("os.delete", project_version=project_v)
    await os_client.delete_by_query(
        index=os.GLOBAL_INDEX_NAME,
        body={"query": {"term": {"project_version_id": project_v.id}}},
    )
    await os_client.delete_by_query(
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
