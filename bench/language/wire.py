import enum
import typing
from collections import OrderedDict
from dataclasses import dataclass
from typing import Optional, Union
from uuid import UUID, uuid5

from more_itertools import first

from bench import language
from bench.language import ErrorType
from bench.language.parse import get_type_root_id
from bench.language.reconstruct import get_reference_as_path
from bench.language.type import (
    PRIMITIVE_TYPES,
    BuildSettings,
    EvaluateSettings,
    GeneratedMapping,
    StatementModifier,
    StatementPath,
    StatementType,
    SymbolType,
    TypeTag,
    XKind,
    XSource,
)
from bench.utils.fractional import INTEGER_ZERO, generate_n_keys_between
from bench.utils.func import describe_type

#
# Stable, concise and flat language data structures for transit and storage.
#


@dataclass(repr=False, slots=True)
class TypeNodeData:
    id: UUID
    revision: int
    name: Optional[str]
    tag: TypeTag
    statement_id: UUID
    order_key: str
    description: Optional[str] = None
    value: Optional[typing.Any] = None
    reference: Union[None, StatementPath, UUID] = None
    parent_id: Optional[UUID] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag.value}"

    def __repr__(self):
        return f"<TypeNode {str(self)}>"


@dataclass(repr=False, slots=True)
class SimpleTypeNodeData:
    id: UUID
    revision: int
    name: Optional[str]
    tag: TypeTag
    statement_id: UUID
    order_key: str
    description: Optional[str]
    is_output: bool
    is_array: bool
    is_nullable: bool
    value: Optional[typing.Any] = None
    reference_id: Union[None, UUID] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag.value}"

    def __repr__(self):
        return f"<SimpleTypeNode {str(self)}>"


@dataclass(repr=False, slots=True)
class RecordData:
    id: UUID
    statement_id: UUID
    order_key: str
    revision: int
    data: Optional[typing.Any] = None

    def __str__(self):
        return f"{self.order_key} {describe_type(self.data)}"

    def __repr__(self):
        return f"<Record {str(self)}>"


@dataclass(repr=False, slots=True)
class XBlockData:
    id: UUID
    statement_id: UUID
    order_key: str
    kind: XKind
    source: XSource
    revision: int
    value: Optional[typing.Any] = None
    path: Optional[str] = None
    description: Optional[str] = None

    def __str__(self):
        return f"{self.order_key} {self.kind.value} {self.source.value}"

    def __repr__(self):
        return f"<XBlock {str(self)}>"


@dataclass(repr=False, slots=True)
class ModuleData:
    id: UUID
    name: str
    files: list["FileData"]
    committed: bool = False

    def __str__(self):
        return f"{self.name}@{self.id} ({len(self.files)} files)"

    def __repr__(self):
        return f"<Module {str(self)}>"


@dataclass(repr=False, slots=True)
class FileData:
    id: UUID
    module_id: UUID
    path: str
    statements: list["StatementData"]
    revision: int
    generated: bool

    def __str__(self):
        generated_str = ".gen" if self.generated else ""
        return f"{self.module_id}/{self.path}{generated_str}"

    def __repr__(self):
        return f"<File {str(self)}>"


ModuleReference = typing.NamedTuple(
    "ModuleReference", [("name", str), ("version", str), ("id", Optional[UUID])]
)


@dataclass(repr=False, slots=True)
class StatementData:
    id: UUID
    module_id: UUID
    file_id: UUID
    order_key: str
    revision: int
    type: StatementType
    modifier: Optional[StatementModifier]
    name: Optional[str]
    fqn: Optional[str]
    parent_id: Optional[UUID]
    reference: Union[None, StatementPath, UUID]
    text: Optional[str]
    symbol_type: Optional[SymbolType]
    generated: bool
    # symbol contents
    root_type_tag: Optional[TypeTag] = None
    type_nodes: Union[list[SimpleTypeNodeData], None] = None
    description: Optional[str] = None
    lang: Optional[str] = None
    code: Optional[str] = None
    xblocks: Optional[list[XBlockData]] = None
    provider: Optional[str] = None
    external_name: Optional[str] = None
    records: Optional[list[RecordData]] = None
    generated_mappings: Optional[list[GeneratedMapping]] = None
    build_settings: Optional[BuildSettings] = None
    evaluate_settings: Optional[EvaluateSettings] = None
    reference_module: Optional[ModuleReference] = None

    @property
    def reference_id(self) -> Optional[UUID]:
        return self.reference if isinstance(self.reference, UUID) else None

    def __str__(self):
        parent_str = f"{self.parent_id}:" if self.parent_id else ""
        loc = str(self.file_id) + ":" + parent_str + str(self.order_key)
        symbol_type_str = self.symbol_type.name if self.symbol_type else ""
        ref_str = f"ref={self.reference}" if self.reference else ""
        ref_module_str = f"ref_module={self.reference_module}" if self.reference_module else ""
        content_str = ", ".join((s for s in (ref_str, ref_module_str) if s))
        return f"{loc}: {self.type.name} {symbol_type_str} {self.name} ({content_str})"

    def __repr__(self):
        return f"<Statement {str(self)}>"


