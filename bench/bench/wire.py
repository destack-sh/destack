from __future__ import annotations

import abc
import re
import traceback
import typing
from collections import OrderedDict, defaultdict, deque
from dataclasses import dataclass, replace
from datetime import datetime
from typing import Any, ClassVar, Optional
from uuid import UUID

from bench import bench as lang
from bench.bench import Code, IssueType, StatementType, TypeHint, TypeTag
from bench.bench.const import ExecutionTriggerType, ExpectationModifier, InterpScope, TypeFlag, MOT
from bench.bench.dataset import Query, Sort
from bench.bench.issue import IssueKind
from bench.bench.type import ModuleNode, ModuleReference
from bench.runtime.common.type import ExecutionFrame, PyFrameData, RunErrorData
from bench.utils.func import describe_type

if typing.TYPE_CHECKING:
    from bench.bench.build import XBlock
    from bench.bench.session import Session


#
# Stable, concise and flat language data nodes for transit and storage.
# TODO @Performance @Robustness: use an optimized and evolvable :WireFormat
#


ParentsT = set[MOT]
NodeDataT = typing.TypeVar("NodeDataT", bound="NodeData")
NodeT = typing.TypeVar("NodeT", bound="ModuleNode")
DataT = typing.TypeVar("DataT")
ObjectT = typing.TypeVar("ObjectT")


class ModuleTree:
    """An indexed tree of module nodes"""

    def __init__(self, nodes: list[NodeT | NodeDataT] = None):
        self.nodes: dict[UUID, NodeT] = {}
        self.children: dict[UUID, list[UUID]] = defaultdict(list)
        for node in nodes or []:
            self.add(node)

    def __str__(self):
        return f"{len(self.nodes)} nodes"

    def __repr__(self):
        return f"<ModuleTree {self}>"

    def __contains__(self, item):
        return item in self.nodes

    def __getitem__(self, item):
        return self.nodes[item]

    def add(self, node: NodeT | NodeDataT):
        """Add a node to the tree (error if node already exists)"""
        if node.id in self.nodes:
            raise ValueError(f"node with id {node.id} already exists in {self}")
        self.nodes[node.id] = node
        if node.parent_id is not None:
            self.children[node.parent_id].append(node.id)

    def replace(self, node: NodeT | NodeDataT):
        """Upsert a node in the tree (replace if node already exists)"""
        old_node = self.nodes.get(node.id)
        if old_node is not None and old_node.parent_id is not None:
            self.children[old_node.parent_id].remove(node.id)
        self.nodes[node.id] = node
        self.children[node.parent_id].append(node.id)

    def remove(self, node: NodeT | NodeDataT, recursive: bool = True):
        """Remove a node from the tree (incl. all descendants if recursive)"""
        if recursive:
            descendants = self.get_descendants(node.id, recursive=True)
            for descendant in descendants:
                self.nodes.pop(descendant.id)
                self.children.pop(descendant.id)
        self.nodes.pop(node.id)
        self.children.pop(node.id)

    def truncate(
        self, node: NodeT | NodeDataT, t: NodeT | NodeDataT | None = None, recursive: bool = True
    ):
        """Truncate descendants of a node"""
        descendants = self.get_descendants(node.id, t, recursive=recursive)
        for descendant in descendants:
            self.children.pop(descendant.id)
            self.nodes.pop(descendant.id)

    def prune(self, t: NodeT | NodeDataT):
        """Prune all nodes of the given type"""
        for node in list(self.nodes.values()):
            if isinstance(node, t):
                self.remove(node, recursive=True)

    @property
    def root(self) -> Optional[NodeT | NodeDataT]:
        roots = [node for node in self.nodes.values() if node.parent_id is None]
        if len(roots) > 1:
            raise ValueError(f"expected 0 or 1 root nodes, got {roots}")
        return roots[0] if roots else None

    def walk_bfs(
        self, root: typing.Union[NodeT, None] = None
    ) -> typing.Generator[NodeT | NodeDataT, None, None]:
        """Walks the tree in breadth-first order"""
        root = root or self.root
        if root is None:
            raise ValueError(f"cannot walk tree with no root node")

        queue = deque([root])
        while queue:
            current_node = queue.popleft()
            yield current_node
            for child_id in self.children[current_node.id]:
                queue.append(self.nodes[child_id])

    def get_child(
        self, parent_id: UUID, t: NodeT | NodeDataT | None = None
    ) -> Optional["NodeT | NodeDataT"]:
        """Finds one or zero children of the given type"""
        children = self.get_descendants(parent_id, t)
        if len(children) > 1:
            raise ValueError(
                f"expected 0 or 1 children of type {t} for parent {parent_id}, got {children}"
            )
        return children[0] if children else None

    def get_descendants(
        self, parent_id: UUID, t: NodeT | NodeDataT | None = None, recursive: bool = False
    ) -> list["NodeT | NodeDataT"]:
        """Finds all children (or descendants) of the given type"""
        children = [
            self.nodes[child_id]
            for child_id in self.children[parent_id]
            if t is None or isinstance(self.nodes[child_id], t)
        ]
        if recursive:
            for child in children:
                children.extend(self.get_descendants(child.id, t, recursive=True))
        return children

    def get_ancestor(
        self, node_id: UUID, t: NodeT | NodeDataT | None = None
    ) -> Optional["NodeT | NodeDataT"]:
        """Finds the next ancestor of the given type"""
        node = self.nodes.get(node_id)
        if node is None:
            raise ValueError(f"node {node_id} is not in {self}")
        while node:
            if t is None or isinstance(node, t):
                return node
            if node.parent_id is None:
                return None
            node = self.nodes[node.parent_id]
        return None

    def get_ancestors(
        self, node_id: UUID, t: NodeT | NodeDataT | None = None, include_self: bool = False
    ) -> list["NodeT | NodeDataT"]:
        """Finds all ancestors of the given type"""
        ancestors = []
        node = self.nodes.get(node_id)
        if node is None:
            raise ValueError(f"node {node_id} is not in {self}")
        if include_self:
            ancestors.append(node)
        while node:
            if t is None or isinstance(node, t):
                ancestors.append(node)
            if node.parent_id is None:
                break
            node = self.nodes[node.parent_id]
        return ancestors


