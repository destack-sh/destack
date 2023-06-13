"""
Structs for syncing project model contents across servers and clients.
It shouldn't live in models, so we can use it in messages.py, which shouldn't depend on Django.
Maybe a better move would be to make the payload partially opaque and keep this in api.
"""
import enum
from dataclasses import dataclass, replace
from functools import cached_property
from typing import Any, Optional
from uuid import UUID

from bench.bench import Module, StatementType, wire
from bench.bench.wire import (
    MOT_BY_DATA_CLASS,
    STATEMENT_TYPE_BY_DATA_CLASS,
    CodeData,
    DatasetData,
    DatasetViewData,
    ExpectationData,
    FieldData,
    FileData,
    InterpData,
    ModelData,
    ModuleData,
    ModuleObjectType,
    ModuleTree,
    NodeData,
    RecordData,
    RequirementData,
    StatementData,
    TaskData,
    TypeData,
    ValueData,
)


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
    PASTE_STATEMENT = "PASTE_STATEMENT"
    SOFT_DELETE_STATEMENT = "SOFT_DELETE_STATEMENT"
    RESTORE_STATEMENT = "RESTORE_STATEMENT"
    MORPH_STATEMENT = "MORPH_STATEMENT"
    COMMENT_STATEMENT = "COMMENT_STATEMENT"
    MOVE_STATEMENT = "MOVE_STATEMENT"
    RENAME_STATEMENT = "RENAME_STATEMENT"
    UPDATE_STATEMENT_TEXT = "UPDATE_STATEMENT_TEXT"  # for comments
    UPDATE_STATEMENT = "UPDATE_STATEMENT"
    DELETE_STATEMENT = "DELETE_STATEMENT"
    # Symbols
    UPDATE_SYMBOL_DESCRIPTION = "UPDATE_SYMBOL_DESCRIPTION"
    UPDATE_SYMBOL_CODE = "UPDATE_SYMBOL_CODE"
    UPDATE_SYMBOL_MODIFIER = "UPDATE_SYMBOL_MODIFIER"
    UPDATE_SYMBOL_LANGUAGE = "UPDATE_SYMBOL_LANGUAGE"
    UPDATE_SYMBOL_VALUE = "UPDATE_SYMBOL_VALUE"
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
    MOVE_RECORD = "MOVE_RECORD"
    SOFT_DELETE_RECORD = "SOFT_DELETE_RECORD"
    DELETE_RECORD = "DELETE_RECORD"
    RESTORE_RECORD = "RESTORE_RECORD"
    # Interp
    UPDATE_INTERP = "UPDATE_INTERP"

    @property
    def kind(self) -> "ModuleMutationKind":
        return _MODULE_MUTATION_MAP[self][0]

    @property
    def scope(self) -> "ModuleObjectType":
        return _MODULE_MUTATION_MAP[self][1]

    @property
    def simple(self) -> bool:
        return self in SIMPLE_MUTATIONS


class ModuleMutationKind(enum.StrEnum):
    CREATE = "CREATE"
    UPDATE = "UPDATE"
    DELETE = "DELETE"


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
MOT = ModuleObjectType

