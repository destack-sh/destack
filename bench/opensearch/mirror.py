from typing import TypeVar, Generic

from django.db.models import Model
from opensearchpy import Date, Float, Keyword, Long, Object, Text

from bench import models
from bench.bench import wire
import bench.opensearch.type as os

NAME_FIELD = Text(
    fields={
        os.FieldType.SEARCH_AS_YOU_TYPE.value: {
            "type": os.FieldType.SEARCH_AS_YOU_TYPE.value,
        },
        os.FieldType.KEYWORD.value: {
            "type": os.FieldType.KEYWORD.value,
        },
    }
)
HTML_FIELD = Text(char_filter=["html_strip"])

NodeT = TypeVar("NodeT", bound=Model)
MirrorT = TypeVar("MirrorT", bound=os.Document)
DataT = TypeVar("DataT", bound=wire.NodeData)


class NodePacker(Generic[NodeT, MirrorT, DataT]):
    def mirror(self, node: NodeT) -> MirrorT:
        raise NotImplementedError

    def pack(self, mirror: MirrorT) -> DataT:
        raise NotImplementedError

    def unpack(self, data: DataT) -> MirrorT:
        raise NotImplementedError


# basic

Document = os.Document


class CrudThing(os.Document):
    created_at = Date()
    updated_at = Date()
    deleted_at = Date(ignore_malformed=True)  # to allow reset to 'null'  via invalid date
    created_by_id = Keyword()
    last_edited_at = Date()
    last_edited_by_id = Keyword()


class Revisioned(os.Document):
    revision = Long(index=False)


class CrudThingPacker(NodePacker):
    def mirror(self, node: NodeT) -> MirrorT:
        return CrudThing(
            created_at=node.created_at,
            updated_at=node.updated_at,
            deleted_at=node.deleted_at,
            created_by_id=node.created_by_id,
            last_edited_at=node.last_edited_at,
            last_edited_by_id=node.last_edited_by_id,
            revision=node.revision,
        )


class RemoteObject(os.Document):
    sha512 = Keyword()
    content_length = Long()
    content_type = Keyword()
    name = NAME_FIELD


class Secret(os.Document):
    sha512 = Keyword()
    name = NAME_FIELD


# global


class Owner(os.Document):
    name = NAME_FIELD
    slug = NAME_FIELD
    type = Keyword()
    email = NAME_FIELD


class Project(CrudThing, os.Document):
    name = NAME_FIELD
    description = Text()


# module/project content


class File(CrudThing, Revisioned, os.Document):
    project_version_id = Keyword()
    name = NAME_FIELD
    type = Keyword()


class FilePacker(CrudThingPacker, NodePacker[models.File, File, wire.FileData]):
    def mirror(self, node: models.File) -> File:
        crud = super().mirror(node)
        return File(
            **crud.__dict__,
            project_version_id=node.project_version_id,
            name=node.name,
        )


class Statement(CrudThing, Revisioned, os.Document):
    project_version_id = Keyword()
    file_id = Keyword()
    type = Keyword()
    name = NAME_FIELD
    description = Text()
    html = HTML_FIELD
    code = Text()
    value = Object(dynamic=False)  # user defined


class Field(CrudThing, Revisioned, os.Document):
    project_version_id = Keyword()
    statement_id = Keyword()
    name = NAME_FIELD
    tag = Keyword()
    hint = Keyword()


class Tile(CrudThing, Revisioned, os.Document):
    project_version_id = Keyword()
    statement_id = Keyword()
    type = Keyword()
    name = NAME_FIELD
    description = Text()


class Record(CrudThing, os.Document):  # uses seq number as revision
    statement_id = Keyword()
    order_key = Keyword()
    name = NAME_FIELD  # single name field to copy all data names to
    data = Object(dynamic=False)  # user defined


class Comment(CrudThing, os.Document):
    html = HTML_FIELD


# sessions/logs


class Session(os.Document):
    project_version_id = Keyword()
    created_at = Date()
    updated_at = Date()
    started_at = Date()
    terminated_at = Date()
    cached_generated_at = Date()
    cached_duration = Float()
    duration = Float()
    status = Keyword()


class Execution(os.Document):
    project_version_id = Keyword()
    session_id = Keyword()
    runnable_id = Keyword()
    created_at = Date()
    updated_at = Date()
    started_at = Date()
    terminated_at = Date()
    cached_generated_at = Date()
    cached_duration = Float()
    duration = Float()
    status = Keyword()
    inputs = Object(dynamic=False)  # user defined
    outputs = Object(dynamic=False)  # user defined
    metadata = Object(dynamic=False)  # user defined (mostly)


class LogEntry(os.Document):
    project_version_id = Keyword()
    session_id = Keyword()
    created_at = Date()
    level = Keyword()
    logger = Keyword()
    message = Object(dynamic=False)
