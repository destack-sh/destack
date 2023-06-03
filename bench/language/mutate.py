"""
Structs for syncing project model contents across servers and clients.
It shouldn't live in models, so we can use it in messages.py, which shouldn't depend on Django.
Maybe a better move would be to make the payload partially opaque and keep this in api.
"""
import enum
from dataclasses import dataclass, replace
from functools import cached_property
from itertools import chain
from typing import Any, Optional
from uuid import UUID

from bench.language import ModuleIndex, wire
from bench.language.type import StatementType
from bench.language.wire import FieldData, FileData, ModuleData, RecordData, StatementData


class ModuleMutationType(enum.StrEnum):
    """Fine-grained atomic mutations for multiplayer modules."""

    # Files
    CREATE_FILE = "CREATE_FILE"
    PASTE_FILE = "PASTE_FILE"
    SOFT_DELETE_FILE = "SOFT_DELETE_FILE"
    RESTORE_FILE = "RESTORE_FILE"
    RENAME_FILE = "RENAME_FILE"
    MOVE_FILE = "MOVE_FILE"
    UPDATE_FILE = "UPDATE_FILE"
    DELETE_FILE = "DELETE_FILE"
    # Statements
    CREATE_STATEMENT = "CREATE_STATEMENT"
    CREATE_STATEMENT_BLANK = "CREATE_STATEMENT_BLANK"
    PASTE_STATEMENT = "PASTE_STATEMENT"
    SOFT_DELETE_STATEMENT = "SOFT_DELETE_STATEMENT"
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
    DELETE_STATEMENT = "DELETE_STATEMENT"
    # Types
    CREATE_FIELD = "CREATE_FIELD"
    UPDATE_FIELD = "UPDATE_FIELD"
    RENAME_FIELD = "RENAME_FIELD"
    UPDATE_FIELD_DESCRIPTION = "UPDATE_FIELD_DESCRIPTION"
    UPDATE_FIELD_TYPE = "UPDATE_FIELD_TYPE"
    MOVE_FIELD = "MOVE_FIELD"
    SOFT_DELETE_FIELD = "SOFT_DELETE_FIELD"
    DELETE_FIELD = "DELETE_FIELD"
    RESTORE_FIELD = "RESTORE_FIELD"
    # Records
    TRUNCATE_RECORDS = "TRUNCATE_RECORDS"
    CREATE_RECORD = "CREATE_RECORD"
    UPDATE_RECORD = "UPDATE_RECORD"
    UPDATE_RECORD_PATH = "UPDATE_RECORD_PATH"
    MOVE_RECORD = "MOVE_RECORD"
    SOFT_DELETE_RECORD = "SOFT_DELETE_RECORD"
    DELETE_RECORD = "DELETE_RECORD"
    RESTORE_RECORD = "RESTORE_RECORD"

    @property
    def kind(self) -> "ModuleMutationKind":
        return _MODULE_MUTATION_MAP[self][0]

    @property
    def scope(self) -> "ModuleMutationScope":
        return _MODULE_MUTATION_MAP[self][1]

    @property
    def simple(self) -> bool:
        return self in SIMPLE_MUTATIONS


class ModuleMutationKind(enum.StrEnum):
    CREATE = "CREATE"
    UPDATE = "UPDATE"
    DELETE = "DELETE"


class ModuleMutationScope(enum.StrEnum):
    """Scope of a mutation."""

    FILE = "FILE"
    STATEMENT = "STATEMENT"
    FIELD = "FIELD"
    RECORD = "RECORD"


# Basic CRUD mutations with full (flat) data for the model
SIMPLE_MUTATIONS = {
    ModuleMutationType.CREATE_FILE,
    ModuleMutationType.UPDATE_FILE,
    ModuleMutationType.DELETE_FILE,
    ModuleMutationType.CREATE_STATEMENT,
    ModuleMutationType.UPDATE_STATEMENT,
    ModuleMutationType.DELETE_STATEMENT,
    ModuleMutationType.CREATE_FIELD,
    ModuleMutationType.UPDATE_FIELD,
    ModuleMutationType.DELETE_FIELD,
    ModuleMutationType.CREATE_RECORD,
    ModuleMutationType.UPDATE_RECORD,
    ModuleMutationType.DELETE_RECORD,
    # other not directly CRUD
    ModuleMutationType.TRUNCATE_RECORDS,
}