_MODULE_MUTATION_MAP: dict[MMT, tuple[MMK, MOT]] = {
    # Files
    MMT.CREATE_FILE: (MMK.CREATE, MOT.FILE),
    MMT.SOFT_DELETE_FILE: (MMK.DELETE, MOT.FILE),
    MMT.RESTORE_FILE: (MMK.CREATE, MOT.FILE),
    MMT.RENAME_FILE: (MMK.UPDATE, MOT.FILE),
    MMT.MOVE_FILE: (MMK.UPDATE, MOT.FILE),
    MMT.UPDATE_FILE: (MMK.UPDATE, MOT.FILE),
    MMT.DELETE_FILE: (MMK.DELETE, MOT.FILE),
    # Statements
    MMT.CREATE_STATEMENT: (MMK.CREATE, MOT.STATEMENT),
    MMT.SOFT_DELETE_STATEMENT: (MMK.DELETE, MOT.STATEMENT),
    MMT.RESTORE_STATEMENT: (MMK.CREATE, MOT.STATEMENT),
    MMT.MORPH_STATEMENT: (MMK.UPDATE, MOT.STATEMENT),
    MMT.COMMENT_STATEMENT: (MMK.UPDATE, MOT.STATEMENT),
    MMT.MOVE_STATEMENT: (MMK.UPDATE, MOT.STATEMENT),
    MMT.RENAME_STATEMENT: (MMK.UPDATE, MOT.STATEMENT),
    MMT.UPDATE_STATEMENT: (MMK.UPDATE, MOT.STATEMENT),
    MMT.DELETE_STATEMENT: (MMK.DELETE, MOT.STATEMENT),
    MMT.UPDATE_STATEMENT_TEXT: (MMK.UPDATE, MOT.STATEMENT),
    # Symbols
    MMT.UPDATE_SYMBOL_DESCRIPTION: (MMK.UPDATE, MOT.STATEMENT),
    MMT.UPDATE_SYMBOL_CODE: (MMK.UPDATE, MOT.STATEMENT),
    MMT.UPDATE_SYMBOL_MODIFIER: (MMK.UPDATE, MOT.STATEMENT),
    MMT.UPDATE_SYMBOL_LANGUAGE: (MMK.UPDATE, MOT.STATEMENT),
    MMT.UPDATE_SYMBOL_VALUE: (MMK.UPDATE, MOT.STATEMENT),
    # Types
    MMT.CREATE_FIELD: (MMK.CREATE, MOT.FIELD),
    MMT.UPDATE_FIELD: (MMK.UPDATE, MOT.FIELD),
    MMT.RENAME_FIELD: (MMK.UPDATE, MOT.FIELD),
    MMT.UPDATE_FIELD_DESCRIPTION: (MMK.UPDATE, MOT.FIELD),
    MMT.UPDATE_FIELD_TYPE: (MMK.UPDATE, MOT.FIELD),
    MMT.MOVE_FIELD: (MMK.UPDATE, MOT.FIELD),
    MMT.DELETE_FIELD: (MMK.DELETE, MOT.FIELD),
    MMT.SOFT_DELETE_FIELD: (MMK.DELETE, MOT.FIELD),
    MMT.RESTORE_FIELD: (MMK.CREATE, MOT.FIELD),
    # Records
    MMT.TRUNCATE_RECORDS: (MMK.DELETE, MOT.STATEMENT),
    MMT.CREATE_RECORD: (MMK.CREATE, MOT.RECORD),
    MMT.UPDATE_RECORD: (MMK.UPDATE, MOT.RECORD),
    MMT.MOVE_RECORD: (MMK.UPDATE, MOT.RECORD),
    MMT.DELETE_RECORD: (MMK.DELETE, MOT.RECORD),
    MMT.SOFT_DELETE_RECORD: (MMK.DELETE, MOT.RECORD),
    MMT.RESTORE_RECORD: (MMK.CREATE, MOT.RECORD),
    # Interp
    MMT.UPDATE_INTERP: (MMK.UPDATE, MOT.INTERP),
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
    # really annoyingly manual until we get a proper :WireFormat

    _data_file: Optional[FileData] = None
    _data_statement: Optional[StatementData] = None
    _data_statement__type: Optional[StatementType] = None  # discriminator for 'union'
    _data_statement_type: Optional[TypeData] = None
    _data_statement_task: Optional[TaskData] = None
    _data_statement_expectation: Optional[ExpectationData] = None
    _data_statement_code: Optional[CodeData] = None
    _data_statement_model: Optional[ModelData] = None
    _data_statement_requirement: Optional[RequirementData] = None
    _data_statement_value: Optional[ValueData] = None
    _data_statement_dataset: Optional[DatasetData] = None
    _data_field: Optional[FieldData] = None
    _data_record: Optional[RecordData] = None
    _data_interp: Optional[InterpData] = None
    _data_dataset_view: Optional[DatasetViewData] = None

    @property
    def data(self) -> NodeData:
        if self._data_statement__type is not None:
            # map to _symbol_<type>
            return getattr(self, f"_data_statement_{self._data_statement__type.value.lower()}")
        else:
            return getattr(self, f"_data_{self.type.scope.value.lower()}")

    @data.setter
    def data(self, value: NodeData):
        if type(value) in STATEMENT_TYPE_BY_DATA_CLASS:
            # map to _symbol_<type>
            statement_type = STATEMENT_TYPE_BY_DATA_CLASS[type(value)]
            self._data_statement__type = statement_type
            setattr(self, f"_data_statement_{statement_type.value.lower()}", value)
        else:
            setattr(self, f"_data_{self.type.scope.value.lower()}", value)

    def __str__(self):
        return f"{self.type} {self.revision} {self.data}"

    def __repr__(self):
        return f"<Mutation {self}>"


class ModuleMutator:
    """Helper for mutating module data."""

    def __init__(
        self,
        module: Module | ModuleData | UUID,
        mutations: list[ModuleMutation] = None,
        extra_nodes: list[NodeData] = None,
        file_id: UUID = None,
        statement_id: UUID = None,
    ):
        if isinstance(module, Module):
            module = wire.pack_module(module)
        if isinstance(module, ModuleData):
            self.module = module
            self.module_id = module.id
            self.tree = ModuleTree(module.nodes)
        else:
            self.module = None
            self.module_id = module
            self.tree = ModuleTree()
        # default file and statement id
        self.file_id = file_id
        self.statement_id = statement_id
        if self.statement_id and not self.file_id:
            raise ValueError("statement_id requires file_id")
        self.mutations = []
        # add extra nodes
        for node in extra_nodes or []:
            self.tree.add(node)
        # immediately apply mutations
        for mutation in mutations or []:
            self._apply(mutation)

    def __str__(self):
        return f"mutate {len(self.mutations)} {self.module or '<no module>'}"

    def __repr__(self):
        return f"<Mutator {self}>"

    def reset(self):
        raise NotImplementedError

    def do(self, type: MMT, obj: NodeData) -> "ModuleMutator":
        if type == MMT.CREATE_STATEMENT:
            statement_id = obj.id
            file_id = self.file_id or self.tree.get_ancestor(obj.parent_id, wire.FileData)
        elif type == MMT.CREATE_FILE:
            statement_id = None
            file_id = obj.id
        else:
            statement_id = (
                self.statement_id or self.tree.get_ancestor(obj.parent_id, wire.StatementData).id
            )
            file_id = self.file_id or self.tree.get_ancestor(statement_id, wire.FileData).id
        mutation = ModuleMutation(
            type=type,
            project_version_id=self.module_id,
            revision=obj.revision if isinstance(obj, wire.Revisioned) else None,
            file_id=file_id,
            statement_id=statement_id,
        )
        mutation.data = obj
        self.mutations.append(mutation)
        self._apply(mutation)
        return self

    def _apply(self, mut: ModuleMutation):
        if mut.type.kind == MMK.CREATE:
            self.tree.add(mut.data)
        elif mut.type.kind == MMK.UPDATE:
            self.tree.replace(mut.data)
        elif mut.type.kind == MMK.DELETE:
            self.tree.remove(mut.data)
        else:
            raise ValueError(f"unexpected mutation kind {mut}")

    def truncate_records(self, statement_id: UUID) -> "ModuleMutator":
        """Truncates all records of the given statement."""
        symbol = self.module.symbols_by_id[statement_id]
        self.do(MMT.TRUNCATE_RECORDS, wire.pack_node_flat(symbol.source))
        return self

    def create_many(self, *objs: NodeData) -> "ModuleMutator":
        for obj in objs:
            self.create(obj)
        return self

    def create(self, obj: NodeData) -> "ModuleMutator":
        mot = MOT_BY_DATA_CLASS[type(obj)]
        mmt = MMT(f"CREATE_{mot.name}")
        self.do(mmt, obj)
        return self

    def update_many(self, *objs: NodeData) -> "ModuleMutator":
        for obj in objs:
            self.update(obj)
        return self

    def update(self, obj: NodeData) -> "ModuleMutator":
        mot = MOT_BY_DATA_CLASS[type(obj)]
        mmt = MMT(f"UPDATE_{mot.name}")
        self.do(mmt, obj)
        return self

    def delete_many(self, *objs: NodeData) -> "ModuleMutator":
        for obj in objs:
            self.delete(obj)
        return self

    def delete(self, obj: NodeData) -> "ModuleMutator":
        mot = MOT_BY_DATA_CLASS[type(obj)]
        mmt = MMT(f"DELETE_{mot.name}")
        self.do(mmt, obj)
        return self

    def bundle(self) -> "MutationBundle":
        return MutationBundle(self.mutations)

    def apply(self) -> ModuleData:
        # already applied in memory
        if self.module is None:
            raise ValueError(f"cannot apply {self} without a module")
        return replace(self.module, nodes=list(self.tree.nodes.values()))


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
        # interp changes are 'simple' because we don't apply them here
        return not any(
            m.type not in SIMPLE_MUTATIONS and m.type.scope != MOT.INTERP for m in self.mutations
        )

    @property
    def complex_mutations(self):
        return [m for m in self.mutations if m.type not in SIMPLE_MUTATIONS]

    def __getitem__(self, type: MMT | MMK | MOT) -> list[ModuleMutation]:
        if type in self._cache:
            return self._cache[type]
        if isinstance(type, MMT):
            mutations = [m for m in self.mutations if m.type == type]
        elif isinstance(type, MMK):
            mutations = [m for m in self.mutations if m.type.kind == type]
        elif isinstance(type, MOT):
            mutations = [m for m in self.mutations if m.type.scope == type]
        else:
            raise TypeError(f"Invalid mutation type: {type}")
        self._cache[type] = mutations
        return mutations

    def compact(self) -> list[ModuleMutation]:
        """
        Compact simple mutations into fewer semantically identical mutations.

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

    def batched(self) -> list[tuple[MMT, list[ModuleMutation]]]:
        """
        Batch consecutive mutations by type in order of appearance.
        (there may be multiple batches of the same type).
        """

        batches: list[tuple[MMT, list[ModuleMutation]]] = []
        current_batch: list[ModuleMutation] = []
        current_type: MMT | None = None

        for mutation in self.mutations:
            if mutation.type != current_type:
                if current_type is not None:
                    batches.append((current_type, current_batch))
                current_type = mutation.type
                current_batch = []
            current_batch.append(mutation)

        if current_batch:
            batches.append((current_type, current_batch))

        return batches
