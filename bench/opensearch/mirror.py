from opensearchpy import Date, Float, Keyword, Long, Object, Text

import bench.opensearch.type as os

NAME_FIELD = Text(
    fields={
        os.FieldType.SEARCH_AS_YOU_TYPE.value: {
            "type": os.FieldType.SEARCH_AS_YOU_TYPE.value,
        }
    }
)


# basic


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
    email = Keyword()


class Project(os.Document):
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


class File(CrudThing, os.Document):
    project_version_id = Keyword()
    name = NAME_FIELD
    type = Keyword()
    description = Text()
    code = Text()


class Statement(CrudThing, os.Document):
    project_version_id = Keyword()
    file_id = Keyword()
    name = NAME_FIELD
    type = Keyword()
    modifier = Keyword()
    symbol_type = Keyword()
    description = Text()
    code = Text()


class Field(CrudThing, os.Document):
    project_version_id = Keyword()
    statement_id = Keyword()
    name = NAME_FIELD
    tag = Keyword()
    hint = Keyword()


class Screen(CrudThing, os.Document):
    project_version_id = Keyword()
    name = NAME_FIELD
    description = Text()


class Tile(CrudThing, os.Document):
    project_version_id = Keyword()
    name = NAME_FIELD
    description = Text()
    screen_id = Keyword()


class Comment(CrudThing, os.Document):
    pass


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


class LogEntry(os.Document):
    project_version_id = Keyword()
    session_id = Keyword()
    created_at = Date()
    level = Keyword()
    logger = Keyword()
    message = Object()