MMT = ModuleMutationType
MMK = ModuleMutationKind
MMS = ModuleMutationScope

_MODULE_MUTATION_MAP: dict[MMT, tuple[MMK, MMS]] = {
    # Files
    MMT.CREATE_FILE: (MMK.CREATE, MMS.FILE),
    MMT.SOFT_DELETE_FILE: (MMK.DELETE, MMS.FILE),
    MMT.RESTORE_FILE: (MMK.CREATE, MMS.FILE),
    MMT.RENAME_FILE: (MMK.UPDATE, MMS.FILE),
    MMT.MOVE_FILE: (MMK.UPDATE, MMS.FILE),
    MMT.UPDATE_FILE: (MMK.UPDATE, MMS.FILE),
    MMT.DELETE_FILE: (MMK.DELETE, MMS.FILE),
    # Statements
    MMT.CREATE_STATEMENT: (MMK.CREATE, MMS.STATEMENT),
    MMT.CREATE_STATEMENT_BLANK: (MMK.CREATE, MMS.STATEMENT),
    MMT.SOFT_DELETE_STATEMENT: (MMK.DELETE, MMS.STATEMENT),
    MMT.RESTORE_STATEMENT: (MMK.CREATE, MMS.STATEMENT),
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
    MMT.DELETE_STATEMENT: (MMK.DELETE, MMS.STATEMENT),
    # Types
    MMT.CREATE_FIELD: (MMK.CREATE, MMS.FIELD),
    MMT.UPDATE_FIELD: (MMK.UPDATE, MMS.FIELD),
    MMT.RENAME_FIELD: (MMK.UPDATE, MMS.FIELD),
    MMT.UPDATE_FIELD_DESCRIPTION: (MMK.UPDATE, MMS.FIELD),
    MMT.UPDATE_FIELD_TYPE: (MMK.UPDATE, MMS.FIELD),
    MMT.MOVE_FIELD: (MMK.UPDATE, MMS.FIELD),
    MMT.DELETE_FIELD: (MMK.DELETE, MMS.FIELD),
    MMT.SOFT_DELETE_FIELD: (MMK.DELETE, MMS.FIELD),
    MMT.RESTORE_FIELD: (MMK.CREATE, MMS.FIELD),
    # Records
    MMT.TRUNCATE_RECORDS: (MMK.DELETE, MMS.STATEMENT),
    MMT.CREATE_RECORD: (MMK.CREATE, MMS.RECORD),
    MMT.UPDATE_RECORD: (MMK.UPDATE, MMS.RECORD),
    MMT.UPDATE_RECORD_PATH: (MMK.UPDATE, MMS.RECORD),
    MMT.MOVE_RECORD: (MMK.UPDATE, MMS.RECORD),
    MMT.DELETE_RECORD: (MMK.DELETE, MMS.RECORD),
    MMT.SOFT_DELETE_RECORD: (MMK.DELETE, MMS.RECORD),
    MMT.RESTORE_RECORD: (MMK.CREATE, MMS.RECORD),
}

MutableData = FileData | StatementData | FieldData | RecordData
SCOPE_BY_CLASS = {
    FileData: MMS.FILE,
    StatementData: MMS.STATEMENT,
    FieldData: MMS.FIELD,
    RecordData: MMS.RECORD,
}


