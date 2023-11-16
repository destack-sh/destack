import enum
from collections import OrderedDict
from dataclasses import fields, is_dataclass
from datetime import datetime
from typing import Any, Optional, Union
from uuid import UUID

from strawberry.utils.str_converters import to_camel_case

from bench import models
from bench.language.edit import MNT, EditData
from bench.search import mirror

MutableThing = Union[
    models.File,
    models.Statement,
    models.Field,
    mirror.Record,
]

MAX_RECORD_MUTATIONS_PER_BATCH = 15


def get_api_edit_from_internal(edit: EditData) -> EditData:
    """
    Maps a simple internal edit to an API multiplayer edit.

    This is conceptually the inverse of map_edit_to_internal, but is a bit simpler
    since all internal edits are also valid API edits (it's a subset).

    The main challenge is reconstructing an "input" that is exactly the input that
    would have caused that edit. For some edits, this is theoretical,
    since e.g., hard deletes aren't used in the UX (only for internal synchronisation).
    """
    input = get_gql_input_from_edit(edit)
    api_edit = EditData(
        type=edit.type,
        project_version_id=edit.project_version_id,
        file_id=edit.file_id,
        statement_id=edit.statement_id,
        revision=edit.revision,
        input=input,
    )
    if input is None and edit.node is not None:
        api_edit.node = edit.node
    return api_edit


# extra fields in API edits that are not in internal module data
_EXTRA_FIELDS_BY_SCOPE = {MNT.FILE: {"parent_id": None, "directory": False}}


def get_gql_input_from_edit(edit: EditData) -> Optional[dict]:
    """
    Maps a simple internal edit to an input that would cause the same edit.
    The returned input is already jsonable (not the original input class).
    """
    from bench.api.sync import INPUT_CLASS_BY_TYPE
    from bench.api.utils import to_global_id

    input_cls = INPUT_CLASS_BY_TYPE.get(edit.type)
    if input_cls is None:
        return None
    extra_fields = _EXTRA_FIELDS_BY_SCOPE.get(edit.type.mnt, {})
    input_args = {}
    for field in fields(input_cls):
        if field.name == "project_version_id":
            value = to_global_id("ProjectVersion", edit.project_version_id)
        elif field.name == "file_id":
            value = to_global_id("File", edit.file_id)
        elif field.name == "statement_id":
            value = to_global_id("Statement", edit.statement_id)
        elif field.name in ("statement_ck", "statement_key"):
            value = None  # incorrect, but not actually used in frontend and edits will be overhauled soon
        elif field.name in extra_fields:
            value = extra_fields[field.name]
        else:
            value = getattr(edit.node, field.name, field.default)
            if field.name != "ck" and not field.name.endswith("_ck") and isinstance(value, UUID):
                value = _map_id_field(field.name, value, edit)
        input_args[field.name] = value
    input = input_cls(**input_args)
    input = input_to_gql_jsonable(input)
    return input


def input_to_gql_jsonable(value: Any) -> Any:
    """
    Walk and transform a GraphQL input into a JSON object that can be parsed into that input.
    Also rename keys from snake_case to camelCase.
    """
    from strawberry.relay import GlobalID

    if isinstance(value, GlobalID):
        return str(value)
    elif is_dataclass(value):
        data = OrderedDict()
        for field in fields(value):
            target_key = to_camel_case(field.name)
            data[target_key] = input_to_gql_jsonable(getattr(value, field.name))
        return data
    elif isinstance(value, (list, tuple)):
        return [input_to_gql_jsonable(item) for item in value]
    elif isinstance(value, dict):  # JSON
        return value
    elif isinstance(value, enum.IntFlag):
        return int(value)  # flag enums just use the value
    elif isinstance(value, enum.Enum):
        return value.name  # GQL enums use the name
    elif isinstance(value, (int, float, str, bool, type(None))):
        return value
    elif isinstance(value, (UUID, datetime)):
        return str(value)
    else:
        raise TypeError(f"unexpected value: {value}")


def _map_id_field(key: str, value: UUID, edit: EditData):
    # map id to global id with appropriate type name
    # we can't actually know whether parent id is a file or statement id,
    # so we check against the edit file id... this should be fine?
    from strawberry.relay import GlobalID

    if key == "parent_id" and edit.type.mnt == MNT.STATEMENT:
        if value == edit.file_id:
            type_name = "File"
        else:
            type_name = "Statement"
    elif key == "parent_id" and edit.type.mnt == MNT.FILE:
        # same as above
        if value == edit.project_version_id:
            type_name = "ProjectVersion"
        else:
            type_name = "File"
    else:
        type_name = edit.type.mnt
    value = GlobalID(type_name, str(value))
    return value
