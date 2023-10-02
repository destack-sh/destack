"""
Structs for syncing project model contents across servers and clients.
It shouldn't live in models, so we can use it in messages.py, which shouldn't depend on Django.
Maybe a better move would be to make the payload partially opaque and keep this in api.
"""
import enum
from dataclasses import dataclass
from functools import cached_property
from typing import Any, Iterator, Optional, Union
from uuid import UUID

from bench.language.const import ModuleNodeType
from bench.language.module import ModuleNode, NodeTree
from bench.language.wire import ModuleTreeData, NodeData
from bench.utils.serialize import from_dict


class EditType(enum.StrEnum):
    """Fine-grained atomic edits for multiplayer modules."""

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
    def kind(self) -> "EditKind":
        return _MODULE_EDIT_MAP[self][0]

    @property
    def mnt(self) -> "ModuleNodeType":
        return _MODULE_EDIT_MAP[self][1]

    @property
    def simple(self) -> bool:
        return self in SIMPLE_EDITS

    @staticmethod
    def from_nt(mmk: "EditKind", nt: "ModuleNodeType") -> "EditType":
        return EditType(f"{mmk.value}_{nt.value}")


class EditKind(enum.StrEnum):
    CREATE = "CREATE"
    UPDATE = "UPDATE"
    DELETE = "DELETE"
    TRUNCATE = "TRUNCATE"
    BUMP = "BUMP"


# Basic CUD edits with full (flat) data for the model
SIMPLE_EDITS = {
    # File
    EditType.BUMP_FILE,
    EditType.CREATE_FILE,
    EditType.UPDATE_FILE,
    EditType.DELETE_FILE,
    # Statement
    EditType.BUMP_STATEMENT,
    EditType.CREATE_STATEMENT,
    EditType.UPDATE_STATEMENT,
    EditType.DELETE_STATEMENT,
    # Taggings
    EditType.CREATE_TAGGING,
    EditType.UPDATE_TAGGING,
    EditType.DELETE_TAGGING,
    # Triggers
    EditType.CREATE_TRIGGER,
    EditType.UPDATE_TRIGGER,
    EditType.DELETE_TRIGGER,
    # Fields
    EditType.CREATE_FIELD,
    EditType.UPDATE_FIELD,
    EditType.DELETE_FIELD,
    # Record
    EditType.TRUNCATE_RECORDS,
    EditType.CREATE_RECORD,
    EditType.UPDATE_RECORD,
    EditType.DELETE_RECORD,
    # Interp
    EditType.TRUNCATE_ISSUES,
    EditType.CREATE_ISSUE,
    EditType.TRUNCATE_RESOLVED_FIELDS,
    EditType.CREATE_RESOLVED_FIELD,
}

MET = EditType
MEK = EditKind
MNT = ModuleNodeType

