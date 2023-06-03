from __future__ import annotations

import enum
import random
import uuid
from dataclasses import dataclass, field
from typing import Any, Callable, Union

import structlog

from bench.language import ModuleIndex
from bench.language.type import (
    Build,
    Code,
    Data,
    Expectation,
    Field,
    InterpSymbol,
    Record,
    StatementModifier,
    SymbolType,
    Task,
    Type,
    TypeContent,
    TypeFlag,
    TypeHint,
    TypeNode,
    TypeTag,
)
from bench.utils.fractional import generate_n_keys_between

logger = structlog.get_logger(__name__)

Expect = Union[Task, Code, Data, Expectation]


# TODO @Architecture: merge instruction ops & nodes into lang/parse? :InstructionOps


class InstructionOp(enum.StrEnum):
    """
    The kind of instruction expressed in a symbol (or sub-symbol).
    """

    # do we really need all <symbol_type> definitions here?
    Pseudo = "pseudo"  # pseudo instructions like models, requirements, references, etc.
    BuildDefinition = "build_definition"
    TypeDefinition = "type_definition"
    TaskDefinition = "task_definition"
    TaskStep = "task_step"
    ExpectationDefinition = "expectation_definition"
    DataDefinition = "data_definition"
    RecordDefinition = "record_definition"
    CodeDefinition = "code_definition"
    ModelDefinition = "model_definition"
    SampleData = "sample_data"
    SampleCode = "sample_code"
    EvaluateCode = "evaluate_code"
    CheckCode = "check_code"


@dataclass(repr=False, slots=True)
class Instruction:
    op: InstructionOp
    node: InterpSymbol | TypeNode | Record
    id: uuid.UUID
    children: list["Instruction"] = field(default_factory=list)

    def __str__(self):
        return f"{self.op} {self.node}"

    def __repr__(self):
        return f"<Instruction {self}>"

    @property
    def is_reference(self) -> bool:
        return isinstance(self.node, InterpSymbol) and self.node.reference is not None

    def walk(self, seen: list["Instruction"] = None):
        """Walks the instruction tree in post order ("bottom up"), ignoring cycles."""
        seen = seen if seen is not None else []
        seen.append(self)
        for child in self.children:
            if child not in seen:  # break cycles (allowed)
                yield from child.walk(seen)
        yield self

    def walk_with_parent(self, path: list["Instruction"] = None):
        """Like walk, but includes the parent node."""
        parent = path[-1] if path else None
        path = (path or []) + [self]
        yield self, parent
        for child in self.children:
            if child not in path:
                yield from child.walk_with_parent(path)


@dataclass(repr=False, slots=True)
class InstructionTree:
    """A tree of instructions, may contain cycles."""

    nodes: dict[uuid.UUID, Instruction] = field(default_factory=dict)

    @property
    def roots(self) -> list[Instruction]:
        """Every node that has no parent (not the same as having no children)."""
        children_ids = set()
        for node in self.nodes.values():
            for child in node.children:
                children_ids.add(child.id)
        root_ids = self.nodes.keys() - children_ids
        return [self.nodes[id] for id in root_ids]

    def walk(self, seen: list[Instruction] = None):
        """Walks the instruction tree (depth-first), ignoring cycles."""
        seen = seen if seen is not None else []
        for node in self.roots:
            yield from node.walk(seen)

    def walk_with_parent(self):
        """Like walk but also get the parent node."""
        parent_by_node = {}
        for node in self.nodes.values():
            for child in node.children:
                parent_by_node[child.id] = node
        for node in self.walk():
            yield node, parent_by_node.get(node.id)


