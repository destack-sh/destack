import enum
from dataclasses import replace
from datetime import datetime
from typing import TYPE_CHECKING, Any, Generic, Optional, TypeVar
from uuid import UUID

import bench.opensearch.core as os
from bench import models
from bench.language import StatementType, wire
from bench.language.query import SubfieldType

if TYPE_CHECKING:
    from django.db.models import Model
else:
    Model = object

NAME_FIELD = os.Field(
    os.FT.TEXT,
    fields={
        SubfieldType.starts_with: os.Field(os.FieldType.SEARCH_AS_YOU_TYPE),
        SubfieldType.key: os.Field(os.FieldType.KEYWORD),
    },
)
HTML_FIELD = os.Field(os.FieldType.TEXT, analyzer=os.Analyzer.HTML)

ModelT = TypeVar("ModelT", bound=Model)
MirrorT = TypeVar("MirrorT", bound=os.Document)
DataT = TypeVar("DataT", bound=Any)


class DocumentType(enum.StrEnum):
    REMOTE_OBJECT = "remote_object"
    SECRET = "secret"
    USER = "user"
    ORGANIZATION = "organization"
    PROJECT = "project"
    PROJECT_VERSION = "project_version"
    FILE = "file"
    STATEMENT = "statement"
    FIELD = "field"
    TILE = "tile"
    RECORD = "record"
    COMMENT = "comment"
    SESSION = "session"
    RUN = "run"
    LOG_ENTRY = "log_entry"


DOCUMENT_CLASS_BY_TYPE: dict[DocumentType, type[os.Document]] = {}


def document(type: DocumentType, *, store_type: bool = True):
    """Decorator to register a Document subclass."""

    def decorator(cls):
        cls = os.document(cls, type.value, store_type=store_type)
        if type in DOCUMENT_CLASS_BY_TYPE:
            raise ValueError(
                f"document already registered for {type}: {DOCUMENT_CLASS_BY_TYPE[type]}"
            )
        DOCUMENT_CLASS_BY_TYPE[type] = cls
        return cls

    return decorator


class Packer(Generic[ModelT, MirrorT, DataT]):
    def mirror(self, project_v: models.ProjectVersion | None, node: ModelT) -> MirrorT:
        raise NotImplementedError

    def unmirror(self, mirror: MirrorT) -> ModelT:
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


def unmirror_node(mirror: MirrorT) -> ModelT:
    packer = _packers_by_mirror[type(mirror)]
    return packer.unmirror(mirror)


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


# basic

Document = os.Document


@os.document
class CrudThing(os.Document):
    created_at: datetime = os.field(os.FT.DATE)
    updated_at: datetime = os.field(os.FT.DATE)
    # to allow reset to 'null'  via invalid date
    deleted_at: Optional[datetime] = os.field(os.FT.DATE, ignore_malformed=True)
    created_by_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    last_edited_at: datetime = os.field(os.FT.DATE)
    last_edited_by_id: Optional[UUID] = os.field(os.FT.KEYWORD)


class Revisioned(os.Document):
    revision: int = os.field(os.FT.INTEGER, index=False)


@packer(models.CrudModel, CrudThing, None)
class CrudThingPacker(Packer):
    def mirror(self, project_v: models.ProjectVersion | None, node: ModelT) -> MirrorT:
        return CrudThing(
            id=node.id,
            created_at=node.created_at,
            updated_at=node.updated_at,
            deleted_at=node.deleted_at,
            created_by_id=node.created_by_id,
            last_edited_at=node.last_edited_at,
            last_edited_by_id=node.last_edited_by_id,
        )


@document(DocumentType.REMOTE_OBJECT)
class RemoteObject(os.Document):
    #  :RemoteObjectType
    sha512: str = os.field(os.FT.KEYWORD)
    content_length: int = os.field(os.FT.LONG)
    content_type: str = os.field(os.FT.KEYWORD)
    name: str = NAME_FIELD
    status: str = os.field(os.FT.KEYWORD)


@packer(models.RemoteObject, RemoteObject, wire.RemoteObjectData)
class RemoteObjectPacker(Packer[models.RemoteObject, RemoteObject, wire.RemoteObjectData]):
    def mirror(
        self, project_v: models.ProjectVersion | None, node: models.RemoteObject
    ) -> RemoteObject:
        return RemoteObject(
            id=node.id,
            sha512=node.sha512,
            content_length=node.content_length,
            content_type=node.content_type,
            name=node.name,
        )


