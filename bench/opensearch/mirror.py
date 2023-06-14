import enum
from datetime import datetime
from typing import Any, Generic, Optional, TypeVar
from uuid import UUID

from django.db.models import Model

import bench.opensearch.type as os
from bench import models
from bench.bench import StatementType, wire

NAME_FIELD = os.Field(
    os.FT.TEXT,
    fields={
        os.FieldType.SEARCH_AS_YOU_TYPE: os.Field(os.FieldType.SEARCH_AS_YOU_TYPE),
        os.FieldType.KEYWORD: os.Field(os.FieldType.KEYWORD),
    },
)
HTML_FIELD = os.Field(os.FieldType.TEXT, analyzer=os.Analyzer.HTML)

ModelT = TypeVar("ModelT", bound=Model)
MirrorT = TypeVar("MirrorT", bound=os.Document)
DataT = TypeVar("DataT", bound=Any)


class Packer(Generic[ModelT, MirrorT, DataT]):
    def mirror(self, node: ModelT) -> MirrorT:
        raise NotImplementedError

    def pack(self, mirror: MirrorT) -> DataT:
        raise NotImplementedError

    def unpack(self, data: DataT) -> MirrorT:
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
        if data_t is not None:
            _packers_by_data[data_t] = packer
        return cls

    return decorator


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
    def mirror(self, node: ModelT) -> MirrorT:
        return CrudThing(
            created_at=node.created_at,
            updated_at=node.updated_at,
            deleted_at=node.deleted_at,
            created_by_id=node.created_by_id,
            last_edited_at=node.last_edited_at,
            last_edited_by_id=node.last_edited_by_id,
            revision=node.revision,
        )


@os.document
class RemoteObject(os.Document):
    sha512: str = os.field(os.FT.KEYWORD)
    content_length: int = os.field(os.FT.LONG)
    content_type: str = os.field(os.FT.KEYWORD)
    name: str = NAME_FIELD


@packer(models.RemoteObject, RemoteObject, wire.RemoteObjectData)
class RemoteObjectPacker(Packer[models.RemoteObject, RemoteObject, wire.RemoteObjectData]):
    def mirror(self, node: models.RemoteObject) -> RemoteObject:
        return RemoteObject(
            sha512=node.sha512,
            content_length=node.content_length,
            content_type=node.content_type,
            name=node.name,
        )


@os.document
class Secret(os.Document):
    sha512: str = os.field(os.FT.KEYWORD)
    name: str = NAME_FIELD


@packer(models.Secret, Secret, wire.SecretData)
class SecretPacker(Packer[models.Secret, Secret, wire.SecretData]):
    def mirror(self, node: models.Secret) -> Secret:
        return Secret(sha512=node.sha512, name=node.name)


# global


class OwnerType(enum.StrEnum):
    USER = "user"
    ORGANIZATION = "organization"


@os.document
class User(os.Document):
    name: str = NAME_FIELD
    slug: str = NAME_FIELD
    type: str = os.field(os.FT.KEYWORD)
    email: str = NAME_FIELD


@packer(models.User, User, None)
class UserPacker(Packer[models.User, User, None]):
    def mirror(self, node: models.User) -> User:
        return User(name=node.username, slug=node.slug, type=OwnerType.USER.value, email=node.email)


@os.document
class Organization(os.Document):
    name: str = NAME_FIELD
    slug: str = NAME_FIELD
    type: str = os.field(os.FT.KEYWORD)


@packer(models.Organization, Organization, None)
class OrganizationPacker(Packer[models.Organization, Organization, None]):
    def mirror(self, node: models.Organization) -> Organization:
        return Organization(name=node.name, slug=node.slug, type=OwnerType.ORGANIZATION.value)


@os.document
class Project(CrudThing, os.Document):
    name: str = NAME_FIELD
    description: str = os.field(os.FT.TEXT)


@packer(models.Project, Project, None)
class ProjectPacker(CrudThingPacker, Packer[models.Project, Project, None]):
    def mirror(self, node: models.Project) -> Project:
        crud = super().mirror(node)
        return Project(**crud.__dict__, name=node.name, description=node.description)


# module/project content


@os.document
class ProjectVersion(CrudThing, Revisioned, os.Document):
    name: str = NAME_FIELD
    tag: str = NAME_FIELD
    description: str = os.field(os.FT.TEXT)


@packer(models.ProjectVersion, ProjectVersion, None)
class ProjectVersionPacker(CrudThingPacker, Packer[models.ProjectVersion, ProjectVersion, None]):
    def mirror(self, node: models.ProjectVersion) -> ProjectVersion:
        crud = super().mirror(node)
        return ProjectVersion(
            **crud.__dict__, name=node.name, tag=node.tag, description=node.description
        )


