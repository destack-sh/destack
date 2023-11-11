from typing import Any, Generator, Generic, Iterable, Optional, Type, TypeVar
from uuid import UUID

import structlog
from django.db.models import Model

from bench import models
from bench.language import libs, wire
from bench.language.const import INTERP_NODE_TYPES
from bench.language.edit import MEK, MNT, EditData
from bench.models.packer import collect_node
from bench.search import core as os
from bench.search import mirror
from bench.search.client import os_client_sync
from bench.search.core import IndexType
from bench.search.crud import (
    BENCH_LOCAL_MNTS,
    DOCUMENTS_BY_INDEX,
    SEARCH_SEMANTIC_EDIT_TYPES,
    update_field_mappings,
)

logger = structlog.get_logger(__name__)

DEFAULT_FIELDS = {
    os.TYPE_DISCRIMINATOR_KEY: os.TYPE_DISCRIMINATOR_FIELD,
}

GLOBAL_INDEX_SHARDS = 5
GLOBAL_INDEX_REPLICAS = 1

BENCH_INDEX_SHARDS = 1
BENCH_INDEX_REPLICAS = 0
BENCH_MAPPING_TOTAL_FIELDS_LIMIT = 10000  # TODO @Performance: reconsider OS mapping limit

ModelT = TypeVar("ModelT", bound=Model)
MirrorT = TypeVar("MirrorT", bound=os.Document)
DataT = TypeVar("DataT", bound=Any)


class Packer(Generic[ModelT, MirrorT, DataT]):
    def mirror(self, project_v: models.ProjectVersion | None, node: ModelT) -> MirrorT:
        raise NotImplementedError

    def pack(self, mirror: MirrorT) -> DataT:
        raise NotImplementedError

    def unpack(self, project_v: models.ProjectVersion, data: DataT, parent: ModelT) -> MirrorT:
        raise NotImplementedError


_packers_by_model: dict[type[ModelT], Packer[ModelT, MirrorT, DataT]] = {}
_packers_by_mirror: dict[type[MirrorT], Packer[ModelT, MirrorT, DataT]] = {}
_packers_by_data: dict[type[DataT], Packer[ModelT, MirrorT, DataT]] = {}


def packer(model_t: type[ModelT], mirror_t: type[MirrorT], data_t: Optional[type[DataT]] = None):
    """Decorator to register a NodePacker for a model."""

    def decorator(cls: type[Packer[ModelT, MirrorT, DataT]]):
        if model_t in _packers_by_model:
            raise ValueError(
                f"packer already registered for {model_t}: {_packers_by_model[model_t]}"
            )
        if mirror_t in _packers_by_mirror:
            raise ValueError(
                f"packer already registered for {mirror_t}: {_packers_by_mirror[mirror_t]}"
            )
        if data_t is not None and data_t in _packers_by_data:
            raise ValueError(f"packer already registered for {data_t}: {_packers_by_data[data_t]}")
        packer = cls()
        _packers_by_model[model_t] = packer
        _packers_by_mirror[mirror_t] = packer
        _packers_by_mirror[mirror_t.Partial] = packer
        if data_t is not None:
            _packers_by_data[data_t] = packer
        return cls

    return decorator


def has_mirror(node: ModelT) -> bool:
    return type(node) in _packers_by_model


def mirror_node(project_v: models.ProjectVersion | None, node: ModelT) -> MirrorT:
    packer = _packers_by_model[type(node)]
    return packer.mirror(project_v, node)


def pack_node_flat(node: MirrorT) -> DataT:
    packer = _packers_by_mirror[type(node)]
    return packer.pack(node)


def unpack_node_flat(
    project_v: models.ProjectVersion, data: DataT, parent: Optional[ModelT]
) -> MirrorT:
    packer = _packers_by_data[type(data)]
    return packer.unpack(project_v, data, parent)


def get_node_packer(mirror_t: type[MirrorT]) -> Packer[ModelT, MirrorT, DataT]:
    return _packers_by_mirror[mirror_t]


@packer(models.CrudModel, mirror.CrudThing, None)
class CrudThingPacker(Packer):
    def mirror(self, project_v: models.ProjectVersion | None, node: ModelT) -> MirrorT:
        return mirror.CrudThing(
            id=node.id,
            created_at=node.created_at,
            updated_at=node.updated_at,
            deleted_at=node.deleted_at,
            created_by_id=node.created_by_id,
            last_edited_at=node.last_edited_at,
            last_edited_by_id=node.last_edited_by_id,
        )