@document(DocumentType.SECRET)
class Secret(os.Document):
    sha512: str = os.field(os.FT.KEYWORD)
    name: str = NAME_FIELD


@packer(models.Secret, Secret, wire.SecretData)
class SecretPacker(Packer[models.Secret, Secret, wire.SecretData]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.Secret) -> Secret:
        return Secret(id=node.id, sha512=node.sha512, name=node.name)


# global


@document(DocumentType.USER)
class User(os.Document):
    name: str = NAME_FIELD
    slug: str = NAME_FIELD
    email: str = NAME_FIELD


@packer(models.User, User, None)
class UserPacker(Packer[models.User, User, None]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.User) -> User:
        return User(id=node.id, name=node.username, slug=node.slug, email=node.email)


@document(DocumentType.ORGANIZATION)
class Organization(os.Document):
    name: str = NAME_FIELD
    slug: str = NAME_FIELD


@packer(models.Organization, Organization, None)
class OrganizationPacker(Packer[models.Organization, Organization, None]):
    def mirror(
        self, project_v: models.ProjectVersion | None, node: models.Organization
    ) -> Organization:
        return Organization(id=node.id, name=node.name, slug=node.slug)


@document(DocumentType.PROJECT)
class Project(CrudThing, os.Document):
    name: str = NAME_FIELD
    description: str = os.field(os.FT.TEXT)


@packer(models.Project, Project, None)
class ProjectPacker(CrudThingPacker, Packer[models.Project, Project, None]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.Project) -> Project:
        crud = super().mirror(project_v, node)
        return Project(**crud.__dict__, name=node.name, description=node.description)


# module/project content


@document(DocumentType.PROJECT_VERSION)
class ProjectVersion(CrudThing, Revisioned, os.Document):
    project_id: UUID = os.field(os.FT.KEYWORD)
    name: str = NAME_FIELD
    tag: str = NAME_FIELD
    description: str = os.field(os.FT.TEXT)


@packer(models.ProjectVersion, ProjectVersion, None)
class ProjectVersionPacker(CrudThingPacker, Packer[models.ProjectVersion, ProjectVersion, None]):
    def mirror(
        self, project_v: models.ProjectVersion | None, node: models.ProjectVersion
    ) -> ProjectVersion:
        crud = super().mirror(project_v, node)
        return ProjectVersion(
            **crud.__dict__,
            project_id=node.project_id,
            name=node.name,
            tag=node.tag,
            description=node.description,
        )


@document(DocumentType.FILE)
class File(CrudThing, Revisioned, os.Document):
    project_id: UUID = os.field(os.FT.KEYWORD)
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    name: str = NAME_FIELD


@packer(models.File, File, wire.FileData)
class FilePacker(CrudThingPacker, Packer[models.File, File, wire.FileData]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.File) -> File:
        crud = super().mirror(project_v, node)
        return File(
            **crud.__dict__,
            project_version_id=project_v.id,
            project_id=project_v.project_id,
            name=node.name,
        )


@document(DocumentType.STATEMENT)
class Statement(CrudThing, Revisioned, os.Document):
    project_id: UUID = os.field(os.FT.KEYWORD)
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    file_id: UUID = os.field(os.FT.KEYWORD)
    type: StatementType = os.field(os.FT.KEYWORD)
    name: Optional[str] = NAME_FIELD
    description: Optional[str] = os.field(os.FT.TEXT)
    html: Optional[str] = HTML_FIELD
    code: Optional[str] = os.field(os.FT.TEXT)
    # can't index value as it would explode our mappings (module index is global)


@packer(models.Statement, Statement, wire.StatementData)
class StatementPacker(CrudThingPacker, Packer[models.Statement, Statement, wire.StatementData]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.Statement) -> Statement:
        crud = super().mirror(project_v, node)
        return Statement(
            **crud.__dict__,
            project_version_id=node.project_version_id,
            project_id=project_v.project_id,
            file_id=node.file_id,
            type=node.type,
            name=node.name,
            description=node.description,
            html=node.text,
            code=node.code,
        )


