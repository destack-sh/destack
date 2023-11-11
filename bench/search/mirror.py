from dataclasses import replace
from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, TypeVar
from uuid import UUID

import bench.search.core as os
from bench.language import StatementType
from bench.language.expression import SubfieldType

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
TEXT_FIELD = HTML_FIELD

ModelT = TypeVar("ModelT", bound=Model)
MirrorT = TypeVar("MirrorT", bound=os.Document)
DataT = TypeVar("DataT", bound=Any)

DOCUMENT_CLASS_BY_TYPE: dict[os.DocumentType, type[os.Document]] = {}


def document(type: os.DocumentType, *, store_type: bool = True):
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


@document(os.DocumentType.BLOB)
class Blob(os.Document):
    #  :BlobType
    sha512: str = os.field(os.FT.KEYWORD)
    content_length: int = os.field(os.FT.LONG)
    content_type: str = os.field(os.FT.KEYWORD)
    name: str = NAME_FIELD
    status: str = os.field(os.FT.KEYWORD)


@document(os.DocumentType.SECRET)
class Secret(os.Document):
    sha512: str = os.field(os.FT.KEYWORD)
    name: str = NAME_FIELD


# global


@document(os.DocumentType.USER)
class User(os.Document):
    name: str = NAME_FIELD
    slug: str = NAME_FIELD
    email: str = NAME_FIELD


@document(os.DocumentType.ORGANIZATION)
class Organization(os.Document):
    name: str = NAME_FIELD
    slug: str = NAME_FIELD


@document(os.DocumentType.PROJECT)
class Project(CrudThing, os.Document):
    name: str = NAME_FIELD
    description: str = os.field(os.FT.TEXT)


# module/project content


@document(os.DocumentType.PROJECT_VERSION)
class ProjectVersion(CrudThing, Revisioned, os.Document):
    project_id: UUID = os.field(os.FT.KEYWORD)
    name: str = NAME_FIELD
    tag: str = NAME_FIELD
    description: str = os.field(os.FT.TEXT)


@document(os.DocumentType.FILE)
class File(CrudThing, Revisioned, os.Document):
    ck: UUID = os.field(os.FT.KEYWORD)
    project_id: UUID = os.field(os.FT.KEYWORD)
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    name: str = NAME_FIELD


@document(os.DocumentType.STATEMENT)
class Statement(CrudThing, Revisioned, os.Document):
    ck: UUID = os.field(os.FT.KEYWORD)
    project_id: UUID = os.field(os.FT.KEYWORD)
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    file_id: UUID = os.field(os.FT.KEYWORD)
    type: StatementType = os.field(os.FT.KEYWORD)
    name: Optional[str] = NAME_FIELD
    text: Optional[str] = TEXT_FIELD
    code: Optional[str] = os.field(os.FT.TEXT)
    # can't index value as it would explode our mappings (module index is global)


@document(os.DocumentType.FIELD)
class Field(CrudThing, Revisioned, os.Document):
    ck: UUID = os.field(os.FT.KEYWORD)
    project_id: UUID = os.field(os.FT.KEYWORD)
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    statement_id: UUID = os.field(os.FT.KEYWORD)
    name: Optional[str] = NAME_FIELD
    text: Optional[str] = TEXT_FIELD
    type_tag: str = os.field(os.FT.KEYWORD)
    type_hint: Optional[str] = os.field(os.FT.TEXT)


@document(os.DocumentType.TILE)
class Tile(CrudThing, Revisioned, os.Document):
    ck: UUID = os.field(os.FT.KEYWORD)
    project_id: UUID = os.field(os.FT.KEYWORD)
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    statement_id: UUID = os.field(os.FT.KEYWORD)
    type: str = os.field(os.FT.KEYWORD)
    name: Optional[str] = NAME_FIELD


@document(os.DocumentType.RECORD)
class Record(CrudThing, os.Document):
    ck: UUID = os.field(os.FT.KEYWORD)
    project_id: UUID = os.field(os.FT.KEYWORD)
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    statement_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    statement_key: Optional[str] = os.field(os.FT.KEYWORD)
    statement_ck: UUID = os.field(os.FT.KEYWORD)
    # single name field to copy all data names to :RecordNameField
    name: Optional[str] = replace(NAME_FIELD, can_set_directly=False, store=False)
    value: dict = os.field(os.FT.OBJECT, dynamic="strict")  # user defined
    revision: Optional[int] = os.field(os.FT.INTEGER)


@document(os.DocumentType.COMMENT)
class Comment(CrudThing, os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)


# sessions/logs


@document(os.DocumentType.SESSION)
class Session(os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    opened_at: Optional[datetime] = os.field(os.FT.DATE)
    closed_at: Optional[datetime] = os.field(os.FT.DATE)
    trigger_type: Optional[str] = os.field(os.FT.KEYWORD)


@document(os.DocumentType.RUN)
class Run(os.Document):
    project_id: UUID = os.field(os.FT.KEYWORD)
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    worker_node_id: Optional[str] = os.field(os.FT.KEYWORD)
    worker_process_id: Optional[str] = os.field(os.FT.KEYWORD)
    session_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    trigger_type: Optional[str] = os.field(os.FT.KEYWORD)
    root_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    parent_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    statement_id: UUID = os.field(os.FT.KEYWORD)
    statement_ck: UUID = os.field(os.FT.KEYWORD)
    statement_type: str = os.field(os.FT.KEYWORD)
    created_at: datetime = os.field(os.FT.DATE)
    updated_at: datetime = os.field(os.FT.DATE)
    scheduled_at: Optional[datetime] = os.field(os.FT.DATE)
    started_at: Optional[datetime] = os.field(os.FT.DATE)
    terminated_at: Optional[datetime] = os.field(os.FT.DATE)
    duration: Optional[float] = os.field(os.FT.FLOAT)
    status: str = os.field(os.FT.KEYWORD)
    inputs: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined
    outputs: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined
    error: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined (mostly?)
    value: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined (mostly?)


@document(os.DocumentType.LOG_ENTRY)
class LogEntry(os.Document):
    project_version_id: UUID = os.field(os.FT.KEYWORD)
    session_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    run_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    statement_id: Optional[UUID] = os.field(os.FT.KEYWORD)
    statement_ck: Optional[UUID] = os.field(os.FT.KEYWORD)
    created_at: datetime = os.field(os.FT.DATE)
    stream: str = os.field(os.FT.KEYWORD)
    level: Optional[str] = os.field(os.FT.KEYWORD)
    logger: Optional[str] = os.field(os.FT.KEYWORD)
    message: Optional[str] = os.field(os.FT.TEXT)
    value: Optional[dict] = os.field(os.FT.OBJECT, dynamic="strict")  # user defined (mostly?)
