"""
Structs for syncing project model contents across servers and clients.
It shouldn't live in models, so we can use it in messages.py, which shouldn't depend on Django.
Maybe a better move would be to make the payload partially opaque and keep this in api.
"""
import enum
from dataclasses import dataclass
from functools import cached_property
from typing import (
    TYPE_CHECKING,
    Any,
    Generator,
    Iterator,
    Mapping,
    NamedTuple,
    Optional,
    Union,
)
from uuid import UUID

from more_itertools import first

from bench.language.const import INTERP_NODE_TYPES, ModuleNodeType, TypeFlag, TypeTag
from bench.language.module import UNSET, Module, Node, NodeTree, NRel
from bench.language.text import Text, render_text_simple
from bench.utils.serialize import from_dict
from bench.utils.utils import omit_empty

if TYPE_CHECKING:
    from bench.language import File, Run, Statement
    from bench.language.wire import ModuleTreeData, NodeData


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


# Basic CUD edits with full (flat) data for the node
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
    MET.BUMP_FILE: (MEK.BUMP, MNT.FILE),
    MET.PASTE_FILE: (MEK.CREATE, MNT.FILE),
    MET.CREATE_FILE: (MEK.CREATE, MNT.FILE),
    MET.SOFT_DELETE_FILE: (MEK.DELETE, MNT.FILE),
    MET.RESTORE_FILE: (MEK.CREATE, MNT.FILE),
    MET.RENAME_FILE: (MEK.UPDATE, MNT.FILE),
    MET.MOVE_FILE: (MEK.UPDATE, MNT.FILE),
    MET.UPDATE_FILE: (MEK.UPDATE, MNT.FILE),
    MET.DELETE_FILE: (MEK.DELETE, MNT.FILE),
    # Statements
    MET.BUMP_STATEMENT: (MEK.BUMP, MNT.STATEMENT),
    MET.PASTE_STATEMENT: (MEK.CREATE, MNT.STATEMENT),
    MET.CREATE_STATEMENT: (MEK.CREATE, MNT.STATEMENT),
    MET.SOFT_DELETE_STATEMENT: (MEK.DELETE, MNT.STATEMENT),
    MET.RESTORE_STATEMENT: (MEK.CREATE, MNT.STATEMENT),
    MET.MORPH_STATEMENT: (MEK.UPDATE, MNT.STATEMENT),
    MET.MOVE_STATEMENT: (MEK.UPDATE, MNT.STATEMENT),
    MET.RENAME_STATEMENT: (MEK.UPDATE, MNT.STATEMENT),
    MET.UPDATE_STATEMENT: (MEK.UPDATE, MNT.STATEMENT),
    MET.DELETE_STATEMENT: (MEK.DELETE, MNT.STATEMENT),
    MET.UPDATE_STATEMENT_TEXT: (MEK.UPDATE, MNT.STATEMENT),
    MET.UPDATE_STATEMENT_FLAGS: (MEK.UPDATE, MNT.STATEMENT),
    MET.UPDATE_STATEMENT_HEADING_LEVEL: (MEK.UPDATE, MNT.STATEMENT),
    MET.UPDATE_STATEMENT_REFERENCE: (MEK.UPDATE, MNT.STATEMENT),
    MET.UPDATE_SYMBOL_text: (MEK.UPDATE, MNT.STATEMENT),
    MET.UPDATE_SYMBOL_CODE: (MEK.UPDATE, MNT.STATEMENT),
    MET.UPDATE_SYMBOL_MODIFIER: (MEK.UPDATE, MNT.STATEMENT),
    MET.UPDATE_SYMBOL_LANGUAGE: (MEK.UPDATE, MNT.STATEMENT),
    MET.UPDATE_SYMBOL_VALUE: (MEK.UPDATE, MNT.STATEMENT),
    # Taggings
    MET.CREATE_TAGGING: (MEK.CREATE, MNT.TAGGING),
    MET.UPDATE_TAGGING: (MEK.UPDATE, MNT.TAGGING),
    MET.DELETE_TAGGING: (MEK.DELETE, MNT.TAGGING),
    MET.SOFT_DELETE_TAGGING: (MEK.DELETE, MNT.TAGGING),
    MET.RESTORE_TAGGING: (MEK.CREATE, MNT.TAGGING),
    MET.MOVE_TAGGING: (MEK.UPDATE, MNT.TAGGING),
    MET.UPDATE_TAGGING_METADATA: (MEK.UPDATE, MNT.TAGGING),
    # Triggers
    MET.CREATE_TRIGGER: (MEK.CREATE, MNT.TRIGGER),
    MET.UPDATE_TRIGGER: (MEK.UPDATE, MNT.TRIGGER),
    MET.DELETE_TRIGGER: (MEK.DELETE, MNT.TRIGGER),
    MET.SOFT_DELETE_TRIGGER: (MEK.DELETE, MNT.TRIGGER),
    MET.RESTORE_TRIGGER: (MEK.CREATE, MNT.TRIGGER),
    # Fields
    MET.CREATE_FIELD: (MEK.CREATE, MNT.FIELD),
    MET.UPDATE_FIELD: (MEK.UPDATE, MNT.FIELD),
    MET.RENAME_FIELD: (MEK.UPDATE, MNT.FIELD),
    MET.UPDATE_FIELD_TEXT: (MEK.UPDATE, MNT.FIELD),
    MET.UPDATE_FIELD_TYPE: (MEK.UPDATE, MNT.FIELD),
    MET.UPDATE_FIELD_METADATA: (MEK.UPDATE, MNT.FIELD),
    MET.MOVE_FIELD: (MEK.UPDATE, MNT.FIELD),
    MET.DELETE_FIELD: (MEK.DELETE, MNT.FIELD),
    MET.SOFT_DELETE_FIELD: (MEK.DELETE, MNT.FIELD),
    MET.RESTORE_FIELD: (MEK.CREATE, MNT.FIELD),
    # Records
    MET.TRUNCATE_RECORDS: (MEK.TRUNCATE, MNT.RECORD),
    MET.CREATE_RECORD: (MEK.CREATE, MNT.RECORD),
    MET.UPDATE_RECORD: (MEK.UPDATE, MNT.RECORD),
    MET.DELETE_RECORD: (MEK.DELETE, MNT.RECORD),
    MET.SOFT_DELETE_RECORD: (MEK.DELETE, MNT.RECORD),
    MET.RESTORE_RECORD: (MEK.CREATE, MNT.RECORD),
    # Interp
    MET.TRUNCATE_ISSUES: (MEK.TRUNCATE, MNT.ISSUE),
    MET.CREATE_ISSUE: (MEK.CREATE, MNT.ISSUE),
    MET.DELETE_ISSUE: (MEK.DELETE, MNT.ISSUE),
    MET.TRUNCATE_RESOLVED_FIELDS: (MEK.TRUNCATE, MNT.RESOLVED_FIELD),
    MET.CREATE_RESOLVED_FIELD: (MEK.CREATE, MNT.RESOLVED_FIELD),
    MET.DELETE_RESOLVED_FIELD: (MEK.DELETE, MNT.RESOLVED_FIELD),
}

