"""
Structs for syncing project model contents across servers and clients.
It shouldn't live in models, so we can use it in messages.py, which shouldn't depend on Django.
Maybe a better move would be to make the payload partially opaque and keep this in api.
"""
import enum
from dataclasses import dataclass, replace
from datetime import datetime
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

from bench.language.const import (
    INTERP_NODE_TYPES,
    OUT_OF_LINE_NODE_TYPES,
    NodeType,
    TypeFlag,
    TypeTag,
)
from bench.language.module import UNSET, Module, Node, NodeTree, NRel
from bench.language.text import Text, render_text_simple
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import to_uuid
from bench.utils.serialize import from_dict
from bench.utils.utils import format_python, omit_empty

if TYPE_CHECKING:
    from bench.language import File, Run, Statement
    from bench.language.wire import ModuleTreeData
    from bench.language.wiring import AnyNodeData


class EditType(enum.StrEnum):
    """Fine-grained atomic edits for multiplayer modules."""

    # Files
    BUMP_FILE = "BUMP_FILE"
    CREATE_FILE = "CREATE_FILE"
    UPDATE_FILE = "UPDATE_FILE"
    DELETE_FILE = "DELETE_FILE"
    # Files (API)
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
    # Session
    CREATE_SESSION = "CREATE_SESSION"
    UPDATE_SESSION = "UPDATE_SESSION"
    CREATE_RUN = "CREATE_RUN"
    UPDATE_RUN = "UPDATE_RUN"

    @property
    def kind(self) -> "EditKind":
        return _MODULE_EDIT_MAP[self][0]

    @property
    def node_type(self) -> "NodeType":
        return _MODULE_EDIT_MAP[self][1]

    @staticmethod
    def from_nt(mmk: "EditKind", nt: "NodeType") -> "EditType":
        return EditType(f"{mmk.value}_{nt.caps_name}")


class EditKind(enum.StrEnum):
    CREATE = "CREATE"
    UPDATE = "UPDATE"
    DELETE = "DELETE"
    MOVE = "MOVE"
    SOFT_DELETE = "SOFT_DELETE"
    RESTORE = "RESTORE"
    TRUNCATE = "TRUNCATE"
    BUMP = "BUMP"


