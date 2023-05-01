import enum
from collections import OrderedDict
from dataclasses import fields, is_dataclass
from datetime import datetime
from itertools import chain
from typing import Any, Union
from uuid import UUID

from strawberry.utils.str_converters import to_camel_case

from bench import models
from bench.language.mutate import MMS, MMT, ModuleMutation, ModuleMutator
from bench.models import mapper

MutableThing = Union[
    models.File,
    models.Statement,
    models.SimpleTypeNode,
    models.DatasetRecord,
    models.XBlock,
]

# refer to ModuleMutationType and _MODULE_MUTATION_MAP
_TRIVIAL_PUBLIC_TO_INTERNAL = {
    # Files
    MMT.CREATE_FILE: MMT.CREATE_FILE,
    MMT.RENAME_FILE: MMT.UPDATE_FILE,
    MMT.MOVE_FILE: MMT.UPDATE_FILE,
    MMT.UPDATE_FILE: MMT.UPDATE_FILE,
    MMT.DELETE_FILE: MMT.DELETE_FILE,
    MMT.SOFT_DELETE_FILE: MMT.DELETE_FILE,
    # (non-trivial: restore)
    # Statements
    MMT.CREATE_STATEMENT: MMT.CREATE_STATEMENT,
    MMT.CREATE_STATEMENT_BLANK: MMT.CREATE_STATEMENT,
    MMT.MOVE_STATEMENT: MMT.UPDATE_STATEMENT,
    MMT.UPDATE_STATEMENT: MMT.UPDATE_STATEMENT,
    MMT.UPDATE_STATEMENT_MODIFIER: MMT.UPDATE_STATEMENT,
    MMT.UPDATE_STATEMENT_REFERENCE: MMT.UPDATE_STATEMENT,
    MMT.MORPH_STATEMENT: MMT.UPDATE_STATEMENT,
    MMT.RENAME_STATEMENT: MMT.UPDATE_STATEMENT,
    MMT.UPDATE_STATEMENT_TEXT: MMT.UPDATE_STATEMENT,
    MMT.UPDATE_STATEMENT_DESCRIPTION: MMT.UPDATE_STATEMENT,
    MMT.UPDATE_STATEMENT_CODE: MMT.UPDATE_STATEMENT,
    MMT.UPDATE_STATEMENT_LANGUAGE: MMT.UPDATE_STATEMENT,
    MMT.DELETE_STATEMENT: MMT.DELETE_STATEMENT,
    MMT.SOFT_DELETE_STATEMENT: MMT.DELETE_STATEMENT,
    # (non-trivial: restore, comment)
    # Types
    MMT.CREATE_TYPE_NODE: MMT.CREATE_TYPE_NODE,
    MMT.UPDATE_TYPE_NODE: MMT.UPDATE_TYPE_NODE,
    MMT.MOVE_TYPE_NODE: MMT.UPDATE_TYPE_NODE,
    MMT.DELETE_TYPE_NODE: MMT.DELETE_TYPE_NODE,
    MMT.SOFT_DELETE_TYPE_NODE: MMT.DELETE_TYPE_NODE,
    MMT.RESTORE_TYPE_NODE: MMT.UPDATE_TYPE_NODE,
    # Records
    MMT.CREATE_RECORD: MMT.CREATE_RECORD,
    MMT.UPDATE_RECORD: MMT.UPDATE_RECORD,
    MMT.MOVE_RECORD: MMT.UPDATE_RECORD,
    MMT.DELETE_RECORD: MMT.DELETE_RECORD,
    MMT.SOFT_DELETE_RECORD: MMT.DELETE_RECORD,
    MMT.RESTORE_RECORD: MMT.UPDATE_RECORD,
    # X blocks
    MMT.CREATE_XBLOCK: MMT.CREATE_XBLOCK,
    MMT.DELETE_XBLOCK: MMT.DELETE_XBLOCK,
}

_IGNORED_PUBLIC = {
    MMT.UPDATE_GENERATED_MAPPINGS,  # not used in frontend client
}

_SCOPE_TO_TYPE_NAME = {
    MMS.FILE: "File",
    MMS.STATEMENT: "Statement",
    MMS.TYPE_NODE: "TypeNode",
    MMS.RECORD: "Record",
    MMS.XBLOCK: "XBlock",
}


def map_mutation_from_public(
    type: MMT, input: Any, thing: MutableThing
) -> tuple[list[ModuleMutation], list[ModuleMutation]]:
    """
    Remap/create public multiplayer mutation for other clients and internals.
    Returns both the internal and public mutations to publish.
    """
    if isinstance(thing, models.File):
        project_version_id = thing.project_version_id
        file_id = thing.id
        statement_id = None
    elif isinstance(thing, models.Statement):
        project_version_id = thing.project_version_id
        file_id = thing.file_id
        statement_id = thing.id
    elif isinstance(thing, (models.SimpleTypeNode, models.DatasetRecord, models.XBlock)):
        project_version_id = thing.statement.project_version_id
        file_id = thing.statement.file_id
        statement_id = thing.statement_id
    else:
        raise TypeError(f"thing is not a project thing: {thing}")

    public_mutation = ModuleMutation(
        type=type,
        project_version_id=project_version_id,
        file_id=file_id,
        statement_id=statement_id,
        revision=thing.revision,
        input=input,
    )
    if type == MMT.PASTE_FILE:
        public_mutation.type = MMT.CREATE_FILE
        statement_data = mapper.rmap_file_nested(thing, project_version_id)
        internal = ModuleMutator(module_id=project_version_id).create(statement_data)
        public_mutations = list(
            chain.from_iterable(map_mutation_to_public(m) for m in internal.mutations)
        )
        return internal.mutations, public_mutations
    elif type == MMT.PASTE_STATEMENT:  # remap to create children
        public_mutation.type = MMT.CREATE_STATEMENT
        statement_data = mapper.rmap_statement(thing, file_id, project_version_id, flat=False)
        internal = ModuleMutator(module_id=project_version_id).create(statement_data)
        public_mutations = list(
            chain.from_iterable(map_mutation_to_public(m) for m in internal.mutations)
        )
        return internal.mutations, public_mutations
    else:
        # TODO @Broken: remap restore public mutations for previously offline clients
        #  (restore is insufficient if you don't have the original file?/statement/etc.)
        internal = map_mutation_to_internal(public_mutation, thing)
        return internal, [public_mutation]