def rmap_module(module: language.Module) -> ModuleData:
    return ModuleData(
        id=module.id,
        name=module.name,
        files=[rmap_file(file) for file in module.files],
    )


def wmap_module(data: ModuleData) -> language.Module:
    """Maps module data back into a module. Restores explicit statement references without checking!"""
    module = language.Module(id=data.id, name=data.name)
    module.files = [wmap_file(file, module) for file in data.files]

    # TODO @Architecture @Cleanup: remove reference resolution as part of wire mapping
    #  It shouldn't be needed anymore, even now, but getting some errors, so fix later.
    # restore statement references
    statements = {statement.id: statement for file in module.files for statement in file.statements}
    for file_data, file in zip(data.files, module.files):
        for statement_data, statement in zip(file_data.statements, file.statements):
            if statement_data.parent_id is not None:
                # parent should exist but may not if it was not included in the module data
                # but exists in the source (e.g. an invalid comment parent)
                statement.parent = statements.get(statement_data.parent_id)

            # resolve references in statement and in symbol content
            # ignore references we couldn't find since are either
            #  1) refs to "deleted" statements or
            #  2) refs to statements in other modules (which should be by path anyway, but we can't check here)

            if isinstance(statement_data.reference, UUID):
                reference = statements.get(statement_data.reference, None)
                if reference is not None:
                    statement.reference = get_reference_as_path(reference, statement)

            type_node = None
            if isinstance(statement.content, language.TypeNode):
                type_node = statement.content
            elif isinstance(
                statement.content,
                (language.DatasetContent, language.TaskContent, language.CodeContent),
            ):
                type_node = statement.content.type_node
            if type_node is not None:
                for node in type_node.walk():
                    data_node = first(
                        (n for n in statement_data.type_nodes if n.id == node.id), None
                    )
                    if data_node and isinstance(data_node.reference_id, UUID):
                        reference = statements.get(node.reference, None)
                        if reference is not None:
                            node.reference = get_reference_as_path(reference, statement)
                            node.source_reference = node.reference

    return module


def rmap_file(file: language.File) -> FileData:
    return FileData(
        id=file.id,
        module_id=file.module.id,
        path=file.path,
        generated=file.generated,
        statements=[rmap_statement(statement) for statement in file.statements],
        revision=1,
    )


def wmap_file(data: FileData, module: language.Module) -> language.File:
    file = language.File(
        id=data.id,
        module=module,
        path=data.path,
        generated=data.generated,
    )
    file.statements = [wmap_statement(statement, file) for statement in data.statements]
    return file


def rmap_statement(statement: language.Statement) -> StatementData:
    """Maps a language statement to a wire statement (incl. refs)."""
    # use statement id if possible, else use statement path
    reference = (
        statement.reference_id if statement.reference_id is not None else statement.reference
    )
    data = StatementData(
        module_id=statement.file.module.id,
        file_id=statement.file.id,
        revision=1,
        id=statement.id,
        order_key=statement.order_key,
        parent_id=statement.parent_id,
        type=statement.type,
        modifier=statement.modifier,
        reference=reference,
        name=statement.name,
        fqn=statement.fqn,
        text=statement.text,
        symbol_type=statement.symbol_type,
        generated=statement.generated,
    )
    if statement.content is not None:
        rmap_symbol(statement.content, data)
    return data


