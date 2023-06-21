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
from bench.bench.const import ModuleObjectType
from bench.bench.type import ModuleNode
from bench.bench.wire import (
    BASE_DATA_CLASS_BY_MOT,
    MOT_BY_DATA_CLASS,
    STATEMENT_TYPE_BY_DATA_CLASS,
    CodeData,
    DatasetData,
    DatasetViewData,
    ExpectationData,
    FieldData,
    FileData,
    IssueData,
    ModelData,
    ModuleData,
    ModuleTree,
    ModuleTreeData,
    NodeData,
    RecordData,
    RequirementData,
    ResolvedFieldData,
    StatementData,
    TaskData,
    TypeData,
    ValueData,
)


class ModuleMutationType(enum.StrEnum):
    """Fine-grained atomic mutations for multiplayer modules."""

    # Files
    TRUNCATE_FILES = "TRUNCATE_FILES"
    BUMP_FILE = "BUMP_FILE"
    CREATE_FILE = "CREATE_FILE"
    UPDATE_FILE = "UPDATE_FILE"
    DELETE_FILE = "DELETE_FILE"
    # Files (API)
    PASTE_FILE = "PASTE_FILE"
    SOFT_DELETE_FILE = "SOFT_DELETE_FILE"
    RESTORE_FILE = "RESTORE_FILE"
    RENAME_FILE = "RENAME_FILE"
    MOVE_FILE = "MOVE_FILE"
    # Statements
    TRUNCATE_STATEMENTS = "TRUNCATE_STATEMENTS"
    BUMP_STATEMENT = "BUMP_STATEMENT"
    CREATE_STATEMENT = "CREATE_STATEMENT"
    UPDATE_STATEMENT = "UPDATE_STATEMENT"
    DELETE_STATEMENT = "DELETE_STATEMENT"
    # Statements (API)
    PASTE_STATEMENT = "PASTE_STATEMENT"
    SOFT_DELETE_STATEMENT = "SOFT_DELETE_STATEMENT"
    RESTORE_STATEMENT = "RESTORE_STATEMENT"
    MORPH_STATEMENT = "MORPH_STATEMENT"
    COMMENT_STATEMENT = "COMMENT_STATEMENT"
    MOVE_STATEMENT = "MOVE_STATEMENT"
    RENAME_STATEMENT = "RENAME_STATEMENT"
    UPDATE_STATEMENT_TEXT = "UPDATE_STATEMENT_TEXT"
    # Symbols (API)
    UPDATE_SYMBOL_DESCRIPTION = "UPDATE_SYMBOL_DESCRIPTION"
    UPDATE_SYMBOL_CODE = "UPDATE_SYMBOL_CODE"
    UPDATE_SYMBOL_MODIFIER = "UPDATE_SYMBOL_MODIFIER"
    UPDATE_SYMBOL_LANGUAGE = "UPDATE_SYMBOL_LANGUAGE"
    UPDATE_SYMBOL_VALUE = "UPDATE_SYMBOL_VALUE"
    # Types
    TRUNCATE_FIELDS = "TRUNCATE_FIELDS"
    CREATE_FIELD = "CREATE_FIELD"
    UPDATE_FIELD = "UPDATE_FIELD"
    DELETE_FIELD = "DELETE_FIELD"
    # Types (API)
    RENAME_FIELD = "RENAME_FIELD"
    UPDATE_FIELD_DESCRIPTION = "UPDATE_FIELD_DESCRIPTION"
    UPDATE_FIELD_TYPE = "UPDATE_FIELD_TYPE"
    MOVE_FIELD = "MOVE_FIELD"
    SOFT_DELETE_FIELD = "SOFT_DELETE_FIELD"
    RESTORE_FIELD = "RESTORE_FIELD"
    # Records
    TRUNCATE_RECORDS = "TRUNCATE_RECORDS"
    CREATE_RECORD = "CREATE_RECORD"
    UPDATE_RECORD = "UPDATE_RECORD"
    DELETE_RECORD = "DELETE_RECORD"
    # Records (API)
    MOVE_RECORD = "MOVE_RECORD"
    SOFT_DELETE_RECORD = "SOFT_DELETE_RECORD"
    RESTORE_RECORD = "RESTORE_RECORD"
    # Interp
    TRUNCATE_ISSUES = "TRUNCATE_ISSUES"
    CREATE_ISSUE = "CREATE_ISSUE"
    DELETE_ISSUE = "DELETE_ISSUE"
    TRUNCATE_RESOLVED_FIELDS = "TRUNCATE_RESOLVED_FIELDS"
    CREATE_RESOLVED_FIELD = "CREATE_RESOLVED_FIELD"

    @property
    def is_soft(self) -> bool:
        return self.value.startswith("SOFT_")

    @property
    def kind(self) -> "ModuleMutationKind":
        return _MODULE_MUTATION_MAP[self][0]

    @property
    def mot(self) -> "ModuleObjectType":
        return _MODULE_MUTATION_MAP[self][1]

    @property
    def simple(self) -> bool:
        return self in SIMPLE_MUTATIONS


