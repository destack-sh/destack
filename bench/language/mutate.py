"""
Structs for syncing project model contents across servers and clients.
It shouldn't live in models, so we can use it in messages.py, which shouldn't depend on Django.
Maybe a better move would be to make the payload partially opaque and keep this in api.
"""
import enum
from dataclasses import dataclass
from typing import Any, Optional
from uuid import UUID

from bench.language import ModuleIndex, wire
from bench.language.type import GeneratedMapping, StatementType
from bench.language.wire import FileData, ModuleData, RecordData, SimpleTypeNodeData, StatementData


class ModuleMutationType(enum.StrEnum):
    """Fine-grained atomic mutations for multiplayer modules."""

    COMMIT = "COMMIT"
    # Files
    CREATE_FILE = "CREATE_FILE"
    SOFT_DELETE_FILE = "DELETE_FILE"
    RESTORE_FILE = "RESTORE_FILE"
    RENAME_FILE = "RENAME_FILE"
    MOVE_FILE = "MOVE_FILE"
    UPDATE_FILE = "UPDATE_FILE"
    DELETE_FILE = "DELETE_FILE"
    # Statements
    CREATE_STATEMENT = "CREATE_STATEMENT"
    CREATE_STATEMENT_BLANK = "CREATE_STATEMENT_BLANK"
    SOFT_DELETE_STATEMENT = "DELETE_STATEMENT"
    RESTORE_STATEMENT = "RESTORE_STATEMENT"
    UPDATE_STATEMENT_MODIFIER = "UPDATE_STATEMENT_MODIFIER"
    UPDATE_STATEMENT_REFERENCE = "UPDATE_STATEMENT_REFERENCE"
    MORPH_STATEMENT = "MORPH_STATEMENT"
    COMMENT_STATEMENT = "COMMENT_STATEMENT"
    MOVE_STATEMENT = "MOVE_STATEMENT"
    RENAME_STATEMENT = "RENAME_STATEMENT"
    UPDATE_STATEMENT_TEXT = "UPDATE_STATEMENT_TEXT"  # for comments
    UPDATE_STATEMENT_DESCRIPTION = "UPDATE_STATEMENT_DESCRIPTION"
    UPDATE_STATEMENT_CODE = "UPDATE_STATEMENT_CODE"
    UPDATE_STATEMENT_LANGUAGE = "UPDATE_STATEMENT_LANGUAGE"
    UPDATE_STATEMENT = "UPDATE_STATEMENT"
    UPDATE_GENERATED_MAPPINGS = "UPDATE_GENERATED_MAPPINGS"
    DELETE_STATEMENT = "DELETE_STATEMENT"
    # Types
    CREATE_TYPE_NODE = "CREATE_TYPE_NODE"
    UPDATE_TYPE_NODE = "UPDATE_TYPE_NODE"
    MOVE_TYPE_NODE = "MOVE_TYPE_NODE"
    DELETE_TYPE_NODE = "DELETE_TYPE_NODE"
    RESTORE_TYPE_NODE = "RESTORE_TYPE_NODE"
    # Records
    CREATE_RECORD = "CREATE_RECORD"
    UPDATE_RECORD = "UPDATE_RECORD"
    MOVE_RECORD = "MOVE_RECORD"
    DELETE_RECORD = "DELETE_RECORD"
    RESTORE_RECORD = "RESTORE_RECORD"

    @property
    def kind(self) -> "ModuleMutationKind":
        return _MODULE_MUTATION_MAP[self][0]

    @property
    def scope(self) -> "ModuleMutationScope":
        return _MODULE_MUTATION_MAP[self][1]


class ModuleMutationKind(enum.StrEnum):
    CREATE = "CREATE"
    UPDATE = "UPDATE"
    DELETE = "DELETE"


class ModuleMutationScope(enum.StrEnum):
    """Scope of a mutation."""

    FILE = "FILE"
    STATEMENT = "STATEMENT"
    TYPE_NODE = "TYPE_NODE"
    RECORD = "RECORD"


MMT = ModuleMutationType
MMK = ModuleMutationKind
MMS = ModuleMutationScope