@packer(models.Blob, mirror.Blob, wire.BlobData)
class BlobPacker(Packer[models.Blob, mirror.Blob, wire.BlobData]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.Blob) -> mirror.Blob:
        return mirror.Blob(
            id=node.id,
            sha512=node.sha512,
            content_length=node.content_length,
            content_type=node.content_type,
            name=node.name,
        )


@packer(models.Secret, mirror.Secret, wire.SecretData)
class SecretPacker(Packer[models.Secret, mirror.Secret, wire.SecretData]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.Secret) -> mirror.Secret:
        return mirror.Secret(id=node.id, sha512=node.sha512, name=node.name)


@packer(models.User, mirror.User, None)
class UserPacker(Packer[models.User, mirror.User, None]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.User) -> mirror.User:
        return mirror.User(id=node.id, name=node.username, slug=node.slug, email=node.email)


@packer(models.Organization, mirror.Organization, None)
class OrganizationPacker(Packer[models.Organization, mirror.Organization, None]):
    def mirror(
        self, project_v: models.ProjectVersion | None, node: models.Organization
    ) -> mirror.Organization:
        return mirror.Organization(id=node.id, name=node.name, slug=node.slug)


@packer(models.Project, mirror.Project, None)
class ProjectPacker(CrudThingPacker, Packer[models.Project, mirror.Project, None]):
    def mirror(
        self, project_v: models.ProjectVersion | None, node: models.Project
    ) -> mirror.Project:
        crud = super().mirror(project_v, node)
        return mirror.Project(**crud.__dict__, name=node.name, description=node.description)


@packer(models.ProjectVersion, mirror.ProjectVersion, None)
class ProjectVersionPacker(
    CrudThingPacker, Packer[models.ProjectVersion, mirror.ProjectVersion, None]
):
    def mirror(
        self, project_v: models.ProjectVersion | None, node: models.ProjectVersion
    ) -> mirror.ProjectVersion:
        crud = super().mirror(project_v, node)
        return mirror.ProjectVersion(
            **crud.__dict__,
            project_id=node.project_id,
            name=node.name,
            tag=node.tag,
            description=node.description,
        )


@packer(models.File, mirror.File, wire.FileData)
class FilePacker(CrudThingPacker, Packer[models.File, mirror.File, wire.FileData]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.File) -> mirror.File:
        crud = super().mirror(project_v, node)
        return mirror.File(
            **crud.__dict__,
            ck=node.ck,
            project_version_id=project_v.id,
            project_id=project_v.project_id,
            name=node.name,
        )


@packer(models.Statement, mirror.Statement, wire.StatementData)
class StatementPacker(
    CrudThingPacker, Packer[models.Statement, mirror.Statement, wire.StatementData]
):
    def mirror(
        self, project_v: models.ProjectVersion | None, node: models.Statement
    ) -> mirror.Statement:
        crud = super().mirror(project_v, node)
        return mirror.Statement(
            **crud.__dict__,
            ck=node.ck,
            project_version_id=node.project_version_id,
            project_id=project_v.project_id,
            file_id=node.file_id,
            type=node.type,
            name=node.name,
            text=node.text,
            code=node.code,
        )


@packer(models.Field, mirror.Field, wire.FieldData)
class FieldPacker(CrudThingPacker, Packer[models.Field, mirror.Field, wire.FieldData]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.Field) -> mirror.Field:
        crud = super().mirror(project_v, node)
        return mirror.Field(
            **crud.__dict__,
            ck=node.ck,
            project_version_id=node.statement.project_version_id,
            project_id=project_v.project_id,
            statement_id=node.statement_id,
            name=node.name,
            text=node.text,
            type_tag=node.tag,
            type_hint=node.hint,
        )