@os.document
class File(CrudThing, Revisioned, os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    name: str = NAME_FIELD
    type: str = os.field(os.FT.KEYWORD)


@packer(models.File, File, wire.FileData)
class FilePacker(CrudThingPacker, Packer[models.File, File, wire.FileData]):
    def mirror(self, node: models.File) -> File:
        crud = super().mirror(node)
        return File(**crud.__dict__, project_version_id=node.project_version_id, name=node.name)


@os.document
class Statement(CrudThing, Revisioned, os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    file_id: UUID = os.field(os.FT.KEYWORD)
    type: StatementType = os.field(os.FT.KEYWORD)
    name: Optional[str] = NAME_FIELD
    description: Optional[str] = os.field(os.FT.TEXT)
    html: Optional[str] = HTML_FIELD
    code: Optional[str] = os.field(os.FT.TEXT)
    value: Optional[dict] = os.field(os.FT.OBJECT, dynamic=False)  # user defined


@packer(models.Statement, Statement, wire.StatementData)
class StatementPacker(CrudThingPacker, Packer[models.Statement, Statement, wire.StatementData]):
    def mirror(self, node: models.Statement) -> Statement:
        crud = super().mirror(node)
        return Statement(
            **crud.__dict__,
            project_version_id=node.project_version_id,
            file_id=node.file_id,
            type=node.type,
            name=node.name,
            description=node.description,
            html=node.text,
            code=node.code,
            value=node.value,
        )


@os.document
class Field(CrudThing, Revisioned, os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    statement_id: UUID = os.field(os.FT.KEYWORD)
    name: Optional[str] = NAME_FIELD
    tag: str = os.field(os.FT.KEYWORD)
    hint: Optional[str] = os.field(os.FT.TEXT)


@packer(models.Field, Field, wire.FieldData)
class FieldPacker(CrudThingPacker, Packer[models.Field, Field, wire.FieldData]):
    def mirror(self, node: models.Field) -> Field:
        crud = super().mirror(node)
        return Field(
            **crud.__dict__,
            project_version_id=node.statement.project_version_id,
            statement_id=node.statement_id,
            name=node.name,
            tag=node.tag,
            hint=node.hint,
        )


@os.document
class Tile(CrudThing, Revisioned, os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    statement_id: UUID = os.field(os.FT.KEYWORD)
    type: str = os.field(os.FT.KEYWORD)
    name: Optional[str] = NAME_FIELD


# not packed yet because it is not used yet


@os.document
class Record(CrudThing, os.Document):  # uses seq number as revision
    statement_id: UUID = os.field(os.FT.KEYWORD)
    order_key: str = os.field(os.FT.KEYWORD)
    name: Optional[str] = NAME_FIELD  # single name field to copy all data names to
    data: dict = os.field(os.FT.OBJECT, dynamic=False)  # user defined


@packer(Record, Record, wire.RecordData)
class RecordPacker(CrudThingPacker, Packer[Record, Record, wire.RecordData]):
    def mirror(self, node: Record) -> Record:
        return node  # already

    def pack(self, node: Record) -> wire.RecordData:
        return wire.RecordData(
            id=node.id,
            parent_id=node.statement_id,
            order_key=node.order_key,
            name=node.name,
            data=node.data,
        )


@os.document
class Comment(CrudThing, os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    html: str = HTML_FIELD


# sessions/logs


@os.document
class Session(os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    created_at: datetime = os.field(os.FT.DATE)
    updated_at: datetime = os.field(os.FT.DATE)
    started_at: Optional[datetime] = os.field(os.FT.DATE)
    terminated_at: Optional[datetime] = os.field(os.FT.DATE)
    cached_generated_at: Optional[datetime] = os.field(os.FT.DATE)
    cached_duration: Optional[float] = os.field(os.FT.FLOAT)
    duration: Optional[float] = os.field(os.FT.FLOAT)
    status: str = os.field(os.FT.KEYWORD)


@os.document
class Execution(os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    session_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    runnable_id: UUID = os.field(os.FT.KEYWORD)
    created_at: datetime = os.field(os.FT.DATE)
    updated_at: datetime = os.field(os.FT.DATE)
    started_at: Optional[datetime] = os.field(os.FT.DATE)
    terminated_at: Optional[datetime] = os.field(os.FT.DATE)
    cached_generated_at: Optional[datetime] = os.field(os.FT.DATE)
    cached_duration: Optional[float] = os.field(os.FT.FLOAT)
    duration: Optional[float] = os.field(os.FT.FLOAT)
    status: str = os.field(os.FT.KEYWORD)
    inputs: Optional[dict] = os.field(os.FT.OBJECT, dynamic=False)  # user defined
    outputs: Optional[dict] = os.field(os.FT.OBJECT, dynamic=False)  # user defined
    metadata: Optional[dict] = os.field(os.FT.OBJECT, dynamic=False)  # user defined (mostly)


@os.document
class LogEntry(os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    session_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    created_at: datetime = os.field(os.FT.DATE)
    level: str = os.field(os.FT.KEYWORD)
    logger: str = os.field(os.FT.KEYWORD)
    message: str = os.field(os.FT.TEXT)