def wmap_statement(data: StatementData, file: language.File) -> language.Statement:
    """Maps a wire statement's _contents_ (excl. refs) to a language statement."""
    reference = data.reference if isinstance(data.reference, (StatementPath, UUID)) else None
    statement = language.Statement(
        id=data.id,
        file=file,
        parent=None,  # must be restored later
        reference=reference,  # also restored later if it was an id
        order_key=data.order_key,
        type=data.type,
        modifier=data.modifier,
        name=data.name,
        text=data.text,
        symbol_type=data.symbol_type,
        generated=data.generated,
    )
    if statement.type == StatementType.DEFINITION:
        statement.content = wmap_symbol(data)
    return statement


def rmap_symbol(content: language.SymbolContent, data: StatementData) -> None:
    """Maps a language symbol's _contents_ (excl. refs) to a wire statement."""
    if isinstance(content, language.GeneratorContent):
        data.generated_mappings = content.generated_mappings
    # generator content is a component of other content types
    if isinstance(content, language.TypeNode):
        data.description = content.description
        data.root_type_tag, data.type_nodes = rmap_type_nodes(
            data.id, rmap_type_node(data.id, content)
        )
    elif isinstance(content, language.TaskContent):
        data.description = content.description
        data.root_type_tag, data.type_nodes = rmap_type_nodes(
            data.id, rmap_type_node(data.id, content.type_node)
        )
    elif isinstance(content, language.ExpectationContent):
        data.description = content.description
    elif isinstance(content, language.CodeContent):
        data.description = content.description
        data.lang = content.language
        data.code = content.code
        data.root_type_tag, data.type_nodes = rmap_type_nodes(
            data.id, rmap_type_node(data.id, content.type_node)
        )
        data.xblocks = [rmap_xblock(data.id, xblock) for xblock in content.xblocks]
    elif isinstance(content, language.ModelContent):
        data.provider = content.provider
        data.external_name = content.external_name
    elif isinstance(content, language.CapabilityContent):
        data.description = content.description
    elif isinstance(content, language.DatasetContent):
        data.lang = content.language
        data.description = content.description
        data.records = [rmap_record(data.id, r) for r in content.records]
        data.root_type_tag, data.type_nodes = rmap_type_nodes(
            data.id, rmap_type_node(data.id, content.type_node)
        )
    elif isinstance(content, language.BuildContent):
        data.description = content.comment
        data.build_settings = content.settings
        data.evaluate_settings = content.evaluate_settings  # :BuildEvaluationSettings
    elif isinstance(content, language.RequirementContent):
        if content.module_name and content.version:
            data.reference_module = ModuleReference(
                content.module_name, content.version, id=content.module_id
            )
    elif isinstance(content, language.RunconfigContent):
        pass
    else:
        raise ValueError(f"unexpected symbol type {content}")


def wmap_symbol(data: StatementData) -> language.SymbolContent:
    """Maps a wire statement's symbol contents to a language symbol."""
    if data.root_type_tag:
        type_node = wmap_type_node(
            wmap_type_nodes(data.root_type_tag, data.type_nodes, data.id, data.symbol_type)
        )
    else:
        type_node = None

    if data.symbol_type == SymbolType.TYPE:
        type_node.description = data.description  # prefer type node from wire
        return type_node
    elif data.symbol_type == SymbolType.TASK:
        return language.TaskContent(
            generated_mappings=data.generated_mappings,
            type_node=type_node,
            description=data.description,
        )
    elif data.symbol_type == SymbolType.EXPECTATION:
        return language.ExpectationContent(description=data.description)
    elif data.symbol_type == SymbolType.CODE:
        return language.CodeContent(
            generated_mappings=data.generated_mappings,
            description=data.description,
            language=data.lang,
            code=data.code,
            type_node=type_node,
            xblocks=[wmap_xblock(x) for x in (data.xblocks or [])],
        )
    elif data.symbol_type == SymbolType.MODEL:
        return language.ModelContent(
            provider=data.provider,
            external_name=data.external_name,
        )
    elif data.symbol_type == SymbolType.CAPABILITY:
        return language.CapabilityContent(description=data.description)
    elif data.symbol_type == SymbolType.DATA:
        return language.DatasetContent(
            description=data.description,
            language=data.lang,
            type_node=type_node,
            records=[wmap_record(r) for r in (data.records or [])],
        )
    elif data.symbol_type == SymbolType.BUILD:
        return language.BuildContent(
            comment=data.description,
            generated_mappings=data.generated_mappings,
            settings=data.build_settings,
            evaluate_settings=data.evaluate_settings,  # :BuildEvaluationSettings
        )
    elif data.symbol_type == SymbolType.REQUIREMENT:
        return language.RequirementContent(
            module_name=data.reference_module.name if data.reference_module else None,
            version=data.reference_module.version if data.reference_module else None,
            module_id=data.reference_module.id if data.reference_module else None,
        )
    elif data.symbol_type == SymbolType.RUNCONFIG:
        return language.RunconfigContent()
    else:
        raise ValueError(f"unexpected symbol type {data.symbol_type} for statement {data}")


