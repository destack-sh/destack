import enum
from collections import OrderedDict
from dataclasses import fields, is_dataclass
from datetime import datetime
from itertools import chain
from typing import Any, Optional, Union
from uuid import UUID

from strawberry.utils.str_converters import to_camel_case

from bench import models
from bench.language.edit import MET, MNT, Edit, ModuleEditor
from bench.language.module import NodeTree
from bench.models import packer
from bench.models.packer import INTERP_MODEL_TYPES
from bench.opensearch import mirror

MutableThing = Union[
    models.File,
    models.Statement,
    models.Field,
    mirror.Record,
]


MAX_RECORD_MUTATIONS_PER_BATCH = 15


def trim_record_edits(
    edits: list[Edit],
) -> list[Edit]:
    """Trims record edits into bumps if necessary."""
    num_record_updates = 0
    bumped_statement_ids: dict[UUID, UUID] = {}  # statement_id -> file_id
    trimmed_edits = []
    for edit in edits:
        if edit.scope == MNT.Record:
            num_record_updates += 1
            bumped_statement_ids[edit.statement_id] = edit.file_id
            if num_record_updates < MAX_RECORD_MUTATIONS_PER_BATCH:
                trimmed_edits.append(edit)
        else:
            trimmed_edits.append(edit)
    # always add bumps since we don't have proper bump propagation on the frontend yet
    for statement_id, file_id in bumped_statement_ids.items():
        trimmed_edits.append(
            Edit(
                type=MET.BUMP_STATEMENT,
                project_version_id=edits[0].project_version_id,
                file_id=file_id,
                statement_id=statement_id,
            )
        )
    return trimmed_edits


def map_edit_from_api(
    type: MET,
    input: Any,
    thing: MutableThing,
    project_v: models.ProjectVersion,
    statement: Optional[models.Statement],
) -> tuple[list[Edit], list[Edit]]:
    """
    Remap/create API multiplayer edit for other clients and internals.
    Returns both the internal and API edits to publish.
    """
    api_edit = Edit(
        type=type,
        project_version_id=project_v.id,
        revision=thing.revision,
        input=input,
        thing=thing,
    )
    if statement is not None:
        api_edit.file_id = statement.file_id
        api_edit.statement_id = statement.id
    elif isinstance(thing, models.File):
        api_edit.file_id = thing.id
        api_edit.statement_id = None
    elif isinstance(thing, models.Statement):
        api_edit.file_id = thing.file_id
        api_edit.statement_id = thing.id
    elif isinstance(thing, (models.Field, models.Record, models.Tagging, models.Trigger)):
        api_edit.file_id = thing.statement.file_id
        api_edit.statement_id = thing.statement_id
    else:
        raise TypeError(f"thing is not a project thing: {thing}")

    if type in (MET.PASTE_FILE, MET.RESTORE_FILE, MET.PASTE_STATEMENT, MET.RESTORE_STATEMENT):
        packed = packer.pack_node(thing, excluded=[models.Record, *INTERP_MODEL_TYPES])
        file_id = thing.file_id if isinstance(thing, models.Statement) else thing.id
        editor = ModuleEditor(
            tree=NodeTree(),
            project_id=project_v.project_id,
            module_id=project_v.id,
            file_id=file_id,
        )
        internal = editor.create_many(*packed.nodes_list())
        api_edits = list(chain.from_iterable(get_api_edit_from_internal(m) for m in internal.edits))
        # strip interp data from internal edits (but keep in API, user clients need it)
        stripped_internal_edits = [
            m
            for m in internal.edits
            if not isinstance(packed.nodes_by_id[m.data.id], INTERP_MODEL_TYPES)
        ]
        return stripped_internal_edits, api_edits
    else:
        # map everything else to a simple internal edit (CUD_X)
        internal_type = MET(type.kind + "_" + api_edit.mnt.caps_name)
        internal_edit = Edit(
            type=internal_type,
            project_version_id=api_edit.project_version_id,
            revision=thing.revision,
            thing=thing,
        )
        if isinstance(thing, mirror.Document):  # os indexed Document
            internal_edit.data = mirror.pack_node_flat(thing)
        else:
            internal_edit.data = packer.pack_node_flat(thing)
        return [internal_edit], [api_edit]


def get_api_edit_from_internal(edit: Edit) -> list[Edit]:
    """
    Maps a simple internal edit to an API multiplayer edit.

    This is conceptually the inverse of map_edit_to_internal, but is a bit simpler
    since all internal edits are also valid API edits (it's a subset).

    The main challenge is reconstructing an "input" that is exactly the input that
    would have caused that edit. For some edits, this is theoretical,
    since e.g., hard deletes aren't used in the UX (only for internal synchronisation).
    """
    if not edit.type.simple:
        raise ValueError(f"edit is not a simple internal edit: {edit}")
    input = get_gql_input_from_edit(edit)
    api_edit = Edit(
        type=edit.type,
        project_version_id=edit.project_version_id,
        file_id=edit.file_id,
        statement_id=edit.statement_id,
        revision=edit.revision,
        input=input,
    )
    if input is None and edit.data is not None:
        api_edit.data = edit.data
    return [api_edit]


# extra fields in API edits that are not in internal module data
_EXTRA_FIELDS_BY_SCOPE = {
    MNT.File: {
        "parent_id": None,
        "directory": False,
    },
}


def get_gql_input_from_edit(edit: Edit) -> Optional[dict]:
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
            value = getattr(edit.data, field.name, field.default)
            if field.name != "ck" and isinstance(value, UUID):
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
    elif isinstance(value, enum.Enum):
        return value.name  # GQL enums use the name
    elif isinstance(value, (int, float, str, bool, type(None))):
        return value
    elif isinstance(value, (UUID, datetime)):
        return str(value)
    else:
        raise TypeError(f"unexpected value: {value}")


def _map_id_field(key: str, value: UUID, edit: Edit):
    # map id to global id with appropriate type name
    # we can't actually know whether parent id is a file or statement id,
    # so we check against the edit file id... this should be fine?
    from strawberry.relay import GlobalID

    if key == "parent_id" and edit.type.mnt == MNT.Statement:
        if value == edit.file_id:
            type_name = "File"
        else:
            type_name = "Statement"
    elif key == "parent_id" and edit.type.mnt == MNT.File:
        # same as above
        if value == edit.project_version_id:
            type_name = "ProjectVersion"
        else:
            type_name = "File"
    else:
        type_name = edit.type.mnt
    value = GlobalID(type_name, str(value))
    return value