_MODULE_EDIT_MAP: dict[MET, tuple[MEK, MNT]] = {
    # Files
    MET.BUMP_FILE: (MEK.BUMP, MNT.File),
    MET.PASTE_FILE: (MEK.CREATE, MNT.File),
    MET.CREATE_FILE: (MEK.CREATE, MNT.File),
    MET.SOFT_DELETE_FILE: (MEK.DELETE, MNT.File),
    MET.RESTORE_FILE: (MEK.CREATE, MNT.File),
    MET.RENAME_FILE: (MEK.UPDATE, MNT.File),
    MET.MOVE_FILE: (MEK.UPDATE, MNT.File),
    MET.UPDATE_FILE: (MEK.UPDATE, MNT.File),
    MET.DELETE_FILE: (MEK.DELETE, MNT.File),
    # Statements
    MET.BUMP_STATEMENT: (MEK.BUMP, MNT.Statement),
    MET.PASTE_STATEMENT: (MEK.CREATE, MNT.Statement),
    MET.CREATE_STATEMENT: (MEK.CREATE, MNT.Statement),
    MET.SOFT_DELETE_STATEMENT: (MEK.DELETE, MNT.Statement),
    MET.RESTORE_STATEMENT: (MEK.CREATE, MNT.Statement),
    MET.MORPH_STATEMENT: (MEK.UPDATE, MNT.Statement),
    MET.MOVE_STATEMENT: (MEK.UPDATE, MNT.Statement),
    MET.RENAME_STATEMENT: (MEK.UPDATE, MNT.Statement),
    MET.UPDATE_STATEMENT: (MEK.UPDATE, MNT.Statement),
    MET.DELETE_STATEMENT: (MEK.DELETE, MNT.Statement),
    MET.UPDATE_STATEMENT_TEXT: (MEK.UPDATE, MNT.Statement),
    MET.UPDATE_STATEMENT_FLAGS: (MEK.UPDATE, MNT.Statement),
    MET.UPDATE_STATEMENT_HEADING_LEVEL: (MEK.UPDATE, MNT.Statement),
    MET.UPDATE_STATEMENT_REFERENCE: (MEK.UPDATE, MNT.Statement),
    MET.UPDATE_SYMBOL_text: (MEK.UPDATE, MNT.Statement),
    MET.UPDATE_SYMBOL_CODE: (MEK.UPDATE, MNT.Statement),
    MET.UPDATE_SYMBOL_MODIFIER: (MEK.UPDATE, MNT.Statement),
    MET.UPDATE_SYMBOL_LANGUAGE: (MEK.UPDATE, MNT.Statement),
    MET.UPDATE_SYMBOL_VALUE: (MEK.UPDATE, MNT.Statement),
    # Taggings
    MET.CREATE_TAGGING: (MEK.CREATE, MNT.Tagging),
    MET.UPDATE_TAGGING: (MEK.UPDATE, MNT.Tagging),
    MET.DELETE_TAGGING: (MEK.DELETE, MNT.Tagging),
    MET.SOFT_DELETE_TAGGING: (MEK.DELETE, MNT.Tagging),
    MET.RESTORE_TAGGING: (MEK.CREATE, MNT.Tagging),
    MET.MOVE_TAGGING: (MEK.UPDATE, MNT.Tagging),
    MET.UPDATE_TAGGING_METADATA: (MEK.UPDATE, MNT.Tagging),
    # Triggers
    MET.CREATE_TRIGGER: (MEK.CREATE, MNT.Trigger),
    MET.UPDATE_TRIGGER: (MEK.UPDATE, MNT.Trigger),
    MET.DELETE_TRIGGER: (MEK.DELETE, MNT.Trigger),
    MET.SOFT_DELETE_TRIGGER: (MEK.DELETE, MNT.Trigger),
    MET.RESTORE_TRIGGER: (MEK.CREATE, MNT.Trigger),
    # Fields
    MET.CREATE_FIELD: (MEK.CREATE, MNT.Field),
    MET.UPDATE_FIELD: (MEK.UPDATE, MNT.Field),
    MET.RENAME_FIELD: (MEK.UPDATE, MNT.Field),
    MET.UPDATE_FIELD_TEXT: (MEK.UPDATE, MNT.Field),
    MET.UPDATE_FIELD_TYPE: (MEK.UPDATE, MNT.Field),
    MET.UPDATE_FIELD_METADATA: (MEK.UPDATE, MNT.Field),
    MET.MOVE_FIELD: (MEK.UPDATE, MNT.Field),
    MET.DELETE_FIELD: (MEK.DELETE, MNT.Field),
    MET.SOFT_DELETE_FIELD: (MEK.DELETE, MNT.Field),
    MET.RESTORE_FIELD: (MEK.CREATE, MNT.Field),
    # Records
    MET.TRUNCATE_RECORDS: (MEK.TRUNCATE, MNT.Record),
    MET.CREATE_RECORD: (MEK.CREATE, MNT.Record),
    MET.UPDATE_RECORD: (MEK.UPDATE, MNT.Record),
    MET.DELETE_RECORD: (MEK.DELETE, MNT.Record),
    MET.SOFT_DELETE_RECORD: (MEK.DELETE, MNT.Record),
    MET.RESTORE_RECORD: (MEK.CREATE, MNT.Record),
    # Interp
    MET.TRUNCATE_ISSUES: (MEK.TRUNCATE, MNT.Issue),
    MET.CREATE_ISSUE: (MEK.CREATE, MNT.Issue),
    MET.DELETE_ISSUE: (MEK.DELETE, MNT.Issue),
    MET.TRUNCATE_RESOLVED_FIELDS: (MEK.TRUNCATE, MNT.ResolvedField),
    MET.CREATE_RESOLVED_FIELD: (MEK.CREATE, MNT.ResolvedField),
    MET.DELETE_RESOLVED_FIELD: (MEK.DELETE, MNT.ResolvedField),
}

# assert that all edits are in the map
assert set(MET) == set(_MODULE_EDIT_MAP.keys()), "not all edits are mapped"