class ModuleMutationKind(enum.StrEnum):
    CREATE = "CREATE"
    UPDATE = "UPDATE"
    DELETE = "DELETE"
    TRUNCATE = "TRUNCATE"
    BUMP = "BUMP"


# Basic CUD mutations with full (flat) data for the model
SIMPLE_MUTATIONS = {
    # File
    ModuleMutationType.TRUNCATE_FILES,
    ModuleMutationType.BUMP_FILE,
    ModuleMutationType.CREATE_FILE,
    ModuleMutationType.UPDATE_FILE,
    ModuleMutationType.DELETE_FILE,
    # Statement
    ModuleMutationType.TRUNCATE_STATEMENTS,
    ModuleMutationType.BUMP_STATEMENT,
    ModuleMutationType.CREATE_STATEMENT,
    ModuleMutationType.UPDATE_STATEMENT,
    ModuleMutationType.DELETE_STATEMENT,
    # Fields
    ModuleMutationType.TRUNCATE_FIELDS,
    ModuleMutationType.CREATE_FIELD,
    ModuleMutationType.UPDATE_FIELD,
    ModuleMutationType.DELETE_FIELD,
    # Record
    ModuleMutationType.TRUNCATE_RECORDS,
    ModuleMutationType.CREATE_RECORD,
    ModuleMutationType.UPDATE_RECORD,
    ModuleMutationType.DELETE_RECORD,
    # Interp
    ModuleMutationType.TRUNCATE_ISSUES,
    ModuleMutationType.CREATE_ISSUE,
    ModuleMutationType.TRUNCATE_RESOLVED_FIELDS,
    ModuleMutationType.CREATE_RESOLVED_FIELD,
}

MMT = ModuleMutationType
MMK = ModuleMutationKind
MOT = ModuleObjectType

_MODULE_MUTATION_MAP: dict[MMT, tuple[MMK, MOT]] = {
    # Files
    MMT.TRUNCATE_FILES: (MMK.TRUNCATE, MOT.FILE),
    MMT.BUMP_FILE: (MMK.BUMP, MOT.FILE),
    MMT.PASTE_FILE: (MMK.CREATE, MOT.FILE),
    MMT.CREATE_FILE: (MMK.CREATE, MOT.FILE),
    MMT.SOFT_DELETE_FILE: (MMK.DELETE, MOT.FILE),
    MMT.RESTORE_FILE: (MMK.CREATE, MOT.FILE),
    MMT.RENAME_FILE: (MMK.UPDATE, MOT.FILE),
    MMT.MOVE_FILE: (MMK.UPDATE, MOT.FILE),
    MMT.UPDATE_FILE: (MMK.UPDATE, MOT.FILE),
    MMT.DELETE_FILE: (MMK.DELETE, MOT.FILE),
    # Statements
    MMT.TRUNCATE_STATEMENTS: (MMK.TRUNCATE, MOT.STATEMENT),
    MMT.BUMP_STATEMENT: (MMK.BUMP, MOT.STATEMENT),
    MMT.PASTE_STATEMENT: (MMK.CREATE, MOT.STATEMENT),
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
    MMT.TRUNCATE_FIELDS: (MMK.TRUNCATE, MOT.FIELD),
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
    MMT.TRUNCATE_RECORDS: (MMK.TRUNCATE, MOT.RECORD),
    MMT.CREATE_RECORD: (MMK.CREATE, MOT.RECORD),
    MMT.UPDATE_RECORD: (MMK.UPDATE, MOT.RECORD),
    MMT.MOVE_RECORD: (MMK.UPDATE, MOT.RECORD),
    MMT.DELETE_RECORD: (MMK.DELETE, MOT.RECORD),
    MMT.SOFT_DELETE_RECORD: (MMK.DELETE, MOT.RECORD),
    MMT.RESTORE_RECORD: (MMK.CREATE, MOT.RECORD),
    # Interp
    MMT.TRUNCATE_ISSUES: (MMK.TRUNCATE, MOT.ISSUE),
    MMT.CREATE_ISSUE: (MMK.CREATE, MOT.ISSUE),
    MMT.DELETE_ISSUE: (MMK.DELETE, MOT.ISSUE),
    MMT.TRUNCATE_RESOLVED_FIELDS: (MMK.TRUNCATE, MOT.RESOLVED_FIELD),
    MMT.CREATE_RESOLVED_FIELD: (MMK.CREATE, MOT.RESOLVED_FIELD),
}