def rmap_record(statement_id: UUID, record: language.Record) -> RecordData:
    """Maps a record to a record data object."""
    return RecordData(
        id=record.id,
        revision=1,
        data=record.data,
        statement_id=statement_id,
        order_key=record.order_key,
    )


def wmap_record(data: RecordData) -> language.Record:
    """Maps a record data object to a record."""
    return language.Record(id=data.id, data=data.data, order_key=data.order_key)


def rmap_type_node(statement_id: UUID, node: language.TypeNode) -> list[TypeNodeData]:
    """Maps a type node tree structure to a flat list of type node data."""
    nodes_data = OrderedDict()
    for n in node.walk():
        if n.id in nodes_data:
            # already seen (multiple references to same node)
            # this is allowed in rmap but in wmap as TypeNodeData is flattened
            continue
        reference = n.reference
        if isinstance(reference, (language.TypeNode, language.Type)):
            reference = reference.id
        nodes_data[n.id] = TypeNodeData(
            id=n.id,
            revision=1,
            name=n.name,
            tag=n.tag,
            description=n.description,
            value=n.value,
            reference=reference,
            parent_id=None,  # will be set in second pass
            statement_id=statement_id,
            order_key=INTEGER_ZERO,  # will be set in second pass
        )

    # assign parent ids
    for n in node.walk():
        if n.children is not None:
            child_order_keys = generate_n_keys_between(None, None, len(n.children))
            for order_key, child in zip(child_order_keys, n.children):
                nodes_data[child.id].order_key = order_key
                nodes_data[child.id].parent_id = n.id

    return list(nodes_data.values())


def wmap_type_node(nodes_data: list[TypeNodeData]) -> language.TypeNode:
    """Maps a flat list of wire type nodes to a language type node tree."""

    nodes_by_id = {}
    for data in nodes_data:
        if data.id in nodes_by_id:
            raise ValueError(f"duplicate type node id {data.id}: {data} and {nodes_by_id[data.id]}")
        node = language.TypeNode(
            id=data.id,
            name=data.name,
            tag=data.tag,
            description=data.description,
            value=data.value,
            reference=data.reference,
            source_reference=data.reference,
        )
        nodes_by_id[data.id] = node

    # assign children based on parent ids (sorted by order keys, which works per-parent)
    root = None
    for data in sorted(nodes_data, key=lambda n: n.order_key):
        if data.parent_id is not None:
            parent = nodes_by_id[data.parent_id]
            if parent.children is None:
                parent.children = []
            parent.children.append(nodes_by_id[data.id])
        else:
            root = nodes_by_id[data.id]
    if root is None:
        raise ValueError(f"no root node in {nodes_data}")
    return root