_MODULE_MUTATION_MAP: dict[MMT, tuple[MMK, MMS]] = {
    # Files
    MMT.CREATE_FILE: (MMK.CREATE, MMS.FILE),
    MMT.SOFT_DELETE_FILE: (MMK.DELETE, MMS.FILE),
    MMT.RESTORE_FILE: (MMK.UPDATE, MMS.FILE),
    MMT.RENAME_FILE: (MMK.UPDATE, MMS.FILE),
    MMT.MOVE_FILE: (MMK.UPDATE, MMS.FILE),
    MMT.UPDATE_FILE: (MMK.UPDATE, MMS.FILE),
    MMT.DELETE_FILE: (MMK.DELETE, MMS.FILE),
    # Statements
    MMT.CREATE_STATEMENT: (MMK.CREATE, MMS.STATEMENT),
    MMT.CREATE_STATEMENT_BLANK: (MMK.CREATE, MMS.STATEMENT),
    MMT.SOFT_DELETE_STATEMENT: (MMK.DELETE, MMS.STATEMENT),
    MMT.RESTORE_STATEMENT: (MMK.UPDATE, MMS.STATEMENT),
    MMT.UPDATE_STATEMENT_MODIFIER: (MMK.UPDATE, MMS.STATEMENT),
    MMT.UPDATE_STATEMENT_REFERENCE: (MMK.UPDATE, MMS.STATEMENT),
    MMT.MORPH_STATEMENT: (MMK.UPDATE, MMS.STATEMENT),
    MMT.COMMENT_STATEMENT: (MMK.UPDATE, MMS.STATEMENT),
    MMT.MOVE_STATEMENT: (MMK.UPDATE, MMS.STATEMENT),
    MMT.RENAME_STATEMENT: (MMK.UPDATE, MMS.STATEMENT),
    MMT.UPDATE_STATEMENT_TEXT: (MMK.UPDATE, MMS.STATEMENT),
    MMT.UPDATE_STATEMENT_DESCRIPTION: (MMK.UPDATE, MMS.STATEMENT),
    MMT.UPDATE_STATEMENT_CODE: (MMK.UPDATE, MMS.STATEMENT),
    MMT.UPDATE_STATEMENT_LANGUAGE: (MMK.UPDATE, MMS.STATEMENT),
    MMT.UPDATE_STATEMENT: (MMK.UPDATE, MMS.STATEMENT),
    MMT.UPDATE_GENERATED_MAPPINGS: (MMK.UPDATE, MMS.STATEMENT),
    MMT.DELETE_STATEMENT: (MMK.DELETE, MMS.STATEMENT),
    # Types
    MMT.CREATE_TYPE_NODE: (MMK.CREATE, MMS.TYPE_NODE),
    MMT.UPDATE_TYPE_NODE: (MMK.UPDATE, MMS.TYPE_NODE),
    MMT.MOVE_TYPE_NODE: (MMK.UPDATE, MMS.TYPE_NODE),
    MMT.DELETE_TYPE_NODE: (MMK.DELETE, MMS.TYPE_NODE),
    MMT.RESTORE_TYPE_NODE: (MMK.UPDATE, MMS.TYPE_NODE),
    # Records
    MMT.CREATE_RECORD: (MMK.CREATE, MMS.RECORD),
    MMT.UPDATE_RECORD: (MMK.UPDATE, MMS.RECORD),
    MMT.MOVE_RECORD: (MMK.UPDATE, MMS.RECORD),
    MMT.DELETE_RECORD: (MMK.DELETE, MMS.RECORD),
    MMT.RESTORE_RECORD: (MMK.UPDATE, MMS.RECORD),
}


@dataclass(repr=False, slots=True)
class ModuleMutation:
    type: MMT
    project_version_id: UUID
    file_id: Optional[UUID] = None
    statement_id: Optional[UUID] = None
    record_id: Optional[UUID] = None
    type_node_id: Optional[UUID] = None
    revision: Optional[int] = None
    input: Optional[dict[str, Any]] = None  # for GQL mutations
    data: Optional[FileData | StatementData | SimpleTypeNodeData | RecordData] = None