@dataclass(repr=False, slots=True)
class ModuleMutation:
    type: MMT
    project_version_id: UUID
    file_id: Optional[UUID] = None
    statement_id: Optional[UUID] = None
    revision: Optional[int] = None
    input: Optional[dict[str, Any]] = None  # for GQL mutations
    properties: Optional[list[str]] = None  # for partial updates

    thing: Optional[Any] = None  # in-memory object that was mutated, not serialized

    # data as a proper union doesn't work here since the dataclasses overlap
    # and the deserializer doesn't know which one to use (so will pick the first that fits)
    # really annoyingly manual until we get a proper :WireFormat

    _data__mot: Optional[ModuleObjectType] = None  # discriminator for 'union'
    _data_module: Optional[ModuleData] = None
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
    _data_dataset_view: Optional[DatasetViewData] = None
    _data_issue: Optional[IssueData] = None
    _data_resolved_field: Optional[ResolvedFieldData] = None

    def encode_some_attrs(self):  # see serialize and :WireFormat
        return {"thing": None}  # always omit thing

    @property
    def data(self) -> Optional[NodeData]:
        if self._data_statement__type is not None:
            # map to _symbol_<type>
            return getattr(self, f"_data_statement_{self._data_statement__type.value.lower()}")
        elif self._data__mot is not None:
            return getattr(self, f"_data_{self._data__mot.value.lower()}")
        else:
            return None

    @data.setter
    def data(self, value: NodeData):
        self._data__mot = MOT_BY_DATA_CLASS[type(value)]
        if type(value) in STATEMENT_TYPE_BY_DATA_CLASS:
            # map to _symbol_<type>
            statement_type = STATEMENT_TYPE_BY_DATA_CLASS[type(value)]
            self._data_statement__type = statement_type
            setattr(self, f"_data_statement_{statement_type.value.lower()}", value)
        else:
            setattr(self, f"_data_{self._data__mot.value.lower()}", value)

    @property
    def scope(self) -> ModuleObjectType:
        if self._data__mot is None:
            raise ValueError(f"mot is not set on {self}")
        return self._data__mot

    @property
    def mot(self) -> ModuleObjectType:
        return self.type.mot

    def __str__(self):
        properties_str = (" [" + ", ".join(self.properties) + "]") if self.properties else ""
        return f"{self.type} {self.revision}{properties_str}"

    def __repr__(self):
        return f"<Mutation {self}>"


def pack_node_flat_if_needed(node: ModuleNode | NodeData) -> NodeData:
    if isinstance(node, NodeData):
        return node
    else:
        return wire.pack_node_flat(node)


