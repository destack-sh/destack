"""
Structs for syncing project model contents across servers and clients.
It shouldn't live in models, so we can use it in messages.py, which shouldn't depend on Django.
Maybe a better move would be to make the payload partially opaque and keep this in api.
"""
import enum
from dataclasses import dataclass
from functools import cached_property
from typing import Any, Callable, Iterator, Optional, Union
from uuid import UUID

from bench.language.const import ModuleNodeType
from bench.language.module import ModuleNode, NodeTree
from bench.language.wire import ModuleTreeData, NodeData
from bench.utils.serialize import from_dict


class ModuleMutationType(enum.StrEnum):
    """Fine-grained atomic mutations for multiplayer modules."""

    # Files
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
    BUMP_STATEMENT = "BUMP_STATEMENT"
    CREATE_STATEMENT = "CREATE_STATEMENT"
    UPDATE_STATEMENT = "UPDATE_STATEMENT"
    DELETE_STATEMENT = "DELETE_STATEMENT"
    # Statements (API)
    PASTE_STATEMENT = "PASTE_STATEMENT"
    SOFT_DELETE_STATEMENT = "SOFT_DELETE_STATEMENT"
    RESTORE_STATEMENT = "RESTORE_STATEMENT"
    MORPH_STATEMENT = "MORPH_STATEMENT"
    MOVE_STATEMENT = "MOVE_STATEMENT"
    RENAME_STATEMENT = "RENAME_STATEMENT"
    UPDATE_STATEMENT_FLAGS = "UPDATE_STATEMENT_FLAGS"
    UPDATE_STATEMENT_TEXT = "UPDATE_STATEMENT_TEXT"
    UPDATE_STATEMENT_HEADING_LEVEL = "UPDATE_STATEMENT_HEADING_LEVEL"
    UPDATE_STATEMENT_REFERENCE = "UPDATE_STATEMENT_REFERENCE"
    UPDATE_SYMBOL_text = "UPDATE_SYMBOL_text"
    UPDATE_SYMBOL_CODE = "UPDATE_SYMBOL_CODE"
    UPDATE_SYMBOL_MODIFIER = "UPDATE_SYMBOL_MODIFIER"
    UPDATE_SYMBOL_LANGUAGE = "UPDATE_SYMBOL_LANGUAGE"
    UPDATE_SYMBOL_VALUE = "UPDATE_SYMBOL_VALUE"
    # Tags
    CREATE_TAGGING = "CREATE_TAGGING"
    UPDATE_TAGGING = "UPDATE_TAGGING"
    DELETE_TAGGING = "DELETE_TAGGING"
    # Tags (API)
    SOFT_DELETE_TAGGING = "SOFT_DELETE_TAGGING"
    RESTORE_TAGGING = "RESTORE_TAGGING"
    MOVE_TAGGING = "MOVE_TAGGING"
    UPDATE_TAGGING_METADATA = "UPDATE_TAGGING_METADATA"
    # Triggers
    CREATE_TRIGGER = "CREATE_TRIGGER"
    UPDATE_TRIGGER = "UPDATE_TRIGGER"
    DELETE_TRIGGER = "DELETE_TRIGGER"
    # Triggers (API)
    SOFT_DELETE_TRIGGER = "SOFT_DELETE_TRIGGER"
    RESTORE_TRIGGER = "RESTORE_TRIGGER"
    # Fields
    CREATE_FIELD = "CREATE_FIELD"
    UPDATE_FIELD = "UPDATE_FIELD"
    DELETE_FIELD = "DELETE_FIELD"
    # Fields (API)
    RENAME_FIELD = "RENAME_FIELD"
    UPDATE_FIELD_TEXT = "UPDATE_FIELD_TEXT"
    UPDATE_FIELD_TYPE = "UPDATE_FIELD_TYPE"
    UPDATE_FIELD_METADATA = "UPDATE_FIELD_METADATA"
    MOVE_FIELD = "MOVE_FIELD"
    SOFT_DELETE_FIELD = "SOFT_DELETE_FIELD"
    RESTORE_FIELD = "RESTORE_FIELD"
    # Records
    TRUNCATE_RECORDS = "TRUNCATE_RECORDS"
    CREATE_RECORD = "CREATE_RECORD"
    UPDATE_RECORD = "UPDATE_RECORD"
    DELETE_RECORD = "DELETE_RECORD"
    # Records (API)
    SOFT_DELETE_RECORD = "SOFT_DELETE_RECORD"
    RESTORE_RECORD = "RESTORE_RECORD"
    # Interp
    TRUNCATE_ISSUES = "TRUNCATE_ISSUES"
    CREATE_ISSUE = "CREATE_ISSUE"
    DELETE_ISSUE = "DELETE_ISSUE"
    TRUNCATE_RESOLVED_FIELDS = "TRUNCATE_RESOLVED_FIELDS"
    CREATE_RESOLVED_FIELD = "CREATE_RESOLVED_FIELD"
    DELETE_RESOLVED_FIELD = "DELETE_RESOLVED_FIELD"

    @property
    def is_soft_delete(self) -> bool:
        return self.value.startswith("SOFT_DELETE_")

    @property
    def kind(self) -> "ModuleMutationKind":
        return _MODULE_MUTATION_MAP[self][0]

    @property
    def mnt(self) -> "ModuleNodeType":
        return _MODULE_MUTATION_MAP[self][1]

    @property
    def simple(self) -> bool:
        return self in SIMPLE_MUTATIONS

    @staticmethod
    def from_nt(mmk: "ModuleMutationKind", nt: "ModuleNodeType") -> "ModuleMutationType":
        return ModuleMutationType(f"{mmk.value}_{nt.value}")