_MODULE_EDIT_MAP: dict[EditType, tuple[EditKind, NodeType]] = {
    # Files
    EditType.BUMP_FILE: (EditKind.BUMP, NodeType.FILE),
    EditType.CREATE_FILE: (EditKind.CREATE, NodeType.FILE),
    EditType.SOFT_DELETE_FILE: (EditKind.SOFT_DELETE, NodeType.FILE),
    EditType.RESTORE_FILE: (EditKind.RESTORE, NodeType.FILE),
    EditType.RENAME_FILE: (EditKind.UPDATE, NodeType.FILE),
    EditType.MOVE_FILE: (EditKind.MOVE, NodeType.FILE),
    EditType.UPDATE_FILE: (EditKind.UPDATE, NodeType.FILE),
    EditType.DELETE_FILE: (EditKind.DELETE, NodeType.FILE),
    # Statements
    EditType.BUMP_STATEMENT: (EditKind.BUMP, NodeType.STATEMENT),
    EditType.PASTE_STATEMENT: (EditKind.CREATE, NodeType.STATEMENT),
    EditType.CREATE_STATEMENT: (EditKind.CREATE, NodeType.STATEMENT),
    EditType.SOFT_DELETE_STATEMENT: (EditKind.SOFT_DELETE, NodeType.STATEMENT),
    EditType.RESTORE_STATEMENT: (EditKind.RESTORE, NodeType.STATEMENT),
    EditType.MORPH_STATEMENT: (EditKind.UPDATE, NodeType.STATEMENT),
    EditType.MOVE_STATEMENT: (EditKind.MOVE, NodeType.STATEMENT),
    EditType.RENAME_STATEMENT: (EditKind.UPDATE, NodeType.STATEMENT),
    EditType.UPDATE_STATEMENT: (EditKind.UPDATE, NodeType.STATEMENT),
    EditType.DELETE_STATEMENT: (EditKind.DELETE, NodeType.STATEMENT),
    EditType.UPDATE_STATEMENT_TEXT: (EditKind.UPDATE, NodeType.STATEMENT),
    EditType.UPDATE_STATEMENT_FLAGS: (EditKind.UPDATE, NodeType.STATEMENT),
    EditType.UPDATE_STATEMENT_HEADING_LEVEL: (EditKind.UPDATE, NodeType.STATEMENT),
    EditType.UPDATE_STATEMENT_REFERENCE: (EditKind.UPDATE, NodeType.STATEMENT),
    EditType.UPDATE_SYMBOL_text: (EditKind.UPDATE, NodeType.STATEMENT),
    EditType.UPDATE_SYMBOL_CODE: (EditKind.UPDATE, NodeType.STATEMENT),
    EditType.UPDATE_SYMBOL_MODIFIER: (EditKind.UPDATE, NodeType.STATEMENT),
    EditType.UPDATE_SYMBOL_LANGUAGE: (EditKind.UPDATE, NodeType.STATEMENT),
    EditType.UPDATE_SYMBOL_VALUE: (EditKind.UPDATE, NodeType.STATEMENT),
    # Taggings
    EditType.CREATE_TAGGING: (EditKind.CREATE, NodeType.TAGGING),
    EditType.UPDATE_TAGGING: (EditKind.UPDATE, NodeType.TAGGING),
    EditType.DELETE_TAGGING: (EditKind.DELETE, NodeType.TAGGING),
    EditType.SOFT_DELETE_TAGGING: (EditKind.SOFT_DELETE, NodeType.TAGGING),
    EditType.RESTORE_TAGGING: (EditKind.RESTORE, NodeType.TAGGING),
    EditType.MOVE_TAGGING: (EditKind.UPDATE, NodeType.TAGGING),
    # Triggers
    EditType.CREATE_TRIGGER: (EditKind.CREATE, NodeType.TRIGGER),
    EditType.UPDATE_TRIGGER: (EditKind.UPDATE, NodeType.TRIGGER),
    EditType.DELETE_TRIGGER: (EditKind.DELETE, NodeType.TRIGGER),
    EditType.SOFT_DELETE_TRIGGER: (EditKind.SOFT_DELETE, NodeType.TRIGGER),
    EditType.RESTORE_TRIGGER: (EditKind.RESTORE, NodeType.TRIGGER),
    # Fields
    EditType.CREATE_FIELD: (EditKind.CREATE, NodeType.FIELD),
    EditType.UPDATE_FIELD: (EditKind.UPDATE, NodeType.FIELD),
    EditType.RENAME_FIELD: (EditKind.UPDATE, NodeType.FIELD),
    EditType.UPDATE_FIELD_TEXT: (EditKind.UPDATE, NodeType.FIELD),
    EditType.UPDATE_FIELD_TYPE: (EditKind.UPDATE, NodeType.FIELD),
    EditType.MOVE_FIELD: (EditKind.MOVE, NodeType.FIELD),
    EditType.DELETE_FIELD: (EditKind.DELETE, NodeType.FIELD),
    EditType.SOFT_DELETE_FIELD: (EditKind.SOFT_DELETE, NodeType.FIELD),
    EditType.RESTORE_FIELD: (EditKind.RESTORE, NodeType.FIELD),
    # Records
    EditType.TRUNCATE_RECORDS: (EditKind.TRUNCATE, NodeType.RECORD),
    EditType.CREATE_RECORD: (EditKind.CREATE, NodeType.RECORD),
    EditType.UPDATE_RECORD: (EditKind.UPDATE, NodeType.RECORD),
    EditType.DELETE_RECORD: (EditKind.DELETE, NodeType.RECORD),
    EditType.SOFT_DELETE_RECORD: (EditKind.SOFT_DELETE, NodeType.RECORD),
    EditType.RESTORE_RECORD: (EditKind.RESTORE, NodeType.RECORD),
    # Interp
    EditType.TRUNCATE_ISSUES: (EditKind.TRUNCATE, NodeType.ISSUE),
    EditType.CREATE_ISSUE: (EditKind.CREATE, NodeType.ISSUE),
    EditType.DELETE_ISSUE: (EditKind.DELETE, NodeType.ISSUE),
    EditType.TRUNCATE_RESOLVED_FIELDS: (EditKind.TRUNCATE, NodeType.RESOLVED_FIELD),
    EditType.CREATE_RESOLVED_FIELD: (EditKind.CREATE, NodeType.RESOLVED_FIELD),
    EditType.DELETE_RESOLVED_FIELD: (EditKind.DELETE, NodeType.RESOLVED_FIELD),
    # Sessions
    EditType.CREATE_SESSION: (EditKind.CREATE, NodeType.SESSION),
    EditType.UPDATE_SESSION: (EditKind.UPDATE, NodeType.SESSION),
    EditType.CREATE_RUN: (EditKind.CREATE, NodeType.RUN),
    EditType.UPDATE_RUN: (EditKind.UPDATE, NodeType.RUN),
}