@dataclass(repr=False, slots=True)
class ModuleMutation:
    type: MMT
    project_version_id: UUID
    file_id: UUID
    statement_id: Optional[UUID] = None
    revision: Optional[int] = None
    input: Optional[dict[str, Any]] = None  # for GQL mutations

    # data as a proper union doesn't work here since the dataclasses overlap
    # and the deserializer doesn't know which one to use (so will pick the first that fits)

    _data_file: Optional[FileData] = None
    _data_statement: Optional[StatementData] = None
    _data_field: Optional[FieldData] = None
    _data_record: Optional[RecordData] = None

    @property
    def data(self) -> MutableData:
        if self.type.scope == MMS.FILE:
            return self._data_file
        elif self.type.scope == MMS.STATEMENT:
            return self._data_statement
        elif self.type.scope == MMS.FIELD:
            return self._data_field
        elif self.type.scope == MMS.RECORD:
            return self._data_record
        else:
            raise ValueError(f"unexpected mutation scope: {self.type} {self.type.scope}")

    @data.setter
    def data(self, value: MutableData):
        if self.type.scope != SCOPE_BY_CLASS[type(value)]:
            raise ValueError(f"type mismatch: {self.type} {self.type.scope}: {value}")
        if self.type.scope == MMS.FILE:
            self._data_file = value
        elif self.type.scope == MMS.STATEMENT:
            self._data_statement = value
        elif self.type.scope == MMS.FIELD:
            self._data_field = value
        elif self.type.scope == MMS.RECORD:
            self._data_record = value
        else:
            raise ValueError(f"unexpected mutation scope: {self.type} {self.type.scope}")

    def __str__(self):
        return f"{self.type} {self.revision} {self.data}"

    def __repr__(self):
        return f"<Mutation {self}>"