# assert that all edits are in the map
assert set(MET) == set(_MODULE_EDIT_MAP.keys()), "not all edits are mapped"


@dataclass
class Edit:
    """
    An edit to a module/node.
    TODO @Cleanup @Architecture: use new :Edit where possible (see :BE-114)
     also track Edit.edited_by (for Run to enable undo)
    """

    type: MET
    module: "Module"
    node: "Node"  # the node that was edited
    revision: Optional[int] = None  # server revision of node after edit is accepted
    file: Optional["File"] = None  # ancestor file before edit
    statement: Optional["Statement"] = None  # ancestor statement before edit
    properties: dict[str, Any] | None = None  # changed properties (by language name)
    old_properties: dict[str, Any] | None = None  # if edit or hard delete
    edited_by: Optional["Run"] = None

    def undo(self):
        raise NotImplementedError

    def redo(self):
        raise NotImplementedError

    @property
    def kind(self) -> EditKind:
        """The kind of edit (create, update, delete, truncate, bump)."""
        return self.type.kind

    @property
    def scope(self) -> ModuleNodeType:
        """The type of node that was edited. Usually the same as mnt except for truncate."""
        return self.node.mnt

    @property
    def mnt(self) -> ModuleNodeType:
        """The type of node that was edited."""
        return self.type.mnt