class NodePacker(abc.ABC, typing.Generic[NodeDataT, NodeT]):
    """Module node packer"""

    def walk(self, node: NodeT, tree: PackContext) -> None:
        """Walks all direct children of the given node"""
        pass

    def pack(self, node: NodeT) -> NodeDataT:
        """Packs the node itself into the wire format"""
        raise NotImplementedError

    def unpack(self, node: NodeDataT, parent: Optional[NodeT]) -> NodeT:
        """Unpacks the node itself from the wire format"""
        raise NotImplementedError

    def unwalk(self, node: NodeT, tree: ModuleTree) -> None:
        """Re-assigns the node's children"""
        pass


class PackContext(abc.ABC):
    """Tree visitor for packing"""

    def __init__(self):
        self.visited: dict[UUID, ModuleNode] = {}

    def visit(self, node: ModuleNode):
        self.visited[node.id] = node


# registered packers
_node_packers_by_data: dict[typing.Type[NodeDataT], NodePacker] = {}
_node_packers_by_node: dict[typing.Type[NodeT], NodePacker] = {}
MOT_BY_DATA_CLASS: dict[typing.Type[NodeDataT], MOT] = {}
BASE_DATA_CLASS_BY_MOT: dict[MOT, typing.Type[NodeDataT]] = {}


def node_packer(t: MOT, data_t: typing.Type[NodeDataT], node_t: typing.Type[NodeT] | None):
    """Decorator to register a node packer for a given type"""

    def decorator(cls: "NodePacker"):
        if node_t in _node_packers_by_node:
            raise ValueError(
                f"packer for {node_t} already registered: {_node_packers_by_node[node_t]}"
            )
        if data_t in _node_packers_by_data:
            raise ValueError(
                f"packer for {data_t} already registered: {_node_packers_by_data[data_t]}"
            )
        packer = cls()
        _node_packers_by_node[node_t] = packer
        _node_packers_by_data[data_t] = packer
        MOT_BY_DATA_CLASS[data_t] = t
        if t not in BASE_DATA_CLASS_BY_MOT:
            BASE_DATA_CLASS_BY_MOT[t] = data_t
        elif not issubclass(data_t, BASE_DATA_CLASS_BY_MOT[t]):
            raise ValueError(
                f"cannot register {data_t} as {t}, it is not a subclass of {BASE_DATA_CLASS_BY_MOT[t]}"
            )
        return cls

    return decorator


def pack_module(module: lang.Module) -> "ModuleData":
    module_data, nodes = pack_node(module)
    module_data.nodes = nodes
    return module_data