class ModuleMutator:
    """Helper for mutating module data."""

    def __init__(
        self,
        idx: Optional[ModuleIndex] = None,
        mutations: list[ModuleMutation] = None,
        module_id: Optional[UUID] = None,
        source: Optional[wire.ModuleData] = None,
    ):
        self.idx = idx
        self.module = idx.module if idx else None
        self.module_source = source
        self.module_id = module_id or (idx.module.id if idx else None)
        if self.module_id is None:
            raise ValueError("module_id is required")
        self.mutations = mutations or []
        self._created_statements: dict[UUID, StatementData] = {}

    def __str__(self):
        return f"mutate {len(self.mutations)} {self.module or '<no module>'}"

    def __repr__(self):
        return f"<Mutator {self}>"

    def reset(self):
        self.mutations = []
        self._created_statements = {}

    def do(
        self,
        type: MMT,
        obj: MutableData,
    ) -> "ModuleMutator":
        if isinstance(obj, FileData):
            file_id = obj.id
            statement_id = None
        elif isinstance(obj, StatementData):
            statement_id = obj.id
            file_id = obj.file_id
        elif isinstance(obj, (FieldData, RecordData)):
            if obj.statement_id in self._created_statements:
                statement = self._created_statements[obj.statement_id]
                statement_id = statement.id
                file_id = statement.file_id
            else:
                statement = self.idx.statements.get(obj.statement_id)
                statement_id = statement.id
                file_id = statement.file.id
        else:
            raise ValueError(f"unexpected mutation object: {obj}")

        mutation = ModuleMutation(
            type=type,
            project_version_id=self.module_id,
            revision=obj.revision,
            file_id=file_id,
            statement_id=statement_id,
        )
        mutation.data = obj
        self.mutations.append(mutation)
        if type.kind == MMK.CREATE and type.scope == MMS.STATEMENT:
            self._created_statements[statement_id] = obj
        return self

    def truncate_records(self, statement_id: UUID) -> "ModuleMutator":
        """Truncates all records of the given statement."""
        statement = self.idx.statements[statement_id]
        self.do(MMT.TRUNCATE_RECORDS, wire.rmap_statement(statement))
        return self

    def create_many(self, *objs: MutableData) -> "ModuleMutator":
        for obj in objs:
            self.create(obj)
        return self

    def create(self, obj: MutableData, flat: bool = False) -> "ModuleMutator":
        if isinstance(obj, FileData):
            self.do(MMT.CREATE_FILE, obj)
            if not flat:
                for statement in obj.statements:
                    self.create(statement)
        elif isinstance(obj, StatementData):
            self.do(MMT.CREATE_STATEMENT, obj)
            if not flat:
                for type_node in obj.fields or []:
                    self.create(type_node)
                for record in obj.records or []:
                    self.create(record)
        elif isinstance(obj, FieldData):
            self.do(MMT.CREATE_FIELD, obj)
        elif isinstance(obj, RecordData):
            self.do(MMT.CREATE_RECORD, obj)
        else:
            raise ValueError(f"unexpected mutation object: {obj}")
        return self

    def update_many(self, *objs: MutableData) -> "ModuleMutator":
        for obj in objs:
            self.update(obj)
        return self

    def update(self, obj: MutableData) -> "ModuleMutator":
        if isinstance(obj, FileData):
            self.do(MMT.UPDATE_FILE, obj)
        elif isinstance(obj, StatementData):
            self.do(MMT.UPDATE_STATEMENT, obj)
        elif isinstance(obj, FieldData):
            self.do(MMT.UPDATE_FIELD, obj)
        elif isinstance(obj, RecordData):
            self.do(MMT.UPDATE_RECORD, obj)
        else:
            raise ValueError(f"unexpected mutation object: {obj}")
        return self

    def delete_many(self, *objs: MutableData) -> "ModuleMutator":
        for obj in objs:
            self.delete(obj)
        return self

    def delete(self, obj: MutableData) -> "ModuleMutator":
        if isinstance(obj, FileData):
            self.do(MMT.DELETE_FILE, obj)
        elif isinstance(obj, StatementData):
            self.do(MMT.DELETE_STATEMENT, obj)
        elif isinstance(obj, FieldData):
            self.do(MMT.DELETE_FIELD, obj)
        elif isinstance(obj, RecordData):
            self.do(MMT.DELETE_RECORD, obj)
        else:
            raise ValueError(f"unexpected mutation object: {obj}")
        return self

    def bundle(self) -> "MutationBundle":
        return MutationBundle(self.mutations)

    def apply(self) -> ModuleData:
        """Apply (simple!)  mutations to a copy of the module and return the mutated data."""
        if self.module is None:
            raise ValueError("cannot apply mutations without a module")
        mut = MutationBundle(self.mutations)
        if not mut.simple:
            raise ValueError(f"cannot apply complex mutations in {self}: {mut.complex_mutations}")

        if self.module_source:
            module = self.module_source.deepcopy()
        else:
            module = wire.rmap_module(self.module)
        files: dict[UUID, FileData] = {f.id: f for f in module.files}
        statements: dict[UUID, StatementData] = {
            s.id: s for s in chain.from_iterable(f.statements for f in module.files)
        }

        # apply deletes
        deleted_type_nodes = {m.data.id for m in mut[MMT.DELETE_FIELD]}
        deleted_records = {m.data.id for m in mut[MMT.DELETE_RECORD]}
        for m in chain(mut[MMT.DELETE_FIELD], mut[MMT.DELETE_RECORD]):
            statement = statements[m.statement_id]
            if statement.fields:
                statement.fields = [t for t in statement.fields if t.id not in deleted_type_nodes]
            if statement.records:
                statement.records = [r for r in statement.records if r.id not in deleted_records]
        for m in mut[MMT.TRUNCATE_RECORDS]:
            statement = statements[m.statement_id]
            statement.records = []
        for m in mut[MMT.DELETE_STATEMENT]:
            if m.statement_id in statements:  # statement may be non-semantic
                del statements[m.statement_id]
        for m in mut[MMT.DELETE_FILE]:
            if m.file_id in files:  # file may not exist locally?
                for statement in files[m.file_id].statements:
                    del statements[statement.id]
                del files[m.file_id]

        # delete orphaned statements (who no longer have a parent, emulates delete cascade)
        for statement in list(statements.values()):
            if statement.parent_id is not None and statement.parent_id not in statements:
                del statements[statement.id]

        # apply creates
        for m in mut[MMT.CREATE_FILE]:
            files[m.data.id] = m.data
        for m in mut[MMT.CREATE_STATEMENT]:
            statements[m.data.id] = m.data
        for m in mut[MMT.CREATE_FIELD]:
            if statements[m.statement_id].fields is None:
                statements[m.statement_id].fields = []
            statements[m.statement_id].fields.append(m.data)
        for m in mut[MMT.CREATE_RECORD]:
            if statements[m.statement_id].records is None:
                statements[m.statement_id].records = []
            statements[m.statement_id].records.append(m.data)

        # apply updates
        for m in mut[MMT.UPDATE_FILE]:
            files[m.file_id] = m.data
        for m in mut[MMT.UPDATE_STATEMENT]:
            old_statement = statements.get(m.statement_id)
            statements[m.statement_id] = m.data
            # keep statement's relations
            if old_statement is not None:  # otherwise panic?
                statements[m.statement_id].fields = old_statement.fields
                statements[m.statement_id].records = old_statement.records
        for m in mut[MMT.UPDATE_FIELD]:
            statement = statements[m.statement_id]
            _replace_by_id(statement.fields, m.data)
        for m in mut[MMT.UPDATE_RECORD]:
            statement = statements[m.statement_id]
            _replace_by_id(statement.records, m.data)

        # re-assemble module data
        new_module = replace(module, files=list(files.values()))
        for file in files.values():
            file.statements = []
        for statement in statements.values():
            files[statement.file_id].statements.append(statement)
        return new_module