@packer(models.Record, mirror.Record, wire.RecordData)
class RecordPacker(CrudThingPacker, Packer[models.Record, mirror.Record, wire.RecordData]):
    def mirror(self, project_v: models.ProjectVersion | None, node: mirror.Record) -> mirror.Record:
        return mirror.Record(
            id=node.id,
            ck=node.ck,
            project_id=project_v.project_id,
            project_version_id=project_v.id,
            statement_id=node.statement_id,
            statement_ck=node.statement_ck,
            statement_key=node.statement_key,
            value=node.value,
            revision=node.revision,
            created_at=node.created_at,
            created_by_id=node.created_by_id,
            updated_at=node.updated_at,
            deleted_at=node.deleted_at if node.deleted_at else "-",  # reset with invalid date
            last_edited_at=node.last_edited_at,
            last_edited_by_id=node.last_edited_by_id,
        )

    def pack(self, node: models.Record) -> wire.RecordData:
        return wire.RecordData(
            id=node.id,
            ck=node.ck,
            parent_id=node.statement_id,
            parent_key=node.statement_key,
            value=node.value,
            revision=node.revision,
            created_at=node.created_at,
            updated_at=node.updated_at,
            last_edited_at=node.last_edited_at,
            last_changed_at=None,
        )

    def unpack(
        self, project_v: models.ProjectVersion, data: wire.RecordData, parent: models.Statement
    ):
        return mirror.Record(
            id=data.id,
            ck=data.ck,
            project_version_id=project_v.id,
            statement_id=data.parent_id,
            statement_ck=parent.ck,
            statement_key=parent.key,
            value=data.value,
            revision=data.revision,
            created_at=data.created_at,
            created_by_id=None,
            updated_at=data.updated_at,
            deleted_at=None,
            last_edited_at=data.last_edited_at,
            last_edited_by_id=None,
        )


@packer(models.Session, mirror.Session, wire.SessionData)
class SessionPacker(Packer[models.Session, mirror.Session, wire.SessionData]):
    def mirror(
        self, project_v: models.ProjectVersion | None, node: models.Session
    ) -> mirror.Session:
        return mirror.Session(
            id=node.id,
            project_version_id=node.project_version_id,
            opened_at=node.opened_at,
            closed_at=node.closed_at,
            trigger_type=node.trigger_type,
        )


@packer(models.Run, mirror.Run, wire.RunData)
class RunPacker(Packer[models.Run, mirror.Run, wire.RunData]):
    def pack(self, mirror: mirror.Run) -> wire.RunData:
        statement_type = (
            wire.StatementType(mirror.statement_type) if mirror.statement_type else None
        )
        return wire.RunData(
            id=mirror.id,
            project_id=mirror.project_id,
            worker_node_id=mirror.worker_node_id,
            worker_process_id=mirror.worker_process_id,
            module_id=mirror.project_version_id,
            session_id=mirror.session_id,
            trigger_type=mirror.trigger_type,
            trigger_id=None,  # not stored
            root_id=mirror.root_id,
            parent_id=mirror.parent_id,
            statement_id=mirror.statement_id,
            statement_ck=mirror.statement_ck,
            statement_type=statement_type,
            created_at=mirror.created_at,
            updated_at=mirror.updated_at,
            scheduled_at=mirror.scheduled_at,
            started_at=mirror.started_at,
            terminated_at=mirror.terminated_at,
            status=mirror.RunStatus(mirror.status),
            inputs=mirror.inputs,
            outputs=mirror.outputs,
            error=None,
            value=mirror.value,
        )

    def unpack(self, project_v: None, data: wire.RunData, parent: None) -> mirror.Run:
        if data.started_at and data.terminated_at:
            duration = (data.terminated_at - data.started_at).total_seconds()
        else:
            duration = None
        return mirror.Run(
            id=data.id,
            project_id=data.project_id,
            project_version_id=data.module_id,
            worker_node_id=data.worker_node_id,
            worker_process_id=data.worker_process_id,
            session_id=data.session_id,
            trigger_type=data.trigger_type,
            root_id=data.root_id,
            parent_id=data.parent_id,
            statement_id=data.statement_id,
            statement_ck=data.statement_ck,
            statement_type=data.statement_type,
            created_at=data.created_at,
            updated_at=data.updated_at,
            scheduled_at=data.scheduled_at,
            started_at=data.started_at,
            terminated_at=data.terminated_at,
            duration=duration,
            status=data.status,
            inputs=data.inputs,
            outputs=data.outputs,
            error=None,
            value=data.value,
        )


