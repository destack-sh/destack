import enum
from collections import OrderedDict
from dataclasses import fields, is_dataclass
from datetime import datetime
from itertools import chain
from typing import Any, Optional, Union
from uuid import UUID

from strawberry.utils.str_converters import to_camel_case

from bench import models
from bench.language.mutate import MMT, MNT, ModuleMutation, ModuleMutator
from bench.models import packer
from bench.models.packer import INTERP_MODEL_TYPES
from bench.opensearch import mirror

MutableThing = Union[
    models.File,
    models.Statement,
    models.Field,
    mirror.Record,
]

_MNT_TO_TYPE_NAME = {
    MNT.Module: "Module",
    MNT.File: "File",
    MNT.Statement: "Statement",
    MNT.Field: "Field",
    MNT.ResolvedField: "ResolvedField",
    MNT.Record: "Record",
    MNT.Tagging: "Tagging",
    MNT.Trigger: "Trigger",
    MNT.DatasetView: "DatasetView",
    MNT.DatasetViewField: "DatasetViewField",
    MNT.Comment: "Comment",
    MNT.Issue: "Issue",
}
assert len(_MNT_TO_TYPE_NAME) == len(MNT), f"mismatch: {len(_MNT_TO_TYPE_NAME)} != {len(MNT)}"

MAX_RECORD_MUTATIONS_PER_BATCH = 15


def trim_record_mutations(
    mutations: list[ModuleMutation],
) -> list[ModuleMutation]:
    """Trims record mutations into bumps if necessary."""
    num_record_updates = 0
    bumped_statement_ids: dict[UUID, UUID] = {}  # statement_id -> file_id
    trimmed_mutations = []
    for mutation in mutations:
        if mutation.scope == MNT.Record:
            num_record_updates += 1
            bumped_statement_ids[mutation.statement_id] = mutation.file_id
            if num_record_updates < MAX_RECORD_MUTATIONS_PER_BATCH:
                trimmed_mutations.append(mutation)
        else:
            trimmed_mutations.append(mutation)
    # always add bumps since we don't have proper bump propagation on the frontend yet
    for statement_id, file_id in bumped_statement_ids.items():
        trimmed_mutations.append(
            ModuleMutation(
                type=MMT.BUMP_STATEMENT,
                project_version_id=mutations[0].project_version_id,
                file_id=file_id,
                statement_id=statement_id,
            )
        )
    return trimmed_mutations


def map_mutation_from_api(
    type: MMT,
    input: Any,
    thing: MutableThing,
    project_v: models.ProjectVersion,
    statement: Optional[models.Statement],
) -> tuple[list[ModuleMutation], list[ModuleMutation]]:
    """
    Remap/create API multiplayer mutation for other clients and internals.
    Returns both the internal and API mutations to publish.
    """
    api_mutation = ModuleMutation(
        type=type,
        project_version_id=project_v.id,
        revision=thing.revision,
        input=input,
        thing=thing,
    )
    if statement is not None:
        api_mutation.file_id = statement.file_id
        api_mutation.statement_id = statement.id
    elif isinstance(thing, models.File):
        api_mutation.file_id = thing.id
        api_mutation.statement_id = None
    elif isinstance(thing, models.Statement):
        api_mutation.file_id = thing.file_id
        api_mutation.statement_id = thing.id
    elif isinstance(thing, (models.Field, models.Record, models.Tagging, models.Trigger)):
        api_mutation.file_id = thing.statement.file_id
        api_mutation.statement_id = thing.statement_id
    else:
        raise TypeError(f"thing is not a project thing: {thing}")

    if type in (MMT.PASTE_FILE, MMT.RESTORE_FILE, MMT.PASTE_STATEMENT, MMT.RESTORE_STATEMENT):
        packed = packer.pack_node(thing, excluded=None)
        file_id = thing.file_id if isinstance(thing, models.Statement) else thing.id
        internal = ModuleMutator(module=project_v.id, file_id=file_id).create_many(
            *packed.nodes_list()
        )
        api_mutations = list(
            chain.from_iterable(get_api_mutation_from_internal(m) for m in internal.mutations)
        )
        # strip interp data from internal mutations (but keep in API, user clients need it)
        stripped_internal_mutations = [
            m
            for m in internal.mutations
            if not isinstance(packed.nodes_by_id[m.data.id], INTERP_MODEL_TYPES)
        ]
        return stripped_internal_mutations, api_mutations
    else:
        # map everything else to a simple internal mutation (CUD_X)
        internal_type = MMT(type.kind + "_" + api_mutation.mnt.caps_name)
        internal_mutation = ModuleMutation(
            type=internal_type,
            project_version_id=api_mutation.project_version_id,
            revision=thing.revision,
            thing=thing,
        )
        if isinstance(thing, mirror.Document):  # os indexed Document
            internal_mutation.data = mirror.pack_node_flat(thing)
        else:
            internal_mutation.data = packer.pack_node_flat(thing)
        return [internal_mutation], [api_mutation]