class MutationBundle:
    """Indexed access to an assumed constant list of mutations."""

    def __init__(self, mutations: list[ModuleMutation]):
        self.mutations = mutations
        self._cache: dict[Any, list[ModuleMutation]] = {}

    def __str__(self):
        return f"mut {len(self.mutations)}"

    def __repr__(self):
        return f"<MutationBundle {self}>"

    @cached_property
    def simple(self) -> bool:
        return not any(m.type not in SIMPLE_MUTATIONS for m in self.mutations)

    @property
    def complex_mutations(self):
        return [m for m in self.mutations if m.type not in SIMPLE_MUTATIONS]

    def __getitem__(self, type: MMT | MMK | MMS) -> list[ModuleMutation]:
        if type in self._cache:
            return self._cache[type]
        if isinstance(type, MMT):
            mutations = [m for m in self.mutations if m.type == type]
        elif isinstance(type, MMK):
            mutations = [m for m in self.mutations if m.type.kind == type]
        elif isinstance(type, MMS):
            mutations = [m for m in self.mutations if m.type.scope == type]
        else:
            raise TypeError(f"Invalid mutation type: {type}")
        self._cache[type] = mutations
        return mutations

    def collapse(self) -> list[ModuleMutation]:
        """
        Collapse simple mutations into fewer semantically identical mutations.

        Reduces:
         1. Successive updates to same object to the last update
         2. Successive deletes of same object to the last delete
         Not implemented yet:
         3. Delete after create to nothing
         4. Create then updated merged into a single create
        """
        if not self.simple:
            raise ValueError(f"cannot collapse complex mutations: {self}")

        reduced_inverse = []
        seen_ops: set[tuple[MMT, UUID]] = set()

        for mutation in reversed(self.mutations):
            key = (mutation.type, mutation.data.id)
            if key in seen_ops:
                continue
            seen_ops.add(key)
            reduced_inverse.append(mutation)

        reduced = list(reversed(reduced_inverse))
        return reduced


NON_SEMANTIC_MUTATION_TYPES = {
    MMT.CREATE_FILE,
    MMT.CREATE_STATEMENT_BLANK,
    MMT.UPDATE_STATEMENT_TEXT,  # for comments
    MMT.MOVE_FIELD,
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


def _replace_by_id(things, new_thing) -> None:
    for i, t in enumerate(things):
        if t.id == new_thing.id:
            things[i] = new_thing
            break
