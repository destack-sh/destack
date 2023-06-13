import enum
from collections import OrderedDict
from dataclasses import fields, is_dataclass
from datetime import datetime
from itertools import chain
from typing import Any, Union, Optional
from uuid import UUID

from strawberry.utils.str_converters import to_camel_case

from bench import models
from bench.bench import wire
from bench.bench.mutate import MMT, MOT, ModuleMutation, ModuleMutator
from bench.models import packer

MutableThing = Union[
    models.File,
    models.Statement,
    models.Field,
    wire.RecordData,
]

_SCOPE_TO_TYPE_NAME = {
    MOT.FILE: "File",
    MOT.STATEMENT: "Statement",
    MOT.FIELD: "TypeNode",
    MOT.RECORD: "Record",
}


def map_mutation_from_api(
    type: MMT, input: Any, thing: MutableThing
) -> tuple[list[ModuleMutation], list[ModuleMutation]]:
    """
    Remap/create API multiplayer mutation for other clients and internals.
    Returns both the internal and API mutations to publish.
    """
    if isinstance(thing, models.File):
        project_version_id = thing.project_version_id
        file_id = thing.id
        statement_id = None
    elif isinstance(thing, models.Statement):
        project_version_id = thing.project_version_id
        file_id = thing.file_id
        statement_id = thing.id
    elif isinstance(thing, models.Field):
        project_version_id = thing.statement.project_version_id
        file_id = thing.statement.file_id
        statement_id = thing.statement_id
    else:
        raise TypeError(f"thing is not a project thing: {thing}")

    api_mutation = ModuleMutation(
        type=type,
        project_version_id=project_version_id,
        file_id=file_id,
        statement_id=statement_id,
        revision=thing.revision,
        input=input,
    )
    if type == MMT.PASTE_FILE:
        _, nodes_data = packer.pack_node(thing)
        internal = ModuleMutator(module=project_version_id).create_many(*nodes_data)
        api_mutations = list(
            chain.from_iterable(get_api_mutation_from_internal(m) for m in internal.mutations)
        )
        return internal.mutations, api_mutations
    elif type in MMT.PASTE_STATEMENT:  # remap to create children
        _, nodes_data = packer.pack_node(thing)
        internal = ModuleMutator(module=project_version_id, file_id=thing.file_id).create_many(
            *nodes_data
        )
        api_mutations = list(
            chain.from_iterable(get_api_mutation_from_internal(m) for m in internal.mutations)
        )
        return internal.mutations, api_mutations
    else:
        # TODO @Broken: remap restore API mutations for previously offline clients
        #  (restore is insufficient if you don't have the original file?/statement/etc.)
        internal = _get_internal_mutation_from_api(api_mutation, thing)
        return internal, [api_mutation]


def _get_internal_mutation_from_api(
    mutation: ModuleMutation, thing: MutableThing
) -> list[ModuleMutation]:
    """
    Maps the full multiplayer mutation set into simple internal mutations.

    Simple here means only CRUD on full objects (no partial/"atomic" mutations).
    The multiplayer module state and the internal module state are not equally representative,
    so we map some mutations to different mutations.
    (e.g., comment mutations map to create/delete mutations on the statement)
    """

    if mutation.type == MMT.COMMENT_STATEMENT:
        if thing.commented:
            internal_type = MMT.DELETE_STATEMENT  # deletes auto-cascade
        else:
            _, nodes_data = packer.pack_node(thing)
            mut = ModuleMutator(module=mutation.project_version_id)
            return mut.create_many(*nodes_data).mutations
    elif mutation.type == MMT.RESTORE_FILE:
        _, nodes_data = packer.pack_node(thing)
        mut = ModuleMutator(module=mutation.project_version_id)
        return mut.create_many(*nodes_data).mutations
    elif mutation.type == MMT.RESTORE_STATEMENT:
        _, nodes_data = packer.pack_node(thing)
        mut = ModuleMutator(module=mutation.project_version_id, file_id=thing.file_id)
        return mut.create_many(*nodes_data).mutations
    else:
        # map everything else to a simple internal mutation (CUD_X)
        internal_type = MMT(mutation.type.kind + "_" + mutation.mot)

    internal_mutation = ModuleMutation(
        type=internal_type,
        project_version_id=mutation.project_version_id,
        file_id=mutation.file_id,
        statement_id=mutation.statement_id,
        revision=thing.revision,
    )
    internal_mutation.data = packer.pack_node_flat(thing)
    return [internal_mutation]


def get_api_mutation_from_internal(mutation: ModuleMutation) -> list[ModuleMutation]:
    """
    Maps a simple internal mutation to an API multiplayer mutation.

    This is conceptually the inverse of map_mutation_to_internal, but is a bit simpler
    since all internal mutations are also valid API mutations (it's a subset).

    The main challenge is reconstructing an "input" that is exactly the input that
    would have caused the same internal mutation. Note that for some mutations, this
    is a theoretical equivalence, since e.g., hard deletes aren't used in the client.
    (but still exposed for the purpose of internal synchronisation with this right here)
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

_EXTRA_FIELD_RENAMES = {"project_version_id": "module_id"}


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
        s_key, t_key = field.name, field.name
        if s_key in _EXTRA_FIELD_RENAMES:
            s_key = _EXTRA_FIELD_RENAMES[s_key]
        if s_key in extra_fields:
            value = extra_fields[s_key]
        else:
            value = getattr(mutation.data, s_key)
        if isinstance(value, UUID):
            value = _map_id_field(s_key, value, mutation.type.mot)
        input_args[t_key] = value
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


def _map_id_field(key: str, value: UUID, scope: MOT):
    from strawberry_django_plus.relay import GlobalID

    # map id to global id with appropriate type name
    if key == "file_id":
        type_name = "File"
    elif key == "statement_id":
        type_name = "Statement"
    elif key == "parent_id" and scope == MOT.STATEMENT:
        type_name = "Statement"
    elif key == "parent_id" and scope == MOT.FILE:
        type_name = "File"
    else:
        type_name = _SCOPE_TO_TYPE_NAME[scope]
    value = GlobalID(type_name, str(value))
    return value