@document(DocumentType.FIELD)
class Field(CrudThing, Revisioned, os.Document):
    project_id: UUID = os.field(os.FT.KEYWORD)
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    statement_id: UUID = os.field(os.FT.KEYWORD)
    name: Optional[str] = NAME_FIELD
    type_tag: str = os.field(os.FT.KEYWORD)
    type_hint: Optional[str] = os.field(os.FT.TEXT)


@packer(models.Field, Field, wire.FieldData)
class FieldPacker(CrudThingPacker, Packer[models.Field, Field, wire.FieldData]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.Field) -> Field:
        crud = super().mirror(project_v, node)
        return Field(
            **crud.__dict__,
            project_version_id=node.statement.project_version_id,
            project_id=project_v.project_id,
            statement_id=node.statement_id,
            name=node.name,
            type_tag=node.tag,
            type_hint=node.hint,
        )


@document(DocumentType.TILE)
class Tile(CrudThing, Revisioned, os.Document):
    project_id: UUID = os.field(os.FT.KEYWORD)
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    statement_id: UUID = os.field(os.FT.KEYWORD)
    type: str = os.field(os.FT.KEYWORD)
    name: Optional[str] = NAME_FIELD


# Tile is not packed yet because it is not used yet


@document(DocumentType.RECORD)
class Record(CrudThing, os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    statement_id: UUID = os.field(os.FT.KEYWORD)
    dataset_id: str = os.field(os.FT.KEYWORD)
    order_key: str = os.field(os.FT.KEYWORD)
    # single name field to copy all data names to :RecordNameField
    name: Optional[str] = replace(NAME_FIELD, can_set_directly=False, store=False)
    value: dict = os.field(os.FT.OBJECT, dynamic="strict")  # user defined
    revision: Optional[int] = os.field(os.FT.INTEGER)  # set from OS 'version' on read (for now)


@packer(Record, Record, wire.RecordData)
class RecordPacker(CrudThingPacker, Packer[Record, Record, wire.RecordData]):
    def mirror(self, project_v: models.ProjectVersion | None, node: Record) -> Record:
        return node  # already

    def pack(self, node: Record) -> wire.RecordData:
        return wire.RecordData(
            id=node.id,
            parent_id=node.statement_id,
            order_key=node.order_key,
            value=node.value,
            revision=node.revision,
            created_at=node.created_at,
            updated_at=node.updated_at,
            last_edited_at=node.last_edited_at,
            last_changed_at=None,
        )

    def unpack(
        self, project_v: models.ProjectVersion, data: wire.RecordData, parent: models.Statement
    ) -> Record:
        return Record(
            id=data.id,
            project_version_id=project_v.id,
            statement_id=data.parent_id,
            dataset_id=parent.dataset.backend_id,
            order_key=data.order_key,
            value=data.value,
            revision=data.revision,
            created_at=data.created_at,
            created_by_id=None,
            updated_at=data.updated_at,
            deleted_at=None,
            last_edited_at=data.last_edited_at,
            last_edited_by_id=None,
        )


@document(DocumentType.COMMENT)
class Comment(CrudThing, os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    html: str = HTML_FIELD


# sessions/logs


@document(DocumentType.SESSION)
class Session(os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    opened_at: Optional[datetime] = os.field(os.FT.DATE)
    closed_at: Optional[datetime] = os.field(os.FT.DATE)
    metadata: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined (mostly?)


@packer(models.Session, Session, wire.SessionData)
class SessionPacker(Packer[models.Session, Session, wire.SessionData]):
    def mirror(self, project_v: models.ProjectVersion | None, node: models.Session) -> Session:
        return Session(
            id=node.id,
            project_version_id=node.project_version_id,
            opened_at=node.opened_at,
            closed_at=node.closed_at,
            metadata=node.metadata,
        )


@document(DocumentType.RUN)
class Run(os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    session_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    worker_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    root_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    parent_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    runnable_id: UUID = os.field(os.FT.KEYWORD)
    runnable_type: str = os.field(os.FT.KEYWORD)
    created_at: datetime = os.field(os.FT.DATE)
    updated_at: datetime = os.field(os.FT.DATE)
    started_at: Optional[datetime] = os.field(os.FT.DATE)
    terminated_at: Optional[datetime] = os.field(os.FT.DATE)
    duration: Optional[float] = os.field(os.FT.FLOAT)
    status: str = os.field(os.FT.KEYWORD)
    inputs: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined
    outputs: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined
    error: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined (mostly?)
    metadata: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined (mostly?)


@packer(models.Run, Run, wire.RunData)
class RunPacker(Packer[models.Run, Run, wire.RunData]):
    def unmirror(self, mirror: Run) -> models.Run:
        return models.Run(
            id=mirror.id,
            project_version_id=mirror.project_version_id,
            session_id=mirror.session_id,
            runnable_id=mirror.runnable_id,
            created_at=mirror.created_at,
            updated_at=mirror.updated_at,
            started_at=mirror.started_at,
            terminated_at=mirror.terminated_at,
            status=mirror.status,
            inputs=mirror.inputs,
            outputs=mirror.outputs,
            error=None,
            metadata=mirror.metadata,
        )

    def pack(self, mirror: Run) -> wire.RunData:
        runnable_type = wire.StatementType(mirror.runnable_type) if mirror.runnable_type else None
        return wire.RunData(
            id=mirror.id,
            module_id=mirror.project_version_id,
            worker_id=mirror.worker_id,
            session_id=mirror.session_id,
            root_id=mirror.root_id,
            parent_id=mirror.parent_id,
            runnable_id=mirror.runnable_id,
            runnable_type=runnable_type,
            created_at=mirror.created_at,
            updated_at=mirror.updated_at,
            started_at=mirror.started_at,
            terminated_at=mirror.terminated_at,
            status=wire.RunStatus(mirror.status),
            inputs=mirror.inputs,
            outputs=mirror.outputs,
            error=None,
            metadata=mirror.metadata,
        )

    def unpack(self, project_v: models.ProjectVersion, data: wire.RunData, parent: None) -> Run:
        if data.started_at and data.terminated_at:
            duration = (data.terminated_at - data.started_at).total_seconds()
        else:
            duration = None
        return Run(
            id=data.id,
            project_version_id=project_v.id,
            worker_id=data.worker_id,
            session_id=data.session_id,
            root_id=data.root_id,
            parent_id=data.parent_id,
            runnable_id=data.runnable_id,
            runnable_type=data.runnable_type,
            created_at=data.created_at,
            updated_at=data.updated_at,
            started_at=data.started_at,
            terminated_at=data.terminated_at,
            duration=duration,
            status=data.status,
            inputs=data.inputs,
            outputs=data.outputs,
            error=None,
            metadata=data.metadata,
        )


@document(DocumentType.LOG_ENTRY)
class LogEntry(os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    worker_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    session_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    run_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    runnable_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    created_at: datetime = os.field(os.FT.DATE)
    stream: str = os.field(os.FT.KEYWORD)
    level: Optional[str] = os.field(os.FT.KEYWORD)
    logger: Optional[str] = os.field(os.FT.KEYWORD)
    message: Optional[str] = os.field(os.FT.TEXT)
    metadata: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined (mostly?)


@packer(LogEntry, LogEntry, wire.LogEntryData)
class LogEntryPacker(Packer[LogEntry, LogEntry, wire.LogEntryData]):
    def pack(self, mirror: LogEntry) -> wire.LogEntryData:
        return wire.LogEntryData(
            id=mirror.id,
            module_id=mirror.project_version_id,
            worker_id=mirror.worker_id,
            session_id=mirror.session_id,
            run_id=mirror.run_id,
            runnable_id=mirror.runnable_id,
            created_at=mirror.created_at,
            stream=mirror.stream,
            level=mirror.level,
            logger=mirror.logger,
            message=mirror.message,
            metadata=mirror.metadata,
        )

    def unpack(
        self, project_v: models.ProjectVersion, data: wire.LogEntryData, parent: None
    ) -> LogEntry:
        return LogEntry(
            id=data.id,
            project_version_id=project_v.id,
            worker_id=data.worker_id,
            session_id=data.session_id,
            run_id=data.run_id,
            runnable_id=data.runnable_id,
            created_at=data.created_at,
            stream=data.stream,
            level=data.level,
            logger=data.logger,
            message=data.message,
            metadata=data.metadata,
        )