def map_instruction(
    node: InterpSymbol | TypeNode,
    tree: InstructionTree,
    op: InstructionOp = None,
    filter: Callable[[InterpSymbol | TypeNode], bool] = None,
) -> Instruction | None:
    """
    Maps out the instruction tree starting from the given symbol.
    If nodes are already present, they are skipped (including the given node).
    """

    if filter is not None and not filter(node):
        return None

    if node.id in tree.nodes and isinstance(tree.nodes[node.id].node, InterpSymbol):
        # we can overwrite the node if it's not a statement
        # (e.g. type nodes and real Types share the same id)
        return tree.nodes[node.id]
    # if this is a reference, walk the referenced symbol directly (can only be definition for now)
    if node.reference is not None and node.reference != node:
        if not isinstance(node.reference, (InterpSymbol, TypeNode)):
            raise ValueError(f"expected reference to be a symbol: {node}")
        pseudo_link = Instruction(
            op=InstructionOp.Pseudo,
            node=node,
            id=node.id,
        )
        reference = map_instruction(node.reference, tree, filter=filter)
        pseudo_link.children.append(reference)
        return pseudo_link

    if op is None:
        # if not explicitly given, figure out instruction type from symbol
        # yeah this kind of feels like it should be in the symbol/language, see :InstructionOps
        if isinstance(node, Field):
            op = InstructionOp.TypeDefinition
        elif node.symbol_type == SymbolType.BUILD:
            op = InstructionOp.BuildDefinition
        elif node.symbol_type == SymbolType.TYPE:
            op = InstructionOp.TypeDefinition
        elif node.symbol_type == SymbolType.TASK:
            op = InstructionOp.TaskDefinition
        elif node.symbol_type == SymbolType.EXPECTATION:
            op = InstructionOp.ExpectationDefinition
        elif node.symbol_type == SymbolType.DATA:
            if node.modifier in (StatementModifier.LIKE, StatementModifier.UNLIKE):
                op = InstructionOp.SampleData
            else:
                op = InstructionOp.DataDefinition
        elif node.symbol_type == SymbolType.CODE:
            if node.modifier in (StatementModifier.LIKE, StatementModifier.UNLIKE):
                op = InstructionOp.SampleCode
            elif node.modifier == StatementModifier.CHECK:
                op = InstructionOp.CheckCode
            else:
                op = InstructionOp.CodeDefinition
        elif node.symbol_type in (SymbolType.MODEL, SymbolType.REQUIREMENT):
            op = InstructionOp.Pseudo
        else:
            raise ValueError(f"unexpected symbol {node}")

    instruction = Instruction(op=op, node=node, id=node.id)
    tree.nodes[node.id] = instruction

    def _map_child(child: InterpSymbol, op: InstructionOp = None):
        child_instruction = map_instruction(child, tree, op=op, filter=filter)
        if child_instruction is not None:
            instruction.children.append(child_instruction)

    if isinstance(node, Build):
        for task in node.tasks:
            _map_child(task, tree)
        for model in node.models:
            _map_child(model, tree)

    if isinstance(node, Data):
        for record in node.records:
            # this will have to change later, see :NaiveTreeTracking
            tree.nodes[record.id] = Instruction(
                op=InstructionOp.RecordDefinition, node=record, id=record.id
            )
            instruction.children.append(tree.nodes[record.id])

    # track types and their sub-symbols
    if isinstance(node, TypeContent):
        for type_node in node.fields:
            # this will have to change later, see :NaiveTreeTracking
            if type_node.reference is not None:
                if type_node.reference is None or isinstance(type_node.reference, uuid.UUID):
                    continue  # ignore unresolved references
                elif not isinstance(type_node.reference, Type):
                    raise RuntimeError(f"type node references must be imputed: {type_node}")
                type_node = type_node.reference
            _map_child(type_node)

    # context
    if isinstance(node, Code):
        for context_symbol in node.context.values():
            _map_child(context_symbol)

    # walk expectations
    if isinstance(node, (Task, Expectation, Type)):
        for expectation in node.expectations:
            _map_child(expectation)

    # walk task steps & implementation
    if isinstance(node, Task):
        for step in node.steps:
            _map_child(step, op=InstructionOp.TaskStep)

    return instruction


def instruction_tree_from_symbol(
    symbol: InterpSymbol, tree: InstructionTree = None
) -> tuple[Instruction, InstructionTree]:
    """Build a tree of instructions from a symbol and its referenced symbols (and sub-symbols)."""
    tree = tree or InstructionTree(nodes={})
    root = map_instruction(symbol, tree)
    return root, tree