def wmap_type_nodes(
    root_type_tag: TypeTag | None,
    type_nodes: list[SimpleTypeNodeData] | None,
    statement_id: UUID,
    symbol_type: SymbolType,
) -> list[TypeNodeData]:
    """
    Reads a simple type node into a graph type node.
    Because the simple type is flat and skips some intermediate nodes, we need to
    reconstruct them and assign reproducible IDs.
    """
    if root_type_tag is None:
        return []

    def new_id(name: str) -> UUID:
        """Generate a reproducible ID for a child node."""
        return uuid5(statement_id, name)

    def _rmap_child_node(node: SimpleTypeNodeData) -> language.TypeNode:
        if node.tag in PRIMITIVE_TYPES or node.tag == TypeTag.LITERAL:
            lang_node = language.TypeNode(
                id=node.id, tag=node.tag, name=node.name, value=node.value
            )
        elif node.tag == TypeTag.TYPE_REFERENCE or node.reference_id is not None:
            lang_node = language.TypeNode(
                id=node.id, tag=node.tag, name=node.name, reference=node.reference_id
            )
        else:
            raise ValueError(f"type node is not represented simply: {node}")

        if node.is_array:  # hoist into array
            lang_node.name = None
            lang_node.id = new_id("array" + str(lang_node.id))
            lang_node = language.TypeNode(
                id=node.id,
                tag=TypeTag.ARRAY,
                name=node.name,
                description=lang_node.description,
                children=[lang_node],
            )
        if node.is_nullable:  # hoist into union
            lang_node.name = None
            lang_node.id = new_id("union" + str(lang_node.id))
            null = language.TypeNode(
                id=new_id("null" + str(lang_node.id)), tag=TypeTag.NULL, name=None
            )
            lang_node = language.TypeNode(
                id=node.id,
                tag=TypeTag.UNION,
                name=node.name,
                description=lang_node.description,
                children=[lang_node, null],
            )
        return lang_node

    type_nodes = type_nodes or []
    if root_type_tag == TypeTag.STRUCT:
        children = [_rmap_child_node(node) for node in type_nodes]
    elif root_type_tag == TypeTag.ENUM:
        # assumes literal string enums only :LiteralStringEnum
        head_type = language.TypeNode(id=new_id("head"), name=None, tag=TypeTag.STRING)
        children = [head_type, *[_rmap_child_node(node) for node in type_nodes]]
    elif root_type_tag == TypeTag.FUNCTION:
        input_children = [_rmap_child_node(node) for node in type_nodes if not node.is_output]
        output_children = [_rmap_child_node(node) for node in type_nodes if node.is_output]
        input = language.TypeNode(
            id=new_id("input"), tag=TypeTag.STRUCT, name="input", children=input_children
        )
        output = language.TypeNode(
            id=new_id("output"), tag=TypeTag.STRUCT, name="output", children=output_children
        )
        children = [input, output]
    else:
        raise ValueError(f"root type node is not represented simply: {type_nodes}")

    if symbol_type == SymbolType.TYPE:
        # use the root id directly
        root = language.TypeNode(id=statement_id, tag=root_type_tag, name=None, children=children)
    else:
        # statements share a deterministic id pair with their type root :TypeNodeRootId
        root = language.TypeNode(
            id=get_type_root_id(statement_id), tag=root_type_tag, name=None, children=children
        )

    wire_nodes_data = rmap_type_node(statement_id, root)
    # patch revision
    type_nodes_revisions = {node.id: node.revision for node in type_nodes}
    for wire_node in wire_nodes_data:
        wire_node.revision = type_nodes_revisions.get(wire_node.id, 1)
    return wire_nodes_data