@dataclass
class EditData:
    type: MET
    project_version_id: UUID
    file_id: Optional[UUID] = None
    statement_id: Optional[UUID] = None
    revision: Optional[int] = None
    input: Optional[dict[str, Any]] = None  # for GQL edits
    properties: Optional[list[str]] = None  # changed properties (by language name), see :Edit
    thing: Optional[Any] = None  # in-memory object that was mutated, not serialized
    _node_mnt: Optional[ModuleNodeType] = None  # discriminator for 'union'
    _node: Optional[Any] = None  # the actual data, custom encode/decoded as union

    def encode_some_attrs(self):  # see serialize and :WireFormat
        # no special encoding of data here
        return {"thing": None}  # always omit thing

    @classmethod
    def decode_some_attrs(cls, data: dict[str, Any]) -> dict[str, Any]:
        from bench.language import wire

        _node = data.get("_node")
        if _node is not None:
            _data_cls = wire.DATA_CLASS_BY_MNT[data["_node_mnt"]]
            _node = from_dict(_data_cls, _node)
        return {"_node": _node}

    @property
    def node(self) -> Optional["NodeData"]:
        return self._node

    @node.setter
    def node(self, node: "NodeData"):
        from bench.language import wire

        self._node_mnt = wire.MNT_BY_DATA_CLASS[type(node)]
        self._node = node

    @property
    def kind(self) -> EditKind:
        return self.type.kind

    @property
    def scope(self) -> ModuleNodeType:
        assert self._node_mnt is not None, f"mnt is not set on {self!r}"
        return self._node_mnt

    @property
    def mnt(self) -> ModuleNodeType:
        return self.type.mnt

    def __str__(self):
        data_str = f" {self.node}" if self.node else ""
        properties_str = (" [" + ", ".join(self.properties) + "]") if self.properties else ""
        return f"{self.type} {self.revision}{data_str}{properties_str}"

    def __repr__(self):
        return f"<Edit {self}>"


def pack_node_flat_if_needed(node: Union[Node, "NodeData"]) -> "NodeData":
    from bench.language import wire

    if isinstance(node, wire.NodeData):
        return node
    else:
        return wire.pack_node_flat(node)