class ModuleMutator:
    """Helper for mutating module data."""

    def __init__(
        self,
        module: Module | ModuleTreeData | UUID,
        mutations: list[ModuleMutation] = None,
        # default file and statement id
        file_id: UUID = None,
        statement_id: UUID = None,
    ):
        if isinstance(module, Module):
            module = wire.pack_module(module)
        if isinstance(module, ModuleTreeData):
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
        # immediately apply mutations
        for mutation in mutations or []:
            self.apply(mutation)

    def __str__(self):
        return f"mutate {len(self.mutations)} {self.module or '<no module>'}"

    def __repr__(self):
        return f"<Mutator {self}>"

    def do(
        self, type: MMT, obj: NodeData, apply: bool = True, properties: list[str] = None
    ) -> "ModuleMutator":
        if isinstance(obj, StatementData):
            statement_id = obj.id
            file_id = self.file_id or self.tree.get_ancestor(obj.parent_id, wire.FileData).id
        elif isinstance(obj, FileData):
            statement_id = None
            file_id = obj.id
        elif isinstance(obj, ModuleData):
            statement_id = None
            file_id = None
        else:
            if self.statement_id:
                statement_id = self.statement_id
            else:
                statement = self.tree.get_ancestor(obj.parent_id, wire.StatementData)
                statement_id = statement.id if statement else None
            if self.file_id:
                file_id = self.file_id
            else:
                file = self.tree.get_ancestor(obj.parent_id, wire.FileData)
                file_id = self.file_id or file.id
        if properties and type.kind != MMK.UPDATE:
            raise ValueError(f"properties only supported for update mutations: {properties}")
        mutation = ModuleMutation(
            type=type,
            project_version_id=self.module_id,
            revision=obj.revision if isinstance(obj, wire.Revisioned) else None,
            file_id=file_id,
            statement_id=statement_id,
            properties=properties,
        )
        mutation.data = obj
        self.mutations.append(mutation)
        if apply:
            self.apply(mutation)
        return self

    def apply(self, mut: ModuleMutation):
        if mut.type.kind == MMK.CREATE:
            self.tree.add(mut.data)
        elif mut.type.kind == MMK.UPDATE:
            self.tree.replace(mut.data)
        elif mut.type.kind == MMK.DELETE:
            self.tree.remove(mut.data)
        elif mut.type.kind == MMK.TRUNCATE:
            self.tree.truncate(mut.data, BASE_DATA_CLASS_BY_MOT[mut.mot])
        else:
            raise ValueError(f"unexpected mutation kind {mut}")

    def truncate(self, obj: NodeData | ModuleNode, mot: MOT, apply: bool = True) -> "ModuleMutator":
        """Truncates all records of the given statement."""
        obj = pack_node_flat_if_needed(obj)
        mmt = MMT(f"TRUNCATE_{mot.name}S")
        self.do(mmt, obj, apply=apply)
        return self

    def create_many(self, *objs: NodeData | ModuleNode, apply: bool = True) -> "ModuleMutator":
        for obj in objs:
            self.create(obj, apply=apply)
        return self

    def create(self, obj: NodeData | ModuleNode, apply: bool = True) -> "ModuleMutator":
        obj = pack_node_flat_if_needed(obj)
        mot = MOT_BY_DATA_CLASS[type(obj)]
        mmt = MMT(f"CREATE_{mot.name}")
        self.do(mmt, obj, apply=apply)
        return self

    def update_many(self, *objs: NodeData | ModuleNode, apply: bool = True) -> "ModuleMutator":
        for obj in objs:
            self.update(obj, apply=apply)
        return self

    def update(
        self, obj: NodeData | ModuleNode, apply: bool = True, properties: list[str] = None
    ) -> "ModuleMutator":
        obj = pack_node_flat_if_needed(obj)
        mot = MOT_BY_DATA_CLASS[type(obj)]
        mmt = MMT(f"UPDATE_{mot.name}")
        self.do(mmt, obj, apply=apply, properties=properties)
        return self

    def delete_many(self, *objs: NodeData | ModuleNode, apply: bool = True) -> "ModuleMutator":
        for obj in objs:
            self.delete(obj, apply=apply)
        return self

    def delete(self, obj: NodeData | ModuleNode, apply: bool = True) -> "ModuleMutator":
        obj = pack_node_flat_if_needed(obj)
        mot = MOT_BY_DATA_CLASS[type(obj)]
        mmt = MMT(f"DELETE_{mot.name}")
        self.do(mmt, obj, apply=apply)
        return self

    def bundle(self) -> "MutationBundle":
        return MutationBundle(self.mutations)

    def to_module(self) -> ModuleTreeData:
        # already applied in memory
        if self.module is None:
            raise ValueError(f"cannot apply {self} without a module")
        return replace(self.module, nodes=list(self.tree.nodes.values()))


class MutationBundle:
    """Indexed access to an assumed constant list of mutations."""

    def __init__(self, mutations: list[ModuleMutation]):
        self.mutations = mutations

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

    # TODO @Perofmrance: mutation compaction & batching can be much smarter
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

    def batch(self) -> list[tuple[MMT, list[ModuleMutation]]]:
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