# assert that all edits are in the map
assert set(EditType) == set(_MODULE_EDIT_MAP.keys()), "not all edits are mapped"


@dataclass
class Edit:
    """
    An edit to a module/node.
    TODO @Cleanup @Architecture: use new :Edit where possible (see :BE-114)
     also track Edit.edited_by (for Run to enable undo)
    """

    type: EditType
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
    def scope(self) -> NodeType:
        """The type of node that was edited. Usually the same as node_type except for truncate."""
        return self.node.metatype

    @property
    def node_type(self) -> NodeType:
        """The type of node that was edited."""
        return self.type.node_type


@dataclass
class EditData:
    type: EditType
    project_version_id: UUID
    file_id: Optional[UUID] = None
    statement_id: Optional[UUID] = None
    revision: Optional[int] = None
    input: Optional[dict[str, Any]] = None  # for GQL edits
    properties: Optional[list[str]] = None  # changed properties (by language name), see :Edit
    thing: Optional[Any] = None  # in-memory object that was mutated, not serialized
    _node_type: Optional[NodeType] = None  # discriminator for 'union'
    _node: Optional[
        AnyNodeData if TYPE_CHECKING else Any
    ] = None  # the actual data, custom encode/decoded as union

    def encode_some_attrs(self):  # see serialize and :WireFormat
        # no special encoding of data here
        return {"thing": None}  # always omit thing

    @classmethod
    def decode_some_attrs(cls, data: dict[str, Any]) -> dict[str, Any]:
        from bench.language import wiring

        _node = data.get("_node")
        if _node is not None:
            _data_cls = wiring.PROTO_CLASS_BY_TYPE[data["_node_type"]]
            _node = from_dict(_data_cls, _node)
        return {"_node": _node}

    @property
    def node(self) -> Optional["AnyNodeData"]:
        return self._node

    @node.setter
    def node(self, node: "AnyNodeData"):
        self._node_type = NodeType(node.metatype.name)
        self._node = node

    @property
    def kind(self) -> EditKind:
        return self.type.kind

    @property
    def scope(self) -> NodeType:
        assert self._node_type is not None, f"node_type is not set on {self!r}"
        return self._node_type

    @property
    def node_type(self) -> NodeType:
        return self.type.node_type

    def to_kind(self, kind: EditKind) -> "EditData":
        new_type = EditType.from_nt(kind, self.node_type)
        return replace(self, type=new_type)

    def __str__(self):
        data_str = f"{self.node.metatype} {self.node.id} " if self.node else ""
        properties_str = (" [" + ", ".join(self.properties) + "]") if self.properties else ""
        return f"{self.type} {data_str}{self.revision}{properties_str}"

    def __repr__(self):
        return f"<Edit {self}>"