class ModuleEditor:
    """
    Create any apply edits to a module.
    TODO @Cleanup: split module mutator into edit creation and application
     also @Performance: pre-filter edits to track
      (e.g. to exclude interp edits in worker, see :InterpFilter)
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
            file_id = self.file_id or self.tree.get_ancestor(node.parent_id, MNT.FILE).id
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
                statement = self.tree.get_ancestor(node.parent_id, MNT.STATEMENT)
                statement_id = statement.id if statement else None
            if self.file_id:
                file_id = self.file_id
            else:
                file_id = self.tree.get_ancestor(node.parent_id, MNT.FILE).id
        edit = EditData(
            type=type,
            project_version_id=self.module_id,
            revision=node.revision if isinstance(node, wire.HasCrud) else None,
            file_id=file_id,
            statement_id=statement_id,
            properties=properties,
        )
        edit.node = node
        self.edits.append(edit)
        if apply:
            self.apply(edit)
        return self

    def apply(self, edit: EditData, raise_on_error: bool = True):
        try:
            if edit.type.kind == MEK.CREATE:
                self.tree.add(edit.node)
            elif edit.type.kind == MEK.UPDATE:
                self.tree.replace(edit.node)
            elif edit.type.kind == MEK.DELETE:
                self.tree.remove(edit.node)
            elif edit.type.kind == MEK.TRUNCATE:
                self.tree.truncate(edit.node, edit.mnt)
            else:
                raise ValueError(f"unexpected edit kind {edit}")
        except Exception as e:
            if raise_on_error:
                raise ValueError(f"failed to apply {edit} to {self.tree!r}") from e

    def apply_all(self, edits: list[EditData], raise_on_error: bool = True):
        for e in edits:
            self.apply(e, raise_on_error=raise_on_error)

    def truncate(
        self, node: Union["NodeData", Node], mnt: MNT, apply: bool = True
    ) -> "ModuleEditor":
        node = pack_node_flat_if_needed(node)
        mmt = MET(f"TRUNCATE_{mnt.caps_name}S")
        self._do(mmt, node, apply=apply)
        return self

    def create_many(self, *nodes: Union["NodeData", Node], apply: bool = True) -> "ModuleEditor":
        for obj in nodes:
            self.create(obj, apply=apply)
        return self

    def create(self, node: Union["NodeData", Node], apply: bool = True) -> "ModuleEditor":
        from bench.language.wire import MNT_BY_DATA_CLASS

        node = pack_node_flat_if_needed(node)
        mnt = MNT_BY_DATA_CLASS[type(node)]
        mmt = MET(f"CREATE_{mnt.caps_name}")
        self._do(mmt, node, apply=apply)
        return self

    def update_many(self, *nodes: Union["NodeData", Node], apply: bool = True) -> "ModuleEditor":
        for obj in nodes:
            self.update(obj, apply=apply)
        return self

    def update(
        self, node: Union["NodeData", Node], apply: bool = True, properties: list[str] = None
    ) -> "ModuleEditor":
        from bench.language.wire import MNT_BY_DATA_CLASS

        assert isinstance(properties, list) or properties is None, f"invalid props: {properties}"

        node = pack_node_flat_if_needed(node)
        mnt = MNT_BY_DATA_CLASS[type(node)]
        mmt = MET(f"UPDATE_{mnt.caps_name}")
        self._do(mmt, node, apply=apply, properties=properties)
        return self

    def delete_many(self, *nodes: Union["NodeData", Node], apply: bool = True) -> "ModuleEditor":
        for obj in nodes:
            self.delete(obj, apply=apply)
        return self

    def delete(self, node: Union["NodeData", Node], apply: bool = True) -> "ModuleEditor":
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

    def __init__(self, edits: list[Edit | EditData]):
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
    def compact(self) -> list[Edit | EditData]:
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
        seen_ops: dict[tuple[MET, UUID], Edit | EditData] = {}

        for edit in reversed(self.edits):
            key = (edit.type, edit.node.id)
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

    def batched(self) -> Iterator[tuple[MET, list[EditData]]]:
        """
        Batch consecutive edits by type in order of appearance.
        """

        current_batch: list[EditData] = []
        current_type: MET | None = None

        for edit in self.edits:
            if edit.type != current_type:
                if current_type is not None:
                    yield current_type, current_batch
                current_type = edit.type
                current_batch = []
            current_batch.append(edit)

        if current_batch:
            yield current_type, current_batch

    def batched_apply(
        self,
        tree: NodeTree,
        project_id: UUID,
        module_id: UUID,
        apply: bool = True,
        raise_on_error: bool = True,
    ) -> Iterator[tuple[MET, list[EditData]]]:
        """
        Batch consecutive edits by type in order of appearance
         AND optionally concurrently apply them to the given module tree.
        (there may be multiple batches of the same type).
        """

        if not apply:
            yield from self.batched()
            return
        editor = ModuleEditor(tree, project_id, module_id)
        for type, batch in self.batched():
            editor.apply_all(batch, raise_on_error=raise_on_error)
            yield type, batch


def diff_modules(
    old_module: "ModuleTreeData", new_module: "ModuleTreeData", project_id: UUID
) -> list[EditData]:
    """
    Get the edits needed to transform old_module into new_module.
    Find nodes by their id (not ck).
    """
    old_tree = NodeTree(old_module.nodes)
    editor = ModuleEditor(old_tree, old_module.id, project_id)
    new_tree = NodeTree(new_module.nodes)

    for new_node in new_tree.walk_bfs():
        if new_node.mnt == ModuleNodeType.MODULE:
            continue  # ignore module itself
        if new_node.id not in old_tree.nodes_by_id:
            editor.create(new_node)
        else:
            old_node = old_tree.nodes_by_id[new_node.id]
            if not new_node.equals_content(old_node):
                editor.update(new_node)
    for old_node in old_tree.walk_bfs():
        if old_node.mnt == ModuleNodeType.MODULE:
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


def render(
    *edits: list[Edit] | EditBundle | list["Node"] | Node,
    target="python",
    record_limit: int = 100,
    recursive: bool = True,
) -> Optional[str]:
    """
    Renders edits or nodes to code in a language.
    Nodes are coerced into create edits with all descendants.
    """
    from bench.language.database import HasDatabase, Record
    from bench.language.statement import Statement

    # coerce to edit bundle
    edits = list(edits)
    if isinstance(edits, Node):
        edits = [edits]
    if isinstance(edits, list):
        if not edits:
            return None
        if isinstance(edits[0], Node):
            nodes: list[Node] = edits
            edits = []
            seen_node_cks: set[UUID] = set()
            for node in nodes:
                tree = node.scope._local_root_tree
                if recursive:
                    descendants = list(node._walk_rec())
                    # add rec  ords to descendants for databases
                    # (this only works when called synchronously)
                    if HasDatabase in node._components:
                        descendants.extend(node.records.limit(record_limit))
                else:
                    descendants = [node]
                # descendants share file id
                if isinstance(node, Statement):
                    file = node.file
                    statement = node
                elif isinstance(node, Record):
                    # Records are not in the inline tree, see :NodeViews
                    file = node.parent.file
                    statement = node.parent
                else:
                    file = None
                    statement = None
                for n in descendants:
                    if n.ck in seen_node_cks or n.mnt in INTERP_NODE_TYPES:
                        continue
                    seen_node_cks.add(n.ck)
                    edit = Edit(
                        type=EditType(f"CREATE_{n.mnt.caps_name}"),
                        module=n.module,
                        node=n,
                        file=file or tree.get_ancestor(n.ck, MNT.FILE),
                        statement=statement or tree.get_ancestor(n.ck, MNT.STATEMENT),
                    )
                    edits.append(edit)
        edits = EditBundle(edits)
    if not isinstance(edits, EditBundle):
        raise ValueError(f"cannot render {edits!r}")

    # render
    if target == "python":
        return render_as_python(edits)
    else:
        raise ValueError(f"cannot render to {target}")


def _render_prop(node: Node, name: str, value: Any) -> str:
    """
    Render a non-relational prop (may be a reference, but not a parent/child relation).
    TODO @Broken: _render_prop recursively (see typing)
    """
    from bench.language.packer import render_value
    from bench.language.text import HasText
    from bench.language.value import HasValue

    if value is None:
        return "None"
    elif isinstance(value, UUID):
        return f'UUID("{value}")'
    elif isinstance(value, (enum.StrEnum, enum.IntEnum)):
        return f"{type(value).__name__}.{value.name}"
    elif isinstance(value, (enum.IntFlag,)):
        # reconstitute flags as a | b | c
        return " | ".join(f"{type(value).__name__}.{v.name}" for v in type(value) if value & v)
    elif isinstance(value, Node):
        return f"'{value.name}'"  # this isn't quite right, may be shadowed/scoped
    elif isinstance(value, (int, float, bool)):
        return repr(value)
    # TODO @Broken: render & parse in-value references properly (e.g. secret, file, node)
    #   Related: figure out good way to set/'coerce' secrets, files, etc. as values
    elif isinstance(value, (str, Text)):
        # render text into simple form
        if isinstance(value, Text):
            value = render_text_simple(value.spans)
        elif name == "text" and HasText in node._components and value and node._text_spans:
            value = render_text_simple(node._text_spans)
        # if it contains newlines transform into multiline string
        # and escape any multiline strings inside
        if "\n" in value:
            value = value.replace('"""', '\\"\\"\\"')
            return f'"""\\\n{value}"""'
        else:
            value = value.replace('"', '\\"')
            return repr(value)
    elif name == "value" and HasValue in node._components and isinstance(value, Mapping):
        value = render_value(
            value,
            node._type_of_value,
            get_k=lambda f: f.py_ident,
            filter_v=lambda v, f: f.tag != TypeTag.FILE and not (f.flags & TypeFlag.IS_SECRET),
            ignore_array=True,
        )
        return omit_empty(value)
    elif hasattr(type(value), "to_python"):
        return type(value).to_python(value)
    else:
        raise ValueError(f"cannot render {value!r} (for {node!r}->{name})")