def unpack_module(module: ModuleData) -> lang.Module:
    module = unpack_node(module.nodes, parent=None)
    return module


def pack_node(root: NodeT) -> tuple[NodeDataT, list[NodeDataT]]:
    """Pack a node and all its descendants"""
    packed: dict[UUID, NodeDataT] = OrderedDict()
    ctx = PackContext()

    # walk and pack until nothing is left to pack
    to_pack: list[NodeT] = [root]
    while to_pack:
        for node in to_pack:
            packer = _node_packers_by_node[type(node)]
            packer.walk(node, ctx)
            packed[node.id] = packer.pack(node)
        to_pack = [node for node in ctx.visited.values() if node.id not in packed]

    return packed[root.id], list(packed.values())


def unpack_node(nodes: list[NodeDataT], parent: Optional[NodeT]) -> NodeT:
    """Unpack a node and all its descendants"""
    data_tree = ModuleTree(nodes)
    unpacked_tree = ModuleTree()

    # unpack all nodes top down (breadth first)
    for node in data_tree.walk_bfs():
        packer = _node_packers_by_data[type(node)]
        node_parent = unpacked_tree.nodes[node.parent_id] if node.parent_id else parent
        unpacked_tree.add(packer.unpack(node, node_parent))

    # 'unwalk' all nodes to re-assign descendants
    for node in unpacked_tree.nodes.values():
        packer = _node_packers_by_node[type(node)]
        packer.unwalk(node, unpacked_tree)

    return unpacked_tree.root


def pack_node_flat(node: NodeT) -> NodeDataT:
    """Pack a language node into a flat module node"""
    packer = _node_packers_by_node[type(node)]
    return packer.pack(node)


def unpack_node_flat(node: NodeDataT, parent: Optional[NodeT]) -> NodeT:
    """Unpack a flat module node into a language node"""
    packer = _node_packers_by_data[type(node)]
    return packer.unpack(node, parent)


@dataclass
class NodeData:
    id: UUID
    parent_id: Optional[UUID]


@dataclass
class Ordered:
    order_key: str


@dataclass
class Revisioned:
    revision: int


@dataclass
class ModuleData(NodeData):
    name: str
    committed: bool
    parent_id: Optional[UUID]
    nodes: Optional[list[NodeData]] = None

    def strip(self) -> ModuleData:
        return replace(self, nodes=None)

    def __str__(self):
        return f"{self.name}@{self.id}"

    def __repr__(self):
        return f"<Module {str(self)}>"


@node_packer(MOT.MODULE, ModuleData, lang.Module)
class ModulePacker(NodePacker[ModuleData, lang.Module]):
    PARENTS: ClassVar[ParentsT] = set()

    def walk(self, module: lang.Module, tree: PackContext):
        for file in module.files:
            tree.visit(file)

    def pack(self, module: lang.Module) -> "ModuleData":
        return ModuleData(
            id=module.id,
            name=module.name,
            committed=module.committed,
            parent_id=None,
        )

    def unpack(self, module: ModuleData, parent: Optional[lang.Module]) -> lang.Module:
        return lang.Module(
            id=module.id,
            name=module.name,
            committed=module.committed,
            files=[],
        )

    def unwalk(self, module: lang.Module, tree: ModuleTree):
        module.files = tree.get_descendants(module.id, lang.File, recursive=True)


@dataclass
class FileData(NodeData, Revisioned):
    name: str

    def __str__(self):
        return f"{self.name}"

    def __repr__(self):
        return f"<File {str(self)}>"


@node_packer(MOT.FILE, FileData, lang.File)
class FilePacker(NodePacker[FileData, lang.File]):
    PARENTS: ClassVar[ParentsT] = {MOT.MODULE}

    def walk(self, file: lang.File, tree: PackContext):
        for statement in file.statements:
            tree.visit(statement)
        for child in file.children:
            tree.visit(child)

    def pack(self, file: lang.File) -> "FileData":
        return FileData(
            id=file.id,
            parent_id=file.module.id,
            name=file.name,
            revision=file.revision,
        )

    def unpack(self, file: FileData, parent: lang.Module) -> lang.File:
        return lang.File(
            id=file.id,
            module=parent,
            name=file.name,
            revision=file.revision,
            statements=[],
            children=[],
        )

    def unwalk(self, file: lang.File, tree: ModuleTree):
        file.statements = tree.get_descendants(file.id, lang.Statement, recursive=True)
        file.children = tree.get_descendants(file.id, lang.File)


