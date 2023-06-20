import enum
from collections import OrderedDict
from dataclasses import fields, is_dataclass
from datetime import datetime
from itertools import chain
from typing import Any, Optional, Union
from uuid import UUID

from strawberry.utils.str_converters import to_camel_case

from bench import models
from bench.bench.mutate import MMT, MOT, ModuleMutation, ModuleMutator
from bench.models import packer
from bench.opensearch import mirror

MutableThing = Union[
    models.File,
    models.Statement,
    models.Field,
    mirror.Record,
]

_SCOPE_TO_TYPE_NAME = {
    MOT.FILE: "File",
    MOT.STATEMENT: "Statement",
    MOT.FIELD: "TypeNode",
    MOT.RECORD: "Record",
}

MAX_RECORD_MUTATIONS_PER_BATCH = 50


def trim_record_mutations(
    mutations: list[ModuleMutation],
) -> list[ModuleMutation]:
    """Trims record mutations into bumps if necessary."""
    num_record_updates = 0
    bumped_statement_ids: dict[UUID, UUID] = {}  # statement_id -> file_id
    trimmed_mutations = []
    for mutation in mutations:
        if mutation.scope == MOT.RECORD:
            num_record_updates += 1
            bumped_statement_ids[mutation.statement_id] = mutation.file_id
            if num_record_updates < MAX_RECORD_MUTATIONS_PER_BATCH:
                trimmed_mutations.append(mutation)
        else:
            trimmed_mutations.append(mutation)
    if num_record_updates >= MAX_RECORD_MUTATIONS_PER_BATCH:
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
    elif isinstance(thing, models.Field):
        api_mutation.file_id = thing.statement.file_id
        api_mutation.statement_id = thing.statement_id
    else:
        raise TypeError(f"thing is not a project thing: {thing}")

    if type in (MMT.PASTE_FILE, MMT.RESTORE_FILE, MMT.PASTE_STATEMENT, MMT.RESTORE_STATEMENT) or (
        type == MMT.COMMENT_STATEMENT and not thing.commented
    ):
        packed = packer.pack_node(thing)
        file_id = thing.file_id if isinstance(thing, models.Statement) else thing.id
        internal = ModuleMutator(module=project_v.id, file_id=file_id).create_many(
            *packed.nodes_list()
        )
        api_mutations = list(
            chain.from_iterable(get_api_mutation_from_internal(m) for m in internal.mutations)
        )
        return internal.mutations, api_mutations
    else:
        if type == MMT.COMMENT_STATEMENT:  # comment -> delete internally
            type = MMT.DELETE_STATEMENT
        # map everything else to a simple internal mutation (CUD_X)
        internal_type = MMT(type.kind + "_" + api_mutation.mot)
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
    if input is None:
        api_mutation.data = mutation.data
    return [api_mutation]


# extra fields in API mutations that are not in internal module data
_EXTRA_FIELDS_BY_SCOPE = {
    MOT.STATEMENT: {
        "commented": False,
    },
    MOT.FILE: {
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

    if mutation.data is None:
        raise ValueError(f"mutation has no data: {mutation}")

    extra_fields = _EXTRA_FIELDS_BY_SCOPE.get(mutation.type.mot, {})
    input_cls = INPUT_CLASS_BY_TYPE[mutation.type]
    input_args = {}
    for field in fields(input_cls):
        if field.name == "project_version_id":
            value = mutation.project_version_id
        elif field.name == "file_id":
            value = mutation.file_id
        elif field.name == "statement_id":
            value = mutation.statement_id
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
    from strawberry_django_plus.relay import GlobalID

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
    from strawberry_django_plus.relay import GlobalID

    # map id to global id with appropriate type name
    if key == "file_id":
        type_name = "File"
    elif key == "statement_id":
        type_name = "Statement"
    elif key == "parent_id" and mutation.type.mot == MOT.STATEMENT:
        # we can't actually know whether parent id is a file or statement id
        # so we check against the mutation file id.. this should be fine?
        if value == mutation.file_id:
            type_name = "File"
        else:
            type_name = "Statement"
    elif key == "parent_id" and mutation.type.mot == MOT.FILE:
        # same as above
        if value == mutation.project_version_id:
            type_name = "ProjectVersion"
        else:
            type_name = "File"
    else:
        type_name = _SCOPE_TO_TYPE_NAME[mutation.type.mot]
    value = GlobalID(type_name, str(value))
    return value