def _sep(*strs) -> str:
    strs = list(strs)
    if strs and isinstance(strs[0], Generator):
        strs = list(strs[0])
    if strs and isinstance(strs[0], (list, tuple)):
        strs = list(strs[0])
    return ", ".join(str(s) for s in strs if s)


class _NodeInit(NamedTuple):
    node: Node
    name: str
    args: dict
    kwargs: dict


class _OpType(enum.StrEnum):
    ASSIGN = "="
    CREATE = "create"
    APPEND = "append"


class _Op(NamedTuple):
    target: str
    op: _OpType
    nodes: list[_NodeInit]


def render_as_python(edits: EditBundle) -> Optional[str]:
    """
    Generate minimal(ish) Python code that produces the given edits.
    The returned order matches the given order of edits, i.e. no dependencies are considered.
    (This should be fine since edits for node subtrees are produced top-down.)
    """
    if not edits:
        return None

    # index nodes to find roots
    nodes_by_ck: dict[UUID, Node] = {}
    for edit in edits.edits:
        assert isinstance(edit, Edit), f"cannot render data {edit!r}"
        nodes_by_ck[edit.node.ck] = edit.node

    # render single edits into 'lines' (target, op, node)
    ops: list[_Op] = []
    for edit in edits.edits:
        edit: Edit
        node = edit.node
        if edit.kind == EditKind.CREATE:
            # get props to create
            init_props = {
                prop.name: getattr(node, prop.name)
                for prop in node.__properties__.values()
                if not prop.is_runtime
                and not prop.is_relation
                and not prop.is_cru
                and prop.name not in ("id", "ck", "parent", "order_key", "key")
                and getattr(node, prop.name, UNSET) is not prop.default
            }

            # simplify props
            if hasattr(type(node), "to_python"):
                init_name, init_args, init_kwargs = type(node).to_python(
                    node, init_props, node.parent
                )
                init_node = _NodeInit(node, init_name, init_args, init_kwargs)
            else:
                init_node = _NodeInit(node, type(node).__name__, {}, init_props)
            del init_props

            # render as define (root) or create/append (child)
            if node.parent and node.parent.ck in nodes_by_ck:
                attach_to_prop = first(
                    p
                    for p in node.parent.__list_properties_by_child__[node.mnt]
                    if not p.children_flags & NRel.Flat
                )
                parent_str = f"{node.parent.py_ident}.{attach_to_prop.name}"
                if node.mnt in (MNT.RECORD, MNT.TAGGING, MNT.TRIGGER):
                    op = _Op(parent_str, _OpType.CREATE, [init_node])
                else:
                    op = _Op(parent_str, _OpType.APPEND, [init_node])
            else:
                op = _Op(node.py_ident, _OpType.ASSIGN, [init_node])
        else:
            raise ValueError(f"cannot render {edit!r}")
        ops.append(op)

    # merge successive ops (if they can be combined like create/append)
    merged: list[_Op] = []
    for op in ops:
        if merged and merged[-1].target == op.target and merged[-1].op == op.op:
            merged[-1].nodes.extend(op.nodes)
        else:
            merged.append(op)

    # render 'ops' into code
    lines = []
    for target, op, nodes in merged:
        if op == "=":
            n, init_name, init_args, init_kwargs = nodes[0]
            init_kwargs = {**init_args, **init_kwargs}
            kwargs_str = _sep(f"{k}={_render_prop(n, k, v)}" for k, v in init_kwargs.items() if v)
            if target:
                lines.append(f"{target} = {init_name}({kwargs_str})")
            else:  # isn't this an error case?
                lines.append(f"{init_name}({kwargs_str})")
            continue

        # stringify each node
        nodes_strs = []
        for node in nodes:
            n, init_name, init_args, init_kwargs = node
            # inline record value (see Record.new)
            if n.mnt == MNT.RECORD:
                from bench.language.packer import render_value

                kwargs_str = _sep(
                    f"{k}={render_value(v, n._type_of_value.resolved_fields.get(k))}"
                    for k, v in n.value.items()
                    if v
                )
                nodes_strs.append(f"Record.new({_sep(kwargs_str)})")
                continue

            if op == _OpType.CREATE and len(nodes) > 1:
                # merge args into kwargs
                init_kwargs = {**init_args, **init_kwargs}
                init_args.clear()

            # render args strs in reverse as soon as a value is set
            args_strs = []
            for k, v in reversed(init_args.items()):
                if v or args_strs:
                    args_strs.append(_render_prop(n, k, v))
            args_str = _sep(*reversed(args_strs))
            kwargs_str = _sep(f"{k}={_render_prop(n, k, v)}" for k, v in init_kwargs.items() if v)
            if op == _OpType.CREATE and len(nodes) == 1:
                nodes_strs.append(f"{_sep(args_str, kwargs_str)}")
            else:
                nodes_strs.append(f"{init_name}({_sep(args_str, kwargs_str)})")

        # join them into merged line
        nodes_str = _sep(nodes_strs)
        if op == _OpType.CREATE and len(nodes) == 1:
            lines.append(f"{target}.create({nodes_str})")
        else:
            if len(nodes) == 1:
                lines.append(f"{target}.append({nodes_str})")
            else:
                lines.append(f"{target}.extend({nodes_str})")

    # format with black
    code = "\n".join(lines)
    try:
        import black

        code = black.format_str(code, mode=black.Mode(line_length=100))
    except Exception as e:
        raise ValueError(f"rendered bad code:\n{code}") from e

    return code