@dataclass(repr=False, slots=True)
class Edit:
    type: MET
    project_version_id: UUID
    file_id: Optional[UUID] = None
    statement_id: Optional[UUID] = None
    revision: Optional[int] = None
    input: Optional[dict[str, Any]] = None  # for GQL edits
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
        return f"<Edit {self}>"


def pack_node_flat_if_needed(node: Union[ModuleNode, "NodeData"]) -> "NodeData":
    from bench.language import wire

    if isinstance(node, wire.NodeData):
        return node
    else:
        return wire.pack_node_flat(node)


class ModuleEditor:
    """
    Create any apply edits to a module.
    TODO @Cleanup: split module mutator into edit creation and application
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
        self.edits = []

    def __str__(self):
        return f"edit {len(self.edits)} {self.module_id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    def reset(self):
        self.edits = []

    def _do(
        self, type: MET, node: "NodeData", apply: bool = True, properties: list[str] = None
    ) -> "ModuleEditor":
        from bench.language import wire

        if isinstance(node, wire.StatementData):
            statement_id = node.id
            file_id = self.file_id or self.tree.get_ancestor(node.parent_id, MNT.File).id
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
                statement = self.tree.get_ancestor(node.parent_id, MNT.Statement)
                statement_id = statement.id if statement else None
            if self.file_id:
                file_id = self.file_id
            else:
                file_id = self.tree.get_ancestor(node.parent_id, MNT.File).id
        if properties and type.kind != MEK.UPDATE:
            raise ValueError(f"properties only supported for update edits: {properties}")
        edit = Edit(
            type=type,
            project_version_id=self.module_id,
            revision=node.revision if isinstance(node, wire.HasCrud) else None,
            file_id=file_id,
            statement_id=statement_id,
            properties=properties,
        )
        edit.data = node
        self.edits.append(edit)
        if apply:
            self.apply(edit)
        return self

    def apply(self, edit: Edit, raise_on_error: bool = True):
        try:
            if edit.type.kind == MEK.CREATE:
                self.tree.add(edit.data)
            elif edit.type.kind == MEK.UPDATE:
                self.tree.replace(edit.data)
            elif edit.type.kind == MEK.DELETE:
                self.tree.remove(edit.data)
            elif edit.type.kind == MEK.TRUNCATE:
                self.tree.truncate(edit.data, edit.mnt)
            else:
                raise ValueError(f"unexpected edit kind {edit}")
        except Exception as e:
            if raise_on_error:
                raise ValueError(f"failed to apply {edit} to {self.tree!r}") from e

    def apply_all(self, edits: list[Edit], raise_on_error: bool = True):
        for e in edits:
            self.apply(e, raise_on_error=raise_on_error)

    def truncate(
        self, node: Union["NodeData", ModuleNode], mnt: MNT, apply: bool = True
    ) -> "ModuleEditor":
        node = pack_node_flat_if_needed(node)
        mmt = MET(f"TRUNCATE_{mnt.caps_name}S")
        self._do(mmt, node, apply=apply)
        return self

    def create_many(
        self, *nodes: Union["NodeData", ModuleNode], apply: bool = True
    ) -> "ModuleEditor":
        for obj in nodes:
            self.create(obj, apply=apply)
        return self

    def create(self, node: Union["NodeData", ModuleNode], apply: bool = True) -> "ModuleEditor":
        from bench.language.wire import MNT_BY_DATA_CLASS

        node = pack_node_flat_if_needed(node)
        mnt = MNT_BY_DATA_CLASS[type(node)]
        mmt = MET(f"CREATE_{mnt.caps_name}")
        self._do(mmt, node, apply=apply)
        return self

    def update_many(
        self, *nodes: Union["NodeData", ModuleNode], apply: bool = True
    ) -> "ModuleEditor":
        for obj in nodes:
            self.update(obj, apply=apply)
        return self

    def update(
        self, node: Union["NodeData", ModuleNode], apply: bool = True, properties: list[str] = None
    ) -> "ModuleEditor":
        from bench.language.wire import MNT_BY_DATA_CLASS

        assert isinstance(properties, list) or properties is None, f"invalid props: {properties}"

        node = pack_node_flat_if_needed(node)
        mnt = MNT_BY_DATA_CLASS[type(node)]
        mmt = MET(f"UPDATE_{mnt.caps_name}")
        self._do(mmt, node, apply=apply, properties=properties)
        return self

    def delete_many(
        self, *nodes: Union["NodeData", ModuleNode], apply: bool = True
    ) -> "ModuleEditor":
        for obj in nodes:
            self.delete(obj, apply=apply)
        return self

    def delete(self, node: Union["NodeData", ModuleNode], apply: bool = True) -> "ModuleEditor":
        from bench.language.wire import MNT_BY_DATA_CLASS

        node = pack_node_flat_if_needed(node)
        mnt = MNT_BY_DATA_CLASS[type(node)]
        mmt = MET(f"DELETE_{mnt.caps_name}")
        self._do(mmt, node, apply=apply)
        return self

    def bundle(self) -> "EditBundle":
        return EditBundle(self.edits)


class EditBundle:
    """Indexed access to a constant list of edits."""

    def __init__(self, edits: list[Edit]):
        self.edits = edits

    def __str__(self):
        return f"edit {len(self.edits)}"

    def __repr__(self):
        return f"<EditBundle {self}>"

    @cached_property
    def simple(self) -> bool:
        return not any(m.type not in SIMPLE_EDITS for m in self.edits)

    @property
    def complex_edits(self):
        return [e for e in self.edits if e.type not in SIMPLE_EDITS]

    # TODO @Performance: edit compaction & batching can be much smarter
    #  But we may also want to record these in full as events... compact before write only?
    def compact(self) -> list[Edit]:
        """
        Compact simple edits into fewer semantically identical edits.

        Reduces:
         1. Successive updates to same object merged into the last update
         2. Successive deletes of same object merged into the last delete
        Not implemented yet:
         3. Delete after create to nothing
         4. Create then updated merged into a single create
        """
        if not self.simple:
            raise ValueError(f"cannot collapse complex edits: {self}")

        reduced_inverse = []
        seen_ops: dict[tuple[MET, UUID], Edit] = {}

        for edit in reversed(self.edits):
            key = (edit.type, edit.data.id)
            if key in seen_ops:
                if edit.type.kind == MEK.UPDATE:
                    # merge properties
                    seen_ops[key].properties.extend(edit.properties)
                    seen_ops[key].properties = list(set(seen_ops[key].properties))
                continue
            seen_ops[key] = edit
            reduced_inverse.append(edit)

        reduced = list(reversed(reduced_inverse))
        return reduced

    def batched_apply(
        self, module: NodeTree, project_id: UUID, module_id: UUID, apply: bool = True
    ) -> Iterator[tuple[MET, list[Edit]]]:
        """
        Batch consecutive edits by type in order of appearance
         AND optionally concurrently apply them to the given module tree.
        (there may be multiple batches of the same type).
        """

        editor = ModuleEditor(module, project_id, module_id)
        current_batch: list[Edit] = []
        current_type: MET | None = None

        for edit in self.edits:
            if edit.type != current_type:
                if current_type is not None:
                    yield current_type, current_batch
                current_type = edit.type
                current_batch = []
            current_batch.append(edit)
            if apply:
                editor.apply(edit)

        if current_batch:
            yield current_type, current_batch


def diff_modules(
    old_module: ModuleTreeData, new_module: ModuleTreeData, project_id: UUID
) -> list[Edit]:
    """
    Get the edits needed to transform old_module into new_module.
    Find nodes by their id (not ck).
    """
    old_tree = NodeTree(old_module.nodes)
    editor = ModuleEditor(old_tree, old_module.id, project_id)
    new_tree = NodeTree(new_module.nodes)

    for new_node in new_tree.walk_bfs():
        if new_node.mnt == ModuleNodeType.Module:
            continue  # ignore module itself
        if new_node.id not in old_tree.nodes_by_id:
            editor.create(new_node)
        else:
            old_node = old_tree.nodes_by_id[new_node.id]
            if not new_node.equals_no_cru(old_node):
                editor.update(new_node)
    for old_node in old_tree.walk_bfs():
        if old_node.mnt == ModuleNodeType.Module:
            continue
        if old_node.id not in new_tree.nodes_by_id:
            editor.delete(old_node)
    # sort into delete -> create -> update
    edits = [
        *(e for e in editor.edits if e.type.kind == MEK.DELETE),
        *(e for e in editor.edits if e.type.kind == MEK.CREATE),
        *(e for e in editor.edits if e.type.kind == MEK.UPDATE),
    ]
    return edits


def create_module(module: ModuleTreeData) -> list[Edit]:
    """
    Get the edits needed to create a new module.
    """
    editor = ModuleEditor(module)
    for node in NodeTree(module.nodes).walk_bfs():
        if node.mnt == ModuleNodeType.Module:
            continue  # ignore module itself
        editor.create(node)
    return editor.edits