class NodeTreeEditor:
    """Create edits to a module node tree."""

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

    def _make_edit(
        self, type: EditType, node: "AnyNodeData", properties: list[str] = None
    ) -> "EditData":
        from bench.language import wire

        if isinstance(node, wire.StatementData):
            statement_id = node.id
            file_id = self.file_id or self.tree.ancestor(to_uuid(node.parent_id), NodeType.FILE).id
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
                statement = self.tree.get_ancestor(to_uuid(node.parent_id), NodeType.STATEMENT)
                statement_id = statement.id if statement else None
            if self.file_id:
                file_id = self.file_id
            else:
                file_id = self.tree.ancestor(to_uuid(node.parent_id), NodeType.FILE).id
        edit = EditData(
            type=type,
            project_version_id=self.module_id,
            revision=getattr(node, "revision", None),
            file_id=file_id,
            statement_id=statement_id,
            properties=properties,
        )
        edit.node = node
        self.edits.append(edit)
        return edit

    def _pack_node_flat_if_needed(self, node: Union[Node, "AnyNodeData"]) -> "AnyNodeData":
        from bench.language import wiring

        if isinstance(node, Node):
            return wiring.pack_node(node)
        else:
            return replace(node)  # shallow copy

    def truncate(self, node: Union["AnyNodeData", Node], node_type: NodeType) -> "EditData":
        return self._make_edit(
            EditType(f"TRUNCATE_{node_type.caps_name}S"), node=self._pack_node_flat_if_needed(node)
        )

    def create_many(self, *nodes: Union["AnyNodeData", Node]) -> list["EditData"]:
        return [self.create(node) for node in nodes]

    def create(self, node: Union["AnyNodeData", Node]) -> "EditData":
        return self._make_edit(
            EditType.from_nt(EditKind.CREATE, node.metatype),
            node=self._pack_node_flat_if_needed(node),
        )

    def update(self, node: Union["AnyNodeData", Node], properties: list[str] = None) -> "EditData":
        assert isinstance(properties, list) or properties is None, f"invalid props: {properties}"
        return self._make_edit(
            type=EditType.from_nt(EditKind.UPDATE, node.metatype),
            node=self._pack_node_flat_if_needed(node),
            properties=properties,
        )

    def move(self, node: Union["AnyNodeData", Node]) -> "EditData":
        return self._make_edit(
            type=EditType.from_nt(EditKind.MOVE, node.metatype),
            node=self._pack_node_flat_if_needed(node),
        )

    def soft_delete_many(
        self, *nodes: Union["AnyNodeData", Node], deleted_at: datetime | None = None
    ) -> list["EditData"]:
        return [self.soft_delete(node, deleted_at) for node in nodes]

    def soft_delete(
        self, node: Union["AnyNodeData", Node], deleted_at: datetime | None = None
    ) -> "EditData":
        # sneakily convert soft delete into hard delete for interp types
        if node.metatype in INTERP_NODE_TYPES:
            return self.delete(node)
        node = self._pack_node_flat_if_needed(node)
        node.deleted_at = deleted_at or utcnow_with_tz()
        return self._make_edit(
            type=EditType.from_nt(EditKind.SOFT_DELETE, node.metatype), node=node
        )

    def restore(self, node: Union["AnyNodeData", Node]) -> "EditData":
        node = self._pack_node_flat_if_needed(node)
        node.deleted_at = None
        return self._make_edit(type=EditType.from_nt(EditKind.RESTORE, node.metatype), node=node)

    def delete(self, node: Union["AnyNodeData", Node]) -> "EditData":
        return self._make_edit(
            type=EditType.from_nt(EditKind.DELETE, node.metatype),
            node=self._pack_node_flat_if_needed(node),
        )


class EditBundle:
    """Indexed access to a constant list of edits."""

    def __init__(self, edits: list[Edit | EditData]):
        self.edits = edits

    def __str__(self):
        return f"edit {len(self.edits)}"

    def __repr__(self):
        return f"<EditBundle {self}>"

    def batched(self) -> Iterator[tuple[EditType, list[EditData]]]:
        """
        Batch consecutive edits by type in order of appearance.
        """

        current_batch: list[EditData] = []
        current_type: EditType | None = None

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
        self, tree: NodeTree, raise_on_error: bool = True
    ) -> Iterator[tuple[EditType, list[EditData]]]:
        """
        Batch consecutive edits by type in order of appearance
         AND optionally concurrently apply them to the given module tree.
        (there may be multiple batches of the same type).
        """

        for type, batch in self.batched():
            yield type, batch
            if type.node_type in OUT_OF_LINE_NODE_TYPES:
                continue  # ignore since it's not in the inline tree
            for edit in batch:
                try:
                    tree.apply_edit(edit)
                except ValueError:
                    if raise_on_error:
                        raise


