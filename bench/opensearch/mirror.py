from opensearchpy import Date, Document, Float, Keyword, Long, Object, Text

import bench.opensearch.type as os

NAME_FIELD = Text(
    fields={os.FieldType.SEARCH_AS_YOU_TYPE: os.Field(os.FieldType.SEARCH_AS_YOU_TYPE)}
)


# basic


class RemoteObject(Document):
    sha512 = Keyword()
    content_length = Long()
    content_type = Keyword()
    name = NAME_FIELD


class Secret(Document):
    sha512 = Keyword()
    name = NAME_FIELD


# module/project content


class CrudThing:
    created_at = Date()
    updated_at = Date()
    deleted_at = Date()
    created_by_id = Keyword()
    last_edited_at = Date()
    last_edited_by_id = Keyword()
    revision = Long(index=False)


class File(CrudThing, Document):
    project_version_id = Keyword()
    name = NAME_FIELD
    type = Keyword()
    description = Text()
    code = Text()


class Statement(CrudThing, Document):
    project_version_id = Keyword()
    file_id = Keyword()
    name = NAME_FIELD
    type = Keyword()
    modifier = Keyword()
    symbol_type = Keyword()
    description = Text()
    code = Text()


class Field(CrudThing, Document):
    project_version_id = Keyword()
    statement_id = Keyword()
    name = NAME_FIELD
    tag = Keyword()
    hint = Keyword()


# sessions


class Execution(Document):
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


# logs


class LogEntry(Document):
    project_version_id = Keyword()
    session_id = Keyword()
    created_at = Date()
    level = Keyword()
    logger = Keyword()
    message = Object()