def map_mutation_to_internal(mutation: ModuleMutation, thing: MutableThing) -> list[ModuleMutation]:
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
            descendants_datas = mapper.rmap_statement_nested(thing)
            mut = ModuleMutator(module_id=mutation.project_version_id)
            return mut.create_many(*descendants_datas).mutations
    elif mutation.type in _TRIVIAL_PUBLIC_TO_INTERNAL:
        internal_type = _TRIVIAL_PUBLIC_TO_INTERNAL.get(mutation.type)
    elif mutation.type == MMT.RESTORE_FILE:
        file_data = mapper.rmap_file_nested(thing, exclude_non_semantic=True)
        mut = ModuleMutator(module_id=mutation.project_version_id)
        return mut.create(file_data).mutations
    elif mutation.type == MMT.RESTORE_STATEMENT:
        descendants_datas = mapper.rmap_statement_nested(thing)
        mut = ModuleMutator(module_id=mutation.project_version_id)
        return mut.create_many(*descendants_datas).mutations
    else:
        raise ValueError(f"mutation cannot be mapped to internal: {mutation}")

    internal_mutation = ModuleMutation(
        type=internal_type,
        project_version_id=mutation.project_version_id,
        file_id=mutation.file_id,
        statement_id=mutation.statement_id,
        revision=thing.revision,
    )
    internal_mutation.data = mapper.rmap_flat(thing)
    return [internal_mutation]


def map_mutation_to_public(mutation: ModuleMutation) -> list[ModuleMutation]:
    """
    Maps a simple internal mutation to a public multiplayer mutation.

    This is conceptually the inverse of map_mutation_to_internal, but is a bit simpler
    since all internal mutations are also valid public mutations (it's a subset).

    The main challenge is reconstructing an "input" that is exactly the input that
    would have caused the same internal mutation. Note that for some mutations, this
    is a theoretical equivalence, since e.g., hard deletes aren't used in the client.
    (but still exposed for the purpose of internal synchronisation with this right here)
    """
    if not mutation.type.simple:
        raise ValueError(f"mutation is not a simple internal mutation: {mutation}")
    if mutation.type in _IGNORED_PUBLIC:
        return []
    input = map_mutation_to_input(mutation)
    public_mutation = ModuleMutation(
        type=mutation.type,
        project_version_id=mutation.project_version_id,
        file_id=mutation.file_id,
        statement_id=mutation.statement_id,
        revision=mutation.revision,
        input=input,
    )
    return [public_mutation]


# extra fields in public mutations that are not in internal module data
_EXTRA_FIELDS_BY_SCOPE = {
    MMS.STATEMENT: {
        "commented": False,
    },
    MMS.FILE: {
        "parent_id": None,
    },
}

_EXTRA_FIELD_RENAMES = {"project_version_id": "module_id"}


def map_mutation_to_input(mutation: ModuleMutation) -> Any:
    """
    Maps a simple internal mutation to an input that would cause the same mutation.
    The returned input is already jsonable (not the original input class).
    """
    from bench.api.sync import INPUT_CLASS_BY_MMT

    if mutation.data is None:
        raise ValueError(f"mutation has no data: {mutation}")

    extra_fields = _EXTRA_FIELDS_BY_SCOPE.get(mutation.type.scope, {})
    input_cls = INPUT_CLASS_BY_MMT[mutation.type]
    input_args = {}
    for field in fields(input_cls):
        key = field.name
        if key in _EXTRA_FIELD_RENAMES:
            key = _EXTRA_FIELD_RENAMES[key]
        if key in extra_fields:
            value = extra_fields[key]
        else:
            value = getattr(mutation.data, key)
        if isinstance(value, UUID):
            value = _map_id_field(key, value, mutation.type.scope)
        input_args[key] = value
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


def _map_id_field(key: str, value: UUID, scope: MMS):
    from strawberry_django_plus.relay import GlobalID

    # map id to global id with appropriate type name
    if key == "file_id":
        type_name = "File"
    elif key == "statement_id":
        type_name = "Statement"
    elif key == "parent_id" and scope == MMS.STATEMENT:
        type_name = "Statement"
    elif key == "parent_id" and scope == MMS.FILE:
        type_name = "File"
    else:
        type_name = _SCOPE_TO_TYPE_NAME[scope]
    value = GlobalID(type_name, str(value))
    return value