class ModuleMutator:
    """Helper for mutating module state."""

    def __init__(self, idx: ModuleIndex, mutations: list[ModuleMutation] = None):
        self.idx = idx
        self.module = idx.module
        self.mutations = mutations or []

    def map(self, generator_id: UUID, mappings: list[GeneratedMapping]) -> "ModuleMutator":
        """Map a list of generated mappings to a statement."""
        statement = wire.rmap_statement(self.idx.statements[generator_id])
        statement.generated_mappings = mappings
        self.do(MMT.UPDATE_GENERATED_MAPPINGS, statement)
        return self

    def do(
        self, type: MMT, obj: FileData | StatementData | SimpleTypeNodeData | RecordData
    ) -> "ModuleMutator":
        mutation = ModuleMutation(
            type=type,
            project_version_id=self.module.id,
            revision=obj.revision,
            file_id=obj.id if isinstance(obj, FileData) else None,
            statement_id=obj.id if isinstance(obj, StatementData) else None,
            record_id=obj.id if isinstance(obj, RecordData) else None,
            type_node_id=obj.id if isinstance(obj, SimpleTypeNodeData) else None,
            data=obj,
        )
        self.mutations.append(mutation)
        return self

    def create_many(
        self, *objs: FileData | StatementData | SimpleTypeNodeData | RecordData
    ) -> "ModuleMutator":
        for obj in objs:
            self.create(obj)
        return self

    def create(
        self, obj: FileData | StatementData | SimpleTypeNodeData | RecordData, flat: bool = False
    ) -> "ModuleMutator":
        if isinstance(obj, FileData):
            self.do(MMT.CREATE_FILE, obj)
            if not flat:
                for statement in obj.statements:
                    self.create(statement)
        elif isinstance(obj, StatementData):
            self.do(MMT.CREATE_STATEMENT, obj)
            if not flat:
                # nocheckin: create type nodes
                for record in obj.records or []:
                    self.create(record)
        elif isinstance(obj, SimpleTypeNodeData):
            self.do(MMT.CREATE_TYPE_NODE, obj)
        elif isinstance(obj, RecordData):
            self.do(MMT.CREATE_RECORD, obj)
        return self

    def update_many(
        self, *objs: FileData | StatementData | SimpleTypeNodeData | RecordData
    ) -> "ModuleMutator":
        for obj in objs:
            self.update(obj)
        return self

    def update(
        self, obj: FileData | StatementData | SimpleTypeNodeData | RecordData
    ) -> "ModuleMutator":
        if isinstance(obj, FileData):
            self.do(MMT.UPDATE_FILE, obj)
        elif isinstance(obj, StatementData):
            self.do(MMT.UPDATE_STATEMENT, obj)
        elif isinstance(obj, SimpleTypeNodeData):
            self.do(MMT.UPDATE_TYPE_NODE, obj)
        elif isinstance(obj, RecordData):
            self.do(MMT.UPDATE_RECORD, obj)
        return self

    def delete_many(
        self, *objs: FileData | StatementData | SimpleTypeNodeData | RecordData
    ) -> "ModuleMutator":
        for obj in objs:
            self.delete(obj)
        return self

    def delete(
        self, obj: FileData | StatementData | SimpleTypeNodeData | RecordData
    ) -> "ModuleMutator":
        if isinstance(obj, FileData):
            self.do(MMT.DELETE_FILE, obj)
        elif isinstance(obj, StatementData):
            self.do(MMT.DELETE_STATEMENT, obj)
        elif isinstance(obj, SimpleTypeNodeData):
            self.do(MMT.DELETE_TYPE_NODE, obj)
        elif isinstance(obj, RecordData):
            self.do(MMT.DELETE_RECORD, obj)
        return self

    def apply(self) -> ModuleData:
        """Apply mutations to the module and return the new module data."""
        module_data = wire.rmap_module(self.module)

        raise NotImplementedError

        return module_data


NON_SEMANTIC_MUTATION_TYPES = {
    MMT.COMMIT,
    MMT.CREATE_FILE,
    MMT.CREATE_STATEMENT_BLANK,
    MMT.UPDATE_STATEMENT_TEXT,  # for comments
    MMT.MOVE_TYPE_NODE,
    MMT.MOVE_RECORD,
}
NON_SEMANTIC_STATEMENT_TYPES = {
    StatementType.COMMENT,
    StatementType.BLANK,
}


def is_semantic_statement(statement_type: StatementType) -> bool:
    return statement_type not in NON_SEMANTIC_STATEMENT_TYPES


def is_semantic_mutation(mutation: ModuleMutation) -> bool:
    # trivial filter for definitely non-semantic mutations
    # we could do more here (like filter blank morphs), but not worth it now
    return mutation.type not in NON_SEMANTIC_MUTATION_TYPES