@dataclass
class StatementData(NodeData, Ordered, Revisioned):
    type: StatementType
    name: Optional[str]

    def __str__(self):
        return f"{self.type.name} {self.name}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"


@node_packer(MOT.STATEMENT, StatementData, lang.Statement)
class StatementPacker(NodePacker[StatementData, lang.Statement]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT, MOT.FILE}

    def walk(self, statement: lang.Statement, tree: PackContext):
        for child in statement.children:
            tree.visit(child)

    def pack(self, statement: lang.Statement) -> "StatementData":
        return StatementData(
            id=statement.id,
            parent_id=statement.parent_id,
            order_key=statement.order_key,
            revision=statement.revision,
            type=statement.type,
            name=statement.name,
        )

    def unpack(
        self, statement: StatementData, parent: lang.File | lang.Statement
    ) -> lang.Statement:
        cls = lang.Blank if statement.type == StatementType.BLANK else lang.Statement
        return cls(
            id=statement.id,
            parent=parent,
            file=parent if isinstance(parent, lang.File) else parent.file,
            children=[],
            order_key=statement.order_key,
            revision=statement.revision,
            type=statement.type,
            name=statement.name,
        )

    def unwalk(self, statement: lang.Statement, tree: ModuleTree):
        statement.children = tree.get_descendants(statement.id, lang.Statement)


class BlankData(StatementData):
    pass