def instruction_tree_from_module(
    module: ModuleIndex, *, exclude_generated: bool, tree: InstructionTree = None
) -> InstructionTree:
    """Build a tree of instructions from a module and its referenced symbols (and sub-symbols)."""
    tree = tree or InstructionTree(nodes={})
    for symbol in module.symbols.values():
        if exclude_generated and symbol.is_generated:
            continue
        # ignore pseudo instructions
        map_instruction(symbol, tree)
    return tree


def source(func):
    return dataclass(repr=False, slots=True)(func)


def anonymous_dataset(type: Type, n_records: int = 0) -> Data:
    order_keys = generate_n_keys_between(None, None, n_records)
    records = [Record(order_key=order_key, data={}) for order_key in order_keys]
    return Data(
        name="",
        tag=type.tag,
        fields=type.fields,
        description="",
        records=records,
        language="jsonl",
    )


@source
class SampleSource:
    def __call__(self) -> Data:
        raise NotImplementedError


@source
class SampleDatasetRandom(SampleSource):
    """Samples the given dataset"""

    source_dataset: Data
    count: int
    seed: int

    def __call__(self) -> Data:
        rng = random.Random(self.seed)
        target_dataset = anonymous_dataset(self.source_dataset.type, self.count)
        n_records = len(self.source_dataset.records)
        sample_indices = rng.sample(range(n_records), min(n_records, self.count))
        for target_i, source_i in enumerate(sample_indices):
            source_data = self.source_dataset.records[source_i].data
            # not sure where to unkey data.. or should we work with dataset instances here?
            target_dataset.records[target_i].data = self.source_dataset.type.unkey(source_data)
        return target_dataset


@source
class SampleFabricateRandom(SampleSource):
    """
    Generates a dataset of the given type by fabricating values
    TODO @Feature: fabricated values are static, use directed probing strategy!
    """

    type: Type
    count: int

    def __call__(self) -> Data:
        target_dataset = anonymous_dataset(self.type, self.count)
        for i in range(self.count):
            target_dataset.records[i].data = fabricate_value(self.type)
        return target_dataset


SAMPLE_BY_TYPE_HINT = {
    TypeHint.UUID: str(uuid.uuid4()),
    TypeHint.NAME: "Max Mustermann",
    TypeHint.EMAIL: "florian@symbolx.com",
    TypeHint.PHONE: "+49 123 456 789",
    TypeHint.URL: "https://symbolx.com",
    TypeHint.KEY: "sk_test_1234567890",
    TypeHint.DATE: "2023-01-01",
    TypeHint.DATETIME: "2023-01-01T10:30:45",
    TypeHint.TIME: "02:08:00",
    TypeHint.RATING: 3,
}


def fabricate_value(type: TypeNode, skip_array: bool = False, is_output: bool = None) -> Any:
    """Synthesizes a value of the given type with fake fields."""
    if type.flags & TypeFlag.IsArray and not skip_array:
        return [fabricate_value(type, skip_array=True)]
    if SAMPLE_BY_TYPE_HINT.get(type.hint) is not None:
        return SAMPLE_BY_TYPE_HINT[type.hint]
    elif type.tag == TypeTag.STRING:
        return "lorem ipsum"
    elif type.tag == TypeTag.NUMBER:
        return 42
    elif type.tag == TypeTag.BOOLEAN:
        return False
    elif type.tag == TypeTag.ENUM:
        if len(type.fields) == 0:
            return None
        return type.fields[0].value
    elif type.tag == TypeTag.STRUCT or type.tag == TypeTag.FUNCTION:
        return {
            subtype.name: fabricate_value(subtype)
            for subtype in type.fields
            if is_output is None or bool(subtype.flags & TypeFlag.IsOutput) == is_output
        }
    elif type.tag == TypeTag.UNION:
        return fabricate_value(type.fields[0])
    elif type.tag == TypeTag.NULL:
        return None
    elif type.tag == TypeTag.LITERAL:
        return type.value
    elif type.tag == TypeTag.ANY:
        return 42  # not sure what to do here
    else:
        raise RuntimeError(f"unexpected type {type.tag}")