@packer(mirror.LogEntry, mirror.LogEntry, wire.LogEntryData)
class LogEntryPacker(Packer[mirror.LogEntry, mirror.LogEntry, wire.LogEntryData]):
    def pack(self, mirror: mirror.LogEntry) -> wire.LogEntryData:
        return wire.LogEntryData(
            id=mirror.id,
            module_id=mirror.project_version_id,
            session_id=mirror.session_id,
            run_id=mirror.run_id,
            statement_id=mirror.statement_id,
            statement_ck=mirror.statement_ck,
            created_at=mirror.created_at,
            stream=mirror.stream,
            level=mirror.level,
            logger=mirror.logger,
            message=mirror.message,
            value=mirror.value,
        )

    def unpack(
        self, project_v: models.ProjectVersion, data: wire.LogEntryData, parent: None
    ) -> mirror.LogEntry:
        return mirror.LogEntry(
            id=data.id,
            project_version_id=project_v.id,
            session_id=data.session_id,
            run_id=data.run_id,
            statement_id=data.statement_id,
            statement_ck=data.statement_ck,
            created_at=data.created_at,
            stream=data.stream,
            level=data.level,
            logger=data.logger,
            message=data.message,
            value=data.value,
        )


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
    logger.info(
        "os.create_index",
        index_name=index_name,
        shards=shards,
        replicas=replicas,
        fields=list(fields.keys()),
    )
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
    # nocheckin: create user/role


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
            os_client_sync.indices.refresh(index=project.os_name)
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
            ret = os_client_sync.bulk(ops, refresh="" if refresh else False)
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
            os_client_sync.delete_by_query(
                index=index, body={"query": {"term": {"statement_key": e.node.key}}}
            )
        elif not has_mirror(e.thing):
            continue  # ignore
        elif e.type.kind in (MEK.CREATE, MEK.UPDATE) or e.type.is_soft_delete:
            mirrored = mirror_node(project_v, e.thing)
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
    nodes: Iterable[models.ModuleNode] | Generator[models.ModuleNode, None, None],
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
        if not has_mirror(node):
            continue
        index = project.os_name if isinstance(node, BENCH_LOCAL_MODELS) else os.GLOBAL_INDEX_NAME
        ops.append({"index": {"_index": index, "_id": str(node.id)}})
        ops.append(mirror_node(project_v, node).to_dict())

    logger.debug(
        "os.write_module", project_version=project_v, index=project.os_name, operations=len(ops)
    )
    if not ops:
        return
    ret = os_client_sync.bulk(ops, refresh="wait_for" if wait else False)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write module to OpenSearch: {ret['items'][:5]}")


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
        ops.append(unpack_node_flat(None, run, None).to_dict())
    logger.debug("os.write_runs", operations=len(ops))
    ret = os_client_sync.bulk(ops)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write runs to OpenSearch: {ret['items'][:5]}")


def write_session_to_os(
    project_v: models.ProjectVersion,
    session: Optional[models.Session],
    runs: list[wire.RunData],
    logs: list[wire.LogEntryData],
) -> None:
    """Writes/mirrors a session to OpenSearch."""

    os_name = project_v.project.os_name
    ops: list[dict] = []
    if session:
        ops.append({"index": {"_index": os_name, "_id": str(session.id)}})
        ops.append(mirror_node(project_v, session).to_dict())
    for run in runs:
        ops.append({"index": {"_index": os_name, "_id": str(run.id)}})
        ops.append(unpack_node_flat(project_v, run, None).to_dict())
    for log in logs:
        ops.append({"index": {"_index": os_name, "_id": str(log.id)}})
        ops.append(unpack_node_flat(project_v, log, None).to_dict())

    logger.debug("os.write_session", project_version=project_v, index=os_name, operations=len(ops))
    ret = os_client_sync.bulk(ops)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write session to OpenSearch: {ret['items'][:5]}")


def write_sessions_to_os(project_v: models.ProjectVersion) -> None:
    """Writes/mirrors all sessions and runs to OpenSearch."""
    from bench.models import packer

    bench_index = project_v.project.os_name
    ops: list[dict] = []
    for session in project_v.sessions.all():
        ops.append({"index": {"_index": bench_index, "_id": str(session.id)}})
        ops.append(mirror_node(project_v, session).to_dict())
    for run in project_v.runs.all():
        ops.append({"index": {"_index": bench_index, "_id": str(run.id)}})
        run = packer.pack_data(run)
        ops.append(unpack_node_flat(project_v, run, None).to_dict())
    logger.debug(
        "os.write_sessions", project_version=project_v, index=bench_index, operations=len(ops)
    )
    ret = os_client_sync.bulk(ops)
    if ret.get("errors"):
        raise RuntimeError(f"failed to write sessions to OpenSearch: {ret['items'][:5]}")


def delete_module_in_os(project_v: models.ProjectVersion):
    logger.debug("os.delete", project_version=project_v)
    os_client_sync.delete_by_query(
        index=os.GLOBAL_INDEX_NAME,
        body={"query": {"term": {"project_version_id": project_v.id}}},
    )
    os_client_sync.delete_by_query(
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