@node_packer(MOT.STATEMENT, BlankData, lang.Blank)
class BlankPacker(StatementPacker, NodePacker[BlankData, lang.Blank]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def pack(self, symbol: lang.Blank) -> "BlankData":
        statement_data = super().pack(symbol)
        return BlankData(**statement_data.__dict__)

    def unpack(self, statement: BlankData, parent: lang.Statement | lang.File) -> lang.Blank:
        statement = super().unpack(statement, parent)
        return lang.Blank(**statement.__dict__)


@dataclass
class TextData(StatementData):
    html: str


@node_packer(MOT.STATEMENT, TextData, lang.Text)
class TextPacker(StatementPacker, NodePacker[TextData, lang.Text]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def pack(self, symbol: lang.Text) -> "TextData":
        statement_data = super().pack(symbol)
        return TextData(**statement_data.__dict__, html=symbol.html)

    def unpack(self, symbol: TextData, parent: lang.Statement | lang.File) -> lang.Text:
        statement = super().unpack(symbol, parent)
        return lang.Text(**statement.__dict__, html=symbol.html)


# symbols


@dataclass
class SymbolData(StatementData):
    pass


@dataclass
class TypeData(SymbolData):
    tag: Optional[TypeTag]
    flags: Optional[TypeFlag]
    description: Optional[str]


@node_packer(MOT.STATEMENT, TypeData, lang.Type)
class TypePacker(StatementPacker, NodePacker[TypeData, lang.Type]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Type, tree: PackContext):
        super().walk(symbol, tree)
        for field in symbol.fields:
            tree.visit(field)

    def pack(self, symbol: lang.Type) -> "TypeData":
        statement_data = super().pack(symbol)
        return TypeData(
            **statement_data.__dict__,
            description=symbol.description,
            tag=symbol.tag,
            flags=symbol.flags,
        )

    def unpack(self, symbol: TypeData, parent: lang.File | lang.Statement) -> lang.Type:
        statement = super().unpack(symbol, parent)
        return lang.Type(
            **statement.__dict__,
            description=symbol.description,
            tag=symbol.tag,
            flags=symbol.flags,
            fields=[],
        )

    def unwalk(self, symbol: lang.Type, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)


@dataclass
class TaskData(SymbolData):
    description: Optional[str]
    modifier: Optional[ExpectationModifier]


@node_packer(MOT.STATEMENT, TaskData, lang.Task)
class TaskPacker(StatementPacker, NodePacker[TaskData, lang.Task]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Task, tree: PackContext):
        super().walk(symbol, tree)
        for field in symbol.fields:
            tree.visit(field)

    def pack(self, symbol: lang.Task) -> "TaskData":
        statement_data = super().pack(symbol)
        return TaskData(
            **statement_data.__dict__,
            description=symbol.description,
            modifier=symbol.modifier,
        )

    def unpack(self, symbol: TaskData, parent: lang.File | lang.Statement) -> lang.Task:
        statement = super().unpack(symbol, parent)
        return lang.Task(
            **statement.__dict__,
            description=symbol.description,
            modifier=symbol.modifier,
            fields=[],
        )

    def unwalk(self, symbol: lang.Task, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)


@dataclass
class ExpectationData(SymbolData):
    description: Optional[str]
    modifier: Optional[ExpectationModifier]
    reference_id: Optional[UUID]


@node_packer(MOT.STATEMENT, ExpectationData, lang.Expectation)
class ExpectationPacker(StatementPacker, NodePacker[ExpectationData, lang.Expectation]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def pack(self, symbol: lang.Expectation) -> "ExpectationData":
        statement_data = super().pack(symbol)
        return ExpectationData(
            **statement_data.__dict__,
            modifier=symbol.modifier,
            description=symbol.description,
            reference_id=symbol.reference.id
            if isinstance(symbol.reference, lang.Statement)
            else symbol.reference,
        )

    def unpack(
        self, symbol: ExpectationData, parent: lang.File | lang.Statement
    ) -> lang.Expectation:
        statement = super().unpack(symbol, parent)
        return lang.Expectation(
            **statement.__dict__,
            modifier=symbol.modifier,
            description=symbol.description,
            reference=symbol.reference_id,
        )


@dataclass
class CodeData(SymbolData):
    modifier: Optional[ExpectationModifier]
    language: Optional[str]
    code: Optional[str]


@node_packer(MOT.STATEMENT, CodeData, lang.Code)
class CodePacker(StatementPacker, NodePacker[CodeData, lang.Code]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Code, tree: PackContext):
        super().walk(symbol, tree)
        for field in symbol.fields:
            tree.visit(field)

    def pack(self, symbol: lang.Code) -> "CodeData":
        statement_data = super().pack(symbol)
        return CodeData(
            **statement_data.__dict__,
            modifier=symbol.modifier,
            language=symbol.language,
            code=symbol.code,
        )

    def unpack(self, symbol: CodeData, parent: lang.File | lang.Statement) -> lang.Code:
        statement = super().unpack(symbol, parent)
        return lang.Code(
            **statement.__dict__,
            modifier=symbol.modifier,
            fields=[],
            language=symbol.language,
            code=symbol.code,
        )

    def unwalk(self, symbol: lang.Code, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)


@dataclass
class ModelData(SymbolData):
    external_name: Optional[str]


@node_packer(MOT.STATEMENT, ModelData, lang.Model)
class ModelPacker(StatementPacker, NodePacker[ModelData, lang.Model]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def pack(self, symbol: lang.Model) -> "ModelData":
        statement_data = super().pack(symbol)
        return ModelData(**statement_data.__dict__, external_name=symbol.external_name)

    def unpack(self, symbol: ModelData, parent: lang.File | lang.Statement) -> lang.Model:
        statement = super().unpack(symbol, parent)
        return lang.Model(**statement.__dict__, external_name=symbol.external_name)


@dataclass
class RequirementData(SymbolData):
    reference_module: Optional[ModuleReference]


@node_packer(MOT.STATEMENT, RequirementData, lang.Requirement)
class RequirementPacker(StatementPacker, NodePacker[RequirementData, lang.Requirement]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def pack(self, symbol: lang.Requirement) -> "RequirementData":
        statement_data = super().pack(symbol)
        reference = (
            ModuleReference(
                module_id=symbol.module_id,
                module_name=symbol.module_name,
                version=symbol.version,
            )
            if symbol.module_id
            else None
        )
        return RequirementData(
            **statement_data.__dict__,
            reference_module=reference,
        )

    def unpack(
        self, symbol: RequirementData, parent: lang.File | lang.Statement
    ) -> lang.Requirement:
        statement = super().unpack(symbol, parent)
        return lang.Requirement(
            **statement.__dict__,
            module_id=symbol.reference_module.id if symbol.reference_module else None,
            module_name=symbol.reference_module.name if symbol.reference_module else None,
            version=symbol.reference_module.version if symbol.reference_module else None,
        )


@dataclass
class ValueData(SymbolData):
    description: Optional[str]
    tag: Optional[TypeTag]
    flags: Optional[TypeFlag]
    value: Optional[typing.Any]
    modifier: Optional[ExpectationModifier]


@node_packer(MOT.STATEMENT, ValueData, lang.Value)
class ValuePacker(StatementPacker, NodePacker[ValueData, lang.Value]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Value, tree: PackContext):
        super().walk(symbol, tree)
        for field in symbol.fields:
            tree.visit(field)

    def pack(self, symbol: lang.Value) -> "ValueData":
        statement_data = super().pack(symbol)
        return ValueData(
            **statement_data.__dict__,
            tag=symbol.tag,
            flags=symbol.flags,
            modifier=symbol.modifier,
            description=symbol.description,
            value=symbol.value,
        )

    def unpack(self, symbol: ValueData, parent: lang.File | lang.Statement) -> lang.Value:
        statement = super().unpack(symbol, parent)
        return lang.Value(
            **statement.__dict__,
            tag=symbol.tag,
            flags=symbol.flags,
            modifier=symbol.modifier,
            description=symbol.description,
            value=symbol.value,
        )

    def unwalk(self, symbol: lang.Value, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)


@dataclass
class DatasetData(SymbolData):
    description: Optional[str]
    modifier: Optional[ExpectationModifier]
    versioned: Optional[bool]


@node_packer(MOT.STATEMENT, DatasetData, lang.Dataset)
class DatasetPacker(StatementPacker, NodePacker[DatasetData, lang.Dataset]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def walk(self, symbol: lang.Dataset, tree: PackContext):
        super().walk(symbol, tree)
        for field in symbol.fields:
            tree.visit(field)

    def pack(self, symbol: lang.Dataset) -> "DatasetData":
        statement_data = super().pack(symbol)
        return DatasetData(
            **statement_data.__dict__,
            modifier=symbol.modifier,
            description=symbol.description,
            versioned=symbol.versioned,
        )

    def unpack(self, symbol: DatasetData, parent: lang.Statement) -> lang.Dataset:
        statement = super().unpack(symbol, parent)
        return lang.Dataset(
            **statement.__dict__,
            modifier=symbol.modifier,
            description=symbol.description,
            versioned=symbol.versioned,
            fields=[],
        )

    def unwalk(self, symbol: lang.Dataset, tree: ModuleTree):
        super().unwalk(symbol, tree)
        symbol.fields = tree.get_descendants(symbol.id, lang.Field)


STATEMENT_DATA_BY_TYPE = {
    StatementType.TYPE: TypeData,
    StatementType.TASK: TaskData,
    StatementType.EXPECTATION: ExpectationData,
    StatementType.CODE: CodeData,
    StatementType.MODEL: ModelData,
    StatementType.REQUIREMENT: RequirementData,
    StatementType.DATASET: DatasetData,
    StatementType.VALUE: ValueData,
}
STATEMENT_TYPE_BY_DATA_CLASS = {v: k for k, v in STATEMENT_DATA_BY_TYPE.items()}


@dataclass
class FieldData(NodeData, Ordered, Revisioned):
    name: Optional[str]
    key: str
    tag: TypeTag
    hint: Optional[TypeHint]
    description: Optional[str]
    flags: TypeFlag
    reference_id: Optional[UUID] = None
    metadata: Optional[typing.Any] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{self.parent_id}:{self.order_key} {name_str}{self.tag.value}"

    def __repr__(self):
        return f"<Field {str(self)}>"


@node_packer(MOT.FIELD, FieldData, lang.Field)
class FieldPacker(NodePacker[FieldData, lang.Field]):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    def pack(self, field: lang.Field) -> "FieldData":
        reference = field.reference.id if isinstance(field.reference, lang.Statement) else None
        return FieldData(
            id=field.id,
            parent_id=field.parent_id,
            order_key=field.order_key,
            revision=field.revision,
            name=field.name,
            key=field.key,
            tag=field.tag,
            hint=field.hint,
            flags=field.flags,
            description=field.description,
            reference_id=reference,
            metadata=field.metadata,
        )

    def unpack(self, field: FieldData, parent: lang.Statement) -> lang.Field:
        return lang.Field(
            parent=parent,
            id=field.id,
            name=field.name,
            revision=field.revision,
            key=field.key,
            tag=field.tag,
            hint=field.hint,
            flags=field.flags,
            description=field.description,
            reference=field.reference_id,
            metadata=field.metadata,
        )


@dataclass
class DatasetViewData(NodeData, Ordered, Revisioned):
    PARENTS: ClassVar[ParentsT] = {MOT.FILE, MOT.STATEMENT}

    id: UUID
    name: str
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    length: Optional[int] = None
    reference_id: Optional[UUID] = None


@node_packer(MOT.DATASET_VIEW, DatasetViewData, lang.DatasetView)
class DatasetViewPacker(NodePacker[DatasetViewData, lang.DatasetView]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, view: lang.DatasetView) -> "DatasetViewData":
        return DatasetViewData(
            id=view.id,
            order_key=view.order_key,
            name=view.name,
            query=view.query,
            sort=view.sort,
            reference_id=view.reference.id if view.reference else None,
        )

    def unpack(self, view: DatasetViewData, parent: lang.Statement) -> lang.DatasetView:
        return lang.DatasetView(
            id=view.id,
            name=view.name,
            source=parent,
            query=view.query,
            sort=view.sort,
            reference=view.reference_id,
        )


@dataclass
class RecordData(NodeData, Ordered, Revisioned):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    data: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.parent_id}:{self.order_key} {describe_type(self.data)}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"


@node_packer(MOT.RECORD, RecordData, lang.Record)
class RecordPacker(NodePacker[RecordData, lang.Record]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, record: lang.Record) -> "RecordData":
        return RecordData(
            id=record.id,
            order_key=record.order_key,
            revision=record.revision,
            data=record.data,
        )

    def unpack(self, record: "RecordData", parent: lang.Statement) -> lang.Record:
        return lang.Record(
            id=record.id,
            revision=record.revision,
            data=record.data,
        )


@dataclass
class ResolvedFieldData(NodeData):
    field_id: UUID


@node_packer(MOT.RESOLVED_FIELD, ResolvedFieldData, lang.ResolvedField)
class ResolvedFieldPacker(NodePacker[ResolvedFieldData, lang.ResolvedField]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, resolved_field: lang.ResolvedField) -> "ResolvedFieldData":
        return ResolvedFieldData(
            id=resolved_field.id,
            parent_id=resolved_field.parent_id,
            field_id=resolved_field.field.id,
        )


@dataclass
class IssueData(NodeData):
    scope: InterpScope
    kind: IssueKind
    type: IssueType
    message: Optional[str]


@node_packer(MOT.ISSUE, IssueData, lang.Issue)
class IssuePacker(NodePacker[IssueData, lang.Issue]):
    PARENTS: ClassVar[ParentsT] = {MOT.STATEMENT}

    def pack(self, issue: lang.Issue) -> "IssueData":
        return IssueData(
            id=issue.id,
            parent_id=issue.statement_id or issue.file_id,
            scope=issue.scope,
            kind=issue.kind,
            type=issue.type,
            message=issue.message,
        )

    def unpack(self, issue: IssueData, parent: lang.Statement) -> lang.Issue:
        raise NotImplementedError


# other objects


class DataPacker(abc.ABC, typing.Generic[DataT, ObjectT]):
    """Generic data packer for non-node data types"""

    def pack(self, object: ObjectT) -> DataT:
        raise NotImplementedError

    def unpack(self, data: DataT) -> ObjectT:
        raise NotImplementedError


_data_packers_by_data: dict[typing.Type[DataT], "DataPacker"] = {}
_data_packers_by_node: dict[typing.Type, "DataPacker"] = {}


def data_packer(data_t: typing.Type[DataT], node_t: typing.Optional[typing.Type] | None):
    """Decorator to register a data packer for a given type"""

    def decorator(cls: "DataPacker"):
        if data_t in _data_packers_by_data:
            raise ValueError(
                f"packer for {data_t} already registered: {_data_packers_by_data[data_t]}"
            )
        if node_t in _data_packers_by_node:
            raise ValueError(
                f"packer for {node_t} already registered: {_data_packers_by_node[node_t]}"
            )
        packer = cls()
        _data_packers_by_data[data_t] = packer
        if node_t:
            _data_packers_by_node[node_t] = packer
        return cls

    return decorator


def pack_data(data: ObjectT) -> DataT:
    """Pack a language data object into a flat module node"""
    packer = _data_packers_by_node[type(data)]
    return packer.pack(data)


def unpack_data(data: DataT) -> ObjectT:
    """Unpack a flat module node into a language data object"""
    packer = _data_packers_by_data[type(data)]
    return packer.unpack(data)


@dataclass
class XBlockData:
    kind: str
    source: str
    value: Optional[typing.Any] = None
    path: Optional[str] = None

    def __str__(self):
        return f"{self.kind} {self.source}"

    def __repr__(self):
        return f"<XBlock {str(self)}>"


@data_packer(XBlockData, "XBlock")
class XBlockPacker(DataPacker[XBlockData, "XBlock"]):
    def pack(self, object: "XBlock") -> XBlockData:
        return XBlockData(
            kind=object.kind, source=object.source, value=object.value, path=object.path
        )

    def unpack(self, data: XBlockData) -> "XBlock":
        from bench.bench.build import XBlock

        return XBlock(kind=data.kind, source=data.source, value=data.value, path=data.path)


@dataclass
class RemoteObjectData:
    id: UUID
    sha512: str
    content_length: int
    content_type: str
    name: Optional[str]

    def __str__(self):
        return f"{self.id} {self.name} ({self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<RemoteObject {self}>"


@data_packer(RemoteObjectData, lang.RemoteObject)
class RemoteObjectPacker(DataPacker[RemoteObjectData, lang.RemoteObject]):
    def pack(self, object: lang.RemoteObject) -> RemoteObjectData:
        return RemoteObjectData(
            id=object.id,
            sha512=object.sha512,
            content_length=object.content_length,
            content_type=object.content_type,
            name=object.name,
        )

    def unpack(self, data: RemoteObjectData) -> lang.RemoteObject:
        return lang.RemoteObject(
            id=data.id,
            sha512=data.sha512,
            content_length=data.content_length,
            content_type=data.content_type,
            name=data.name,
        )


@dataclass
class SecretData:
    id: UUID
    sha512: str
    value: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.id} ({self.sha512})"

    def __repr__(self):
        return f"<Secret {self}>"


@data_packer(SecretData, lang.Secret)
class SecretPacker(DataPacker[SecretData, lang.Secret]):
    def pack(self, object: lang.Secret) -> SecretData:
        return SecretData(id=object.id, sha512=object.sha512, value=object.value)

    def unpack(self, data: SecretData) -> lang.Secret:
        return lang.Secret(id=data.id, sha512=data.sha512, value=data.value)


@dataclass
class ExecutionFrameData:
    """Wire-able representation of an execution frame."""

    id: UUID
    module_id: UUID
    runnable_id: UUID
    root_id: Optional[UUID]
    parent_id: Optional[UUID]
    entered_at: datetime
    exited_at: Optional[datetime]
    cached_generated_at: Optional[datetime]
    cached_duration: Optional[float]
    inputs: Optional[Any]
    outputs: Optional[Any]
    error: Optional[RunErrorData]
    queue_position: Optional[int]
    # additional context data not in ExecutionFrame
    project_id: UUID
    tracing_level: Optional[int]
    worker_id: UUID
    trigger_type: Optional[ExecutionTriggerType]
    trigger_id: Optional[UUID]

    @staticmethod
    def from_frame(frame: ExecutionFrame, *, session: "Session") -> ExecutionFrameData:
        # this should also follow the packer pattern
        if frame.error:
            if frame.runnable is None:
                raise ValueError(f"error outside code: {frame}")
            stack_summary = traceback.StackSummary.extract(
                traceback.walk_tb(frame.error.__traceback__), capture_locals=True
            )
            if isinstance(frame.runnable, Code):
                stack = PyFrameData.from_stack(stack_summary)
                stack = PyFrameData.clean(stack, frame.runnable, session=session)
            else:
                stack = []
            error_str = str(frame.error)
            # remove (source=...) from error message
            error_str = re.sub(r"\(source=.+\)", "", error_str)
            error_data = RunErrorData(
                type=type(frame.error).__name__,
                symbol=str(frame.runnable),
                message=f"{type(frame.error).__name__}: {error_str}",
                traceback=stack,
            )
        else:
            error_data = None
        return ExecutionFrameData(
            id=frame.id,
            module_id=frame.module_id,
            runnable_id=frame.runnable.id if frame.runnable else None,
            root_id=frame.root.id if frame.root else None,
            parent_id=frame.parent.id if frame.parent else None,
            entered_at=frame.entered_at,
            exited_at=frame.exited_at,
            cached_generated_at=frame.cached_generated_at,
            cached_duration=frame.cached_duration,
            inputs=frame.inputs,
            outputs=frame.outputs,
            error=error_data,
            queue_position=frame.queue_position,
            project_id=session.ctx.project_id,
            tracing_level=session.ctx.tracing_level,
            worker_id=session.ctx.worker_id,
            trigger_type=session.ctx.trigger_type,
            trigger_id=session.ctx.trigger_id,
        )