def rmap_type_nodes(
    statement_id: UUID | None,
    type_nodes: list[TypeNodeData] | None,
    impute_type_reference: bool = False,
) -> tuple[TypeTag | None, list[SimpleTypeNodeData] | None]:
    """Maps a tree type node into a simple type node."""
    if not type_nodes:
        return None, None

    root: language.TypeNode = wmap_type_node(type_nodes)
    root_type_tag = root.tag
    child_nodes: list[SimpleTypeNodeData] = []

    # map inner nodes (children)
    def _wmap_child_node(
        node: language.TypeNode,
        order_key: str,
        is_array: bool = False,
        is_nullable: bool = False,
        is_output: bool = False,
    ) -> SimpleTypeNodeData:
        # retain resolved references (we trust it's a valid foreign key, else the save will fail)
        reference_id = node.reference if isinstance(node.reference, UUID) else None
        if node.tag == TypeTag.TYPE_REFERENCE or reference_id is not None:
            return SimpleTypeNodeData(
                id=node.id,
                name=node.name,
                tag=node.tag if impute_type_reference else TypeTag.TYPE_REFERENCE,
                statement_id=statement_id,
                order_key=order_key,
                description=node.description,
                reference_id=reference_id,
                revision=1,
                is_array=is_array,
                is_nullable=is_nullable,
                is_output=is_output,
            )
        elif node.tag in PRIMITIVE_TYPES or node.tag == TypeTag.LITERAL:
            return SimpleTypeNodeData(
                id=node.id,
                name=node.name,
                tag=node.tag,
                statement_id=statement_id,
                order_key=order_key,
                description=node.description,
                value=node.value,
                reference_id=reference_id,
                revision=1,
                is_array=is_array,
                is_nullable=is_nullable,
                is_output=is_output,
            )
        elif node.tag == TypeTag.ARRAY:
            child_node = _wmap_child_node(node.head_type, is_array=True, order_key=order_key)
            child_node.name = node.name
            return child_node
        elif node.is_union_with_null:
            child_node = _wmap_child_node(node.head_type, is_nullable=True, order_key=order_key)
            child_node.name = node.name
            return child_node
        else:
            raise ValueError(f"type node cannot be represented simply: {node}")

    # map root node
    if root.tag == TypeTag.STRUCT:
        child_order_keys = generate_n_keys_between(None, None, len(root.children or []))
        for child, order_key in zip(root.children or [], child_order_keys):
            child_nodes.append(_wmap_child_node(child, order_key=order_key))
    elif root.tag == TypeTag.ENUM:
        if root.head_type.tag != TypeTag.STRING:  # :LiteralStringEnum
            raise ValueError(f"non-string enum head type cannot be represented simply: {root}")
        child_order_keys = generate_n_keys_between(None, None, len(root.members))
        for member, order_key in zip(root.members, child_order_keys):
            child_nodes.append(_wmap_child_node(member, order_key=order_key))
    elif root.tag == TypeTag.FUNCTION:
        child_order_keys = generate_n_keys_between(
            None, None, len(root.input.children or []) + len(root.output.children or [])
        )
        for child, order_key in zip(root.input.children or [], child_order_keys):
            child_nodes.append(_wmap_child_node(child, order_key=order_key, is_output=False))
        for child, order_key in zip(
            root.output.children or [], child_order_keys[len(root.input.children or []) :]
        ):
            child_nodes.append(_wmap_child_node(child, order_key=order_key, is_output=True))
    else:
        raise ValueError(f"root type node cannot be represented simply: {type_nodes}")

    return root_type_tag, child_nodes


def wmap_xblock(xblock: XBlockData) -> language.XBlockContent:
    """Maps an xblock data object to an xblock."""
    return language.XBlockContent(
        id=xblock.id,
        order_key=xblock.order_key,
        kind=xblock.kind,
        source=xblock.source,
        value=xblock.value,
        path=xblock.path,
        description=xblock.description,
    )


def rmap_xblock(statement_id: UUID, xblock: language.XBlockContent) -> XBlockData:
    """Maps an xblock to an xblock data object."""
    return XBlockData(
        id=xblock.id,
        statement_id=statement_id,
        order_key=xblock.order_key,
        kind=xblock.kind,
        source=xblock.source,
        value=xblock.value,
        path=xblock.path,
        description=xblock.description,
        revision=1,
    )


#
# Errors
#


@dataclass(repr=False)
class ErrorData:
    type: ErrorType
    statement_id: Optional[UUID]
    message: str
    verbose_message: Optional[str]

    def __str__(self):
        return f"{self.type.name}: {self.message}"

    def __repr__(self):
        return f"<Error {str(self)}>"


def rmap_error(error: language.Error) -> ErrorData:
    return ErrorData(
        type=error.type,
        statement_id=error.statement.id if error.statement is not None else None,
        message=error.message,
        verbose_message=error.verbose_message,
    )


# TODO @Cleanup: execution data doesn't belong to language wire format

#
# Executions
#


class ExecutionTriggerType(enum.StrEnum):
    REST_API = "rest-api"
    UI_INTERACTIVE = "ui-interactive"
    JOB = "job"
    MANUAL = "manual"


class ExecutionTracingLevel(enum.StrEnum):
    ROOT_FRAME = "root-frame"
    ROOT_FRAME_WITH_DATA = "root-frame-with-data"
    ALL_FRAMES = "all-frames"
    ALL_FRAMES_WITH_DATA = "all-frames-with-data"