class ModuleMutationKind(enum.StrEnum):
    CREATE = "CREATE"
    UPDATE = "UPDATE"
    DELETE = "DELETE"
    TRUNCATE = "TRUNCATE"
    BUMP = "BUMP"


# Basic CUD mutations with full (flat) data for the model
SIMPLE_MUTATIONS = {
    # File
    ModuleMutationType.BUMP_FILE,
    ModuleMutationType.CREATE_FILE,
    ModuleMutationType.UPDATE_FILE,
    ModuleMutationType.DELETE_FILE,
    # Statement
    ModuleMutationType.BUMP_STATEMENT,
    ModuleMutationType.CREATE_STATEMENT,
    ModuleMutationType.UPDATE_STATEMENT,
    ModuleMutationType.DELETE_STATEMENT,
    # Taggings
    ModuleMutationType.CREATE_TAGGING,
    ModuleMutationType.UPDATE_TAGGING,
    ModuleMutationType.DELETE_TAGGING,
    # Triggers
    ModuleMutationType.CREATE_TRIGGER,
    ModuleMutationType.UPDATE_TRIGGER,
    ModuleMutationType.DELETE_TRIGGER,
    # Fields
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
MNT = ModuleNodeType

_MODULE_MUTATION_MAP: dict[MMT, tuple[MMK, MNT]] = {
    # Files
    MMT.BUMP_FILE: (MMK.BUMP, MNT.File),
    MMT.PASTE_FILE: (MMK.CREATE, MNT.File),
    MMT.CREATE_FILE: (MMK.CREATE, MNT.File),
    MMT.SOFT_DELETE_FILE: (MMK.DELETE, MNT.File),
    MMT.RESTORE_FILE: (MMK.CREATE, MNT.File),
    MMT.RENAME_FILE: (MMK.UPDATE, MNT.File),
    MMT.MOVE_FILE: (MMK.UPDATE, MNT.File),
    MMT.UPDATE_FILE: (MMK.UPDATE, MNT.File),
    MMT.DELETE_FILE: (MMK.DELETE, MNT.File),
    # Statements
    MMT.BUMP_STATEMENT: (MMK.BUMP, MNT.Statement),
    MMT.PASTE_STATEMENT: (MMK.CREATE, MNT.Statement),
    MMT.CREATE_STATEMENT: (MMK.CREATE, MNT.Statement),
    MMT.SOFT_DELETE_STATEMENT: (MMK.DELETE, MNT.Statement),
    MMT.RESTORE_STATEMENT: (MMK.CREATE, MNT.Statement),
    MMT.MORPH_STATEMENT: (MMK.UPDATE, MNT.Statement),
    MMT.MOVE_STATEMENT: (MMK.UPDATE, MNT.Statement),
    MMT.RENAME_STATEMENT: (MMK.UPDATE, MNT.Statement),
    MMT.UPDATE_STATEMENT: (MMK.UPDATE, MNT.Statement),
    MMT.DELETE_STATEMENT: (MMK.DELETE, MNT.Statement),
    MMT.UPDATE_STATEMENT_TEXT: (MMK.UPDATE, MNT.Statement),
    MMT.UPDATE_STATEMENT_FLAGS: (MMK.UPDATE, MNT.Statement),
    MMT.UPDATE_STATEMENT_HEADING_LEVEL: (MMK.UPDATE, MNT.Statement),
    MMT.UPDATE_STATEMENT_REFERENCE: (MMK.UPDATE, MNT.Statement),
    MMT.UPDATE_SYMBOL_text: (MMK.UPDATE, MNT.Statement),
    MMT.UPDATE_SYMBOL_CODE: (MMK.UPDATE, MNT.Statement),
    MMT.UPDATE_SYMBOL_MODIFIER: (MMK.UPDATE, MNT.Statement),
    MMT.UPDATE_SYMBOL_LANGUAGE: (MMK.UPDATE, MNT.Statement),
    MMT.UPDATE_SYMBOL_VALUE: (MMK.UPDATE, MNT.Statement),
    # Taggings
    MMT.CREATE_TAGGING: (MMK.CREATE, MNT.Tagging),
    MMT.UPDATE_TAGGING: (MMK.UPDATE, MNT.Tagging),
    MMT.DELETE_TAGGING: (MMK.DELETE, MNT.Tagging),
    MMT.SOFT_DELETE_TAGGING: (MMK.DELETE, MNT.Tagging),
    MMT.RESTORE_TAGGING: (MMK.CREATE, MNT.Tagging),
    MMT.MOVE_TAGGING: (MMK.UPDATE, MNT.Tagging),
    MMT.UPDATE_TAGGING_METADATA: (MMK.UPDATE, MNT.Tagging),
    # Triggers
    MMT.CREATE_TRIGGER: (MMK.CREATE, MNT.Trigger),
    MMT.UPDATE_TRIGGER: (MMK.UPDATE, MNT.Trigger),
    MMT.DELETE_TRIGGER: (MMK.DELETE, MNT.Trigger),
    MMT.SOFT_DELETE_TRIGGER: (MMK.DELETE, MNT.Trigger),
    MMT.RESTORE_TRIGGER: (MMK.CREATE, MNT.Trigger),
    # Fields
    MMT.CREATE_FIELD: (MMK.CREATE, MNT.Field),
    MMT.UPDATE_FIELD: (MMK.UPDATE, MNT.Field),
    MMT.RENAME_FIELD: (MMK.UPDATE, MNT.Field),
    MMT.UPDATE_FIELD_TEXT: (MMK.UPDATE, MNT.Field),
    MMT.UPDATE_FIELD_TYPE: (MMK.UPDATE, MNT.Field),
    MMT.UPDATE_FIELD_METADATA: (MMK.UPDATE, MNT.Field),
    MMT.MOVE_FIELD: (MMK.UPDATE, MNT.Field),
    MMT.DELETE_FIELD: (MMK.DELETE, MNT.Field),
    MMT.SOFT_DELETE_FIELD: (MMK.DELETE, MNT.Field),
    MMT.RESTORE_FIELD: (MMK.CREATE, MNT.Field),
    # Records
    MMT.TRUNCATE_RECORDS: (MMK.TRUNCATE, MNT.Record),
    MMT.CREATE_RECORD: (MMK.CREATE, MNT.Record),
    MMT.UPDATE_RECORD: (MMK.UPDATE, MNT.Record),
    MMT.DELETE_RECORD: (MMK.DELETE, MNT.Record),
    MMT.SOFT_DELETE_RECORD: (MMK.DELETE, MNT.Record),
    MMT.RESTORE_RECORD: (MMK.CREATE, MNT.Record),
    # Interp
    MMT.TRUNCATE_ISSUES: (MMK.TRUNCATE, MNT.Issue),
    MMT.CREATE_ISSUE: (MMK.CREATE, MNT.Issue),
    MMT.DELETE_ISSUE: (MMK.DELETE, MNT.Issue),
    MMT.TRUNCATE_RESOLVED_FIELDS: (MMK.TRUNCATE, MNT.ResolvedField),
    MMT.CREATE_RESOLVED_FIELD: (MMK.CREATE, MNT.ResolvedField),
    MMT.DELETE_RESOLVED_FIELD: (MMK.DELETE, MNT.ResolvedField),
}

# assert that all mutations are in the map
assert set(MMT) == set(_MODULE_MUTATION_MAP.keys()), "not all mutations are mapped"


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

    _data__mnt: Optional[ModuleNodeType] = None  # discriminator for 'union'
    _data: Optional[Any] = None  # the actual data, custom encode/decoded as union

    def encode_some_attrs(self):  # see serialize and :WireFormat
        # no special encoding of data here
        return {"thing": None}  # always omit thing

    @classmethod
    def decode_some_attrs(cls, data: dict[str, Any]) -> dict[str, Any]:
        from bench.language import wire

        _data = data.get("_data")
        if _data is not None:
            _data_cls = wire.DATA_CLASS_BY_MNT[data["_data__mnt"]]
            _data = from_dict(_data_cls, _data)
        return {"_data": _data}

    @property
    def data(self) -> Optional["NodeData"]:
        return self._data

    @data.setter
    def data(self, value: "NodeData"):
        from bench.language import wire

        self._data__mnt = wire.MNT_BY_DATA_CLASS[type(value)]
        self._data = value

    @property
    def scope(self) -> ModuleNodeType:
        if self._data__mnt is None:
            raise ValueError(f"mnt is not set on {self}")
        return self._data__mnt

    @property
    def mnt(self) -> ModuleNodeType:
        return self.type.mnt

    def __str__(self):
        data_str = f" {self.data}" if self.data else ""
        properties_str = (" [" + ", ".join(self.properties) + "]") if self.properties else ""
        return f"{self.type} {self.revision}{data_str}{properties_str}"

    def __repr__(self):
        return f"<Mutation {self}>"


def pack_node_flat_if_needed(node: Union[ModuleNode, "NodeData"]) -> "NodeData":
    from bench.language import wire

    if isinstance(node, wire.NodeData):
        return node
    else:
        return wire.pack_node_flat(node)


ModuleMutationHook = Callable[["ModuleMutator", ModuleMutation], None]


class ModuleMutator:
    """
    Create any apply mutations to a module tree.
    TODO @Cleanup: split module mutator into mutation creation and application
    """

    def __init__(
        self,
        tree: "NodeTree",
        project_id: UUID,
        module_id: UUID,
        # default file and statement id
        file_id: UUID = None,
        statement_id: UUID = None,
    ):
        self.tree = tree
        self.project_id = project_id
        self.module_id = module_id
        self.file_id = file_id
        self.statement_id = statement_id
        self.mutations = []

    def __str__(self):
        return f"mutate {len(self.mutations)} {self.module_id}"

    def __repr__(self):
        return f"<Mutator {self}>"

    def reset(self):
        self.mutations = []

    def do(
        self, type: MMT, node: "NodeData", apply: bool = True, properties: list[str] = None
    ) -> "ModuleMutator":
        from bench.language import wire

        if isinstance(node, wire.StatementData):
            statement_id = node.id
            file_id = self.file_id or self.tree.get_ancestor(node.parent_id, wire.FileData).id
        elif isinstance(node, wire.FileData):
            statement_id = None
            file_id = node.id
        elif isinstance(node, wire.ModuleData):
            statement_id = None
            file_id = None
        else:
            if self.statement_id:
                statement_id = self.statement_id
            else:
                statement = self.tree.get_ancestor(node.parent_id, wire.StatementData)
                statement_id = statement.id if statement else None
            if self.file_id:
                file_id = self.file_id
            else:
                file = self.tree.get_ancestor(node.parent_id, wire.FileData)
                file_id = self.file_id or file.id
        if properties and type.kind != MMK.UPDATE:
            raise ValueError(f"properties only supported for update mutations: {properties}")
        mutation = ModuleMutation(
            type=type,
            project_version_id=self.module_id,
            revision=node.revision if isinstance(node, wire.HasCrud) else None,
            file_id=file_id,
            statement_id=statement_id,
            properties=properties,
        )
        mutation.data = node
        self.mutations.append(mutation)
        if apply:
            self.apply(mutation)
        return self

    def apply(self, mut: ModuleMutation):
        from bench.language.wire import DATA_CLASS_BY_MNT

        try:
            if mut.type.kind == MMK.CREATE:
                self.tree.add(mut.data)
            elif mut.type.kind == MMK.UPDATE:
                self.tree.replace(mut.data)
            elif mut.type.kind == MMK.DELETE:
                self.tree.remove(mut.data)
            elif mut.type.kind == MMK.TRUNCATE:
                self.tree.truncate(mut.data, DATA_CLASS_BY_MNT[mut.mnt])
            else:
                raise ValueError(f"unexpected mutation kind {mut}")
        except Exception as e:
            raise ValueError(f"failed to apply {mut} to {self.tree!r}") from e

    def apply_all(self, mutations: list[ModuleMutation]):
        for mut in mutations:
            self.apply(mut)

    def truncate(
        self, node: Union["NodeData", ModuleNode], mnt: MNT, apply: bool = True
    ) -> "ModuleMutator":
        """Truncates all records of the given statement."""
        node = pack_node_flat_if_needed(node)
        mmt = MMT(f"TRUNCATE_{mnt.caps_name}S")
        self.do(mmt, node, apply=apply)
        return self

    def create_many(
        self, *nodes: Union["NodeData", ModuleNode], apply: bool = True
    ) -> "ModuleMutator":
        for obj in nodes:
            self.create(obj, apply=apply)
        return self

    def create(self, node: Union["NodeData", ModuleNode], apply: bool = True) -> "ModuleMutator":
        from bench.language.wire import MNT_BY_DATA_CLASS

        node = pack_node_flat_if_needed(node)
        mnt = MNT_BY_DATA_CLASS[type(node)]
        mmt = MMT(f"CREATE_{mnt.caps_name}")
        self.do(mmt, node, apply=apply)
        return self

    def update_many(
        self, *nodes: Union["NodeData", ModuleNode], apply: bool = True
    ) -> "ModuleMutator":
        for obj in nodes:
            self.update(obj, apply=apply)
        return self

    def update(
        self, node: Union["NodeData", ModuleNode], apply: bool = True, properties: list[str] = None
    ) -> "ModuleMutator":
        from bench.language.wire import MNT_BY_DATA_CLASS

        node = pack_node_flat_if_needed(node)
        mnt = MNT_BY_DATA_CLASS[type(node)]
        mmt = MMT(f"UPDATE_{mnt.caps_name}")
        self.do(mmt, node, apply=apply, properties=properties)
        return self

    def delete_many(
        self, *nodes: Union["NodeData", ModuleNode], apply: bool = True
    ) -> "ModuleMutator":
        for obj in nodes:
            self.delete(obj, apply=apply)
        return self

    def delete(self, node: Union["NodeData", ModuleNode], apply: bool = True) -> "ModuleMutator":
        from bench.language.wire import MNT_BY_DATA_CLASS

        node = pack_node_flat_if_needed(node)
        mnt = MNT_BY_DATA_CLASS[type(node)]
        mmt = MMT(f"DELETE_{mnt.caps_name}")
        self.do(mmt, node, apply=apply)
        return self

    def bundle(self) -> "MutationBundle":
        return MutationBundle(self.mutations)


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

    # TODO @Performance: mutation compaction & batching can be much smarter
    def compact(self) -> list[ModuleMutation]:
        """
        Compact simple mutations into fewer semantically identical mutations.

        Reduces:
         1. Successive updates to same object merged into the last update
         2. Successive deletes of same object merged into the last delete
        Not implemented yet:
         3. Delete after create to nothing
         4. Create then updated merged into a single create
        """
        if not self.simple:
            raise ValueError(f"cannot collapse complex mutations: {self}")

        reduced_inverse = []
        seen_ops: dict[tuple[MMT, UUID], ModuleMutation] = {}

        for mutation in reversed(self.mutations):
            key = (mutation.type, mutation.data.id)
            if key in seen_ops:
                if mutation.type.kind == MMK.UPDATE:
                    # merge properties
                    seen_ops[key].properties.extend(mutation.properties)
                    seen_ops[key].properties = list(set(seen_ops[key].properties))
            seen_ops[key] = mutation
            reduced_inverse.append(mutation)

        reduced = list(reversed(reduced_inverse))
        return reduced

    def batched_apply(
        self, module: NodeTree, project_id: UUID, module_id: UUID, apply: bool = True
    ) -> Iterator[tuple[MMT, list[ModuleMutation]]]:
        """
        Batch consecutive mutations by type in order of appearance
         AND optionally concurrently apply them to the given module tree.
        (there may be multiple batches of the same type).
        """

        mutator = ModuleMutator(module, project_id, module_id)
        current_batch: list[ModuleMutation] = []
        current_type: MMT | None = None

        for mutation in self.mutations:
            if mutation.type != current_type:
                if current_type is not None:
                    yield current_type, current_batch
                current_type = mutation.type
                current_batch = []
            current_batch.append(mutation)
            if apply:
                mutator.apply(mutation)

        if current_batch:
            yield current_type, current_batch


def diff_modules(old_module: ModuleTreeData, new_module: ModuleTreeData) -> list[ModuleMutation]:
    """
    Get the mutations needed to transform old_module into new_module.
    Find nodes by their id (not ck).
    """
    mutator = ModuleMutator(old_module)
    old_tree = NodeTree(old_module.nodes)
    new_tree = NodeTree(new_module.nodes)

    for new_node in new_tree.walk_bfs():
        if new_node.mnt == ModuleNodeType.Module:
            continue  # ignore module itself
        if new_node.id not in old_tree.nodes_by_id:
            mutator.create(new_node)
        else:
            old_node = old_tree.nodes_by_id[new_node.id]
            if not new_node.equals_no_cru(old_node):
                mutator.update(new_node)
    for old_node in old_tree.walk_bfs():
        if old_node.mnt == ModuleNodeType.Module:
            continue
        if old_node.id not in new_tree.nodes_by_id:
            mutator.delete(old_node)
    # sort into delete -> create -> update
    mutations = [
        *(m for m in mutator.mutations if m.type.kind == MMK.DELETE),
        *(m for m in mutator.mutations if m.type.kind == MMK.CREATE),
        *(m for m in mutator.mutations if m.type.kind == MMK.UPDATE),
    ]
    return mutations


def create_module(module: ModuleTreeData) -> list[ModuleMutation]:
    """
    Get the mutations needed to create a new module.
    """
    mutator = ModuleMutator(module)
    for node in NodeTree(module.nodes).walk_bfs():
        if node.mnt == ModuleNodeType.Module:
            continue  # ignore module itself
        mutator.create(node)
    return mutator.mutations