def get_api_mutation_from_internal(mutation: ModuleMutation) -> list[ModuleMutation]:
    """
    Maps a simple internal mutation to an API multiplayer mutation.

    This is conceptually the inverse of map_mutation_to_internal, but is a bit simpler
    since all internal mutations are also valid API mutations (it's a subset).

    The main challenge is reconstructing an "input" that is exactly the input that
    would have caused that mutation. For some mutations, this is theoretical,
    since e.g., hard deletes aren't used in the UX (only for internal synchronisation).
    """
    if not mutation.type.simple:
        raise ValueError(f"mutation is not a simple internal mutation: {mutation}")
    input = get_gql_input_from_mutation(mutation)
    api_mutation = ModuleMutation(
        type=mutation.type,
        project_version_id=mutation.project_version_id,
        file_id=mutation.file_id,
        statement_id=mutation.statement_id,
        revision=mutation.revision,
        input=input,
    )
    if input is None and mutation.data is not None:
        api_mutation.data = mutation.data
    return [api_mutation]


# extra fields in API mutations that are not in internal module data
_EXTRA_FIELDS_BY_SCOPE = {
    MNT.File: {
        "parent_id": None,
        "directory": False,
    },
}


def get_gql_input_from_mutation(mutation: ModuleMutation) -> Optional[dict]:
    """
    Maps a simple internal mutation to an input that would cause the same mutation.
    The returned input is already jsonable (not the original input class).
    """
    from bench.api.sync import INPUT_CLASS_BY_TYPE
    from bench.api.utils import to_global_id

    input_cls = INPUT_CLASS_BY_TYPE.get(mutation.type)
    if input_cls is None:
        return None
    extra_fields = _EXTRA_FIELDS_BY_SCOPE.get(mutation.type.mnt, {})
    input_args = {}
    for field in fields(input_cls):
        if field.name == "project_version_id":
            value = to_global_id("ProjectVersion", mutation.project_version_id)
        elif field.name == "file_id":
            value = to_global_id("File", mutation.file_id)
        elif field.name == "statement_id":
            value = to_global_id("Statement", mutation.statement_id)
        elif field.name in extra_fields:
            value = extra_fields[field.name]
        else:
            value = getattr(mutation.data, field.name, field.default)
            if isinstance(value, UUID):
                value = _map_id_field(field.name, value, mutation)
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


def _map_id_field(key: str, value: UUID, mutation: ModuleMutation):
    # map id to global id with appropriate type name
    # we can't actually know whether parent id is a file or statement id
    # so we check against the mutation file id.. this should be fine?
    from strawberry.relay import GlobalID

    if key == "parent_id" and mutation.type.mnt == MNT.Statement:
        if value == mutation.file_id:
            type_name = "File"
        else:
            type_name = "Statement"
    elif key == "parent_id" and mutation.type.mnt == MNT.File:
        # same as above
        if value == mutation.project_version_id:
            type_name = "ProjectVersion"
        else:
            type_name = "File"
    else:
        type_name = _MNT_TO_TYPE_NAME[mutation.type.mnt]
    value = GlobalID(type_name, str(value))
    return value