def diff_modules(
    old_module: "ModuleTreeData", new_module: "ModuleTreeData", project_id: UUID
) -> list[EditData]:
    """
    Get the edits needed to transform old_module into new_module.
    Find nodes by their id (not ck).
    """
    old_tree = NodeTree(old_module.nodes)
    editor = NodeTreeEditor(old_tree, old_module.id, project_id)
    new_tree = NodeTree(new_module.nodes)

    for new_node in new_tree.walk_bfs():
        if new_node.metatype == NodeType.MODULE:
            continue  # ignore module itself
        if new_node.id not in old_tree.nodes_by_id:
            old_tree.apply_edit(editor.create(new_node))
        else:
            old_node = old_tree.nodes_by_id[new_node.id]
            if not new_node.equals_content(old_node):
                old_tree.apply_edit(editor.update(new_node))
    for old_node in old_tree.walk_bfs():
        if old_node.metatype == NodeType.MODULE:
            continue
        if old_node.id not in new_tree.nodes_by_id:
            old_tree.apply_edit(editor.delete(old_node))
    # sort into delete -> create -> update
    edits = [
        *(e for e in editor.edits if e.type.kind == EditKind.DELETE),
        *(e for e in editor.edits if e.type.kind == EditKind.CREATE),
        *(e for e in editor.edits if e.type.kind == EditKind.UPDATE),
    ]
    return edits


def render(
    *edits: list[Edit] | EditBundle | list["Node"] | Node,
    target="python",
    record_limit: int = 20,
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
                        descendants.extend(node.records.first(record_limit))
                else:
                    descendants = [node]
                # descendants share file id
                if isinstance(node, Statement):
                    file = node.file
                    statement = node
                elif isinstance(node, Record):
                    file = node.parent.file
                    statement = node.parent
                else:
                    file = None
                    statement = None
                for n in descendants:
                    if n.ck in seen_node_cks or n.metatype in INTERP_NODE_TYPES:
                        continue
                    seen_node_cks.add(n.ck)
                    edit = Edit(
                        type=EditType(f"CREATE_{n.metatype.caps_name}"),
                        module=n.module,
                        node=n,
                        file=file or tree.get_ancestor(n.ck, NodeType.FILE),
                        statement=statement or tree.get_ancestor(n.ck, NodeType.STATEMENT),
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


def filter_field_exclude_blob_and_secret(v, f):
    return f.tag != TypeTag.BLOB and not f.flags & TypeFlag.IS_SECRET


# ignore files and secrets (can't render them properly.. yet?)
DEFAULT_VALUE_FILTER = filter_field_exclude_blob_and_secret


def _render_prop(node: Node, name: str, value: Any) -> str:
    """
    Render a non-relational prop (may be a reference, but not a parent/child relation).
    TODO @Broken: _render_prop recursively with all nodes/structs (blobs, secrets, etc. see typing)
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
            node.metatype_of_value,
            get_k=lambda f: f.py_ident,
            filter_v=DEFAULT_VALUE_FILTER,
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
                and not prop.is_tree_relation
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
                    for p in node.parent.__list_properties_by_child__[node.metatype]
                    if not p.children_flags & NRel.Flat
                )
                parent_str = f"{node.parent.py_ident}.{attach_to_prop.name}"
                if node.metatype in (NodeType.RECORD, NodeType.TAGGING, NodeType.TRIGGER):
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
            if n.metatype == NodeType.RECORD:
                from bench.language.packer import render_value

                kwargs_str = _sep(
                    f"{k}={render_value(v, n.metatype_of_value.resolved_fields.get(k), filter_v=DEFAULT_VALUE_FILTER)}"
                    for k, v in n.value.items()
                    if v and DEFAULT_VALUE_FILTER(v, n.metatype_of_value.resolved_fields.get(k))
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
    code = format_python(code)
    return code
