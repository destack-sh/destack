import enum
import random
import uuid
from dataclasses import dataclass, field
from typing import Union

import structlog

from bench.language import ModuleIndex
from bench.language.type import (
    Code,
    Dataset,
    Expectation,
    InterpSymbol,
    Model,
    Record,
    StatementModifier,
    SymbolType,
    Task,
    Type,
    TypeNode,
    TypeTag,
)
from bench.language.typer import fabricate_value
from bench.runtime.run import instantiate, run
from bench.runtime.type import Modality, TextGenerationSettings
from bench.utils.fractional import generate_n_keys_between

logger = structlog.get_logger(__name__)

Expect = Union[Task, Code, Dataset, Expectation]


class InstructionOp(enum.StrEnum):
    """
    The kind of instruction expressed in a symbol (or sub-symbol).
    TODO @Architecture: should instruction ops & nodes be built during parse?
    """

    BuildDefinition = "build_definition"  # pseudo-instruction
    TypeDefinition = "type_definition"
    TaskDefinition = "task_definition"
    TaskStep = "task_step"
    ExpectationDefinition = "expectation_definition"
    DataDefinition = "data_definition"
    CodeDefinition = "code_definition"
    SampleData = "sample_data"
    SampleCode = "sample_code"
    EvaluateCode = "evaluate_code"
    CheckCode = "check_code"


@dataclass(repr=False, slots=True)
class InstructionNode:
    op: InstructionOp
    node: InterpSymbol | TypeNode | Record
    id: uuid.UUID
    children: list["InstructionNode"] = field(default_factory=list)

    @property
    def is_reference(self) -> bool:
        return isinstance(self.node, InterpSymbol) and self.node.reference is not None

    def walk(self, path: list["InstructionNode"] = None):
        """Walks the instruction tree (depth-first), ignoring cycles."""
        path = (path or []) + [self]
        yield self
        for child in self.children:
            if child not in path:  # break cycles (allowed)
                yield from child.walk(path)


@dataclass(repr=False, slots=True)
class InstructionTree:
    """A tree of instructions, may contain cycles."""

    nodes: dict[uuid.UUID, InstructionNode] = field(default_factory=dict)

    @property
    def roots(self) -> list[InstructionNode]:
        return [node for node in self.nodes.values() if not node.children]

    def walk(self, path: list[InstructionNode] = None) -> None:
        """Walks the instruction tree (depth-first), ignoring cycles."""
        path = path or []
        for node in self.roots:
            yield from node.walk(path)


def map_instruction_node(
    tree: InstructionTree, symbol: InterpSymbol, op: InstructionOp = None
) -> InstructionNode:
    """
    Maps out the instruction tree starting from the given symbol.
    If nodes are already present, they are skipped (including the given node).
    """
    # nocheckin, build instruction tree from symbol

    if symbol.id in tree.nodes and isinstance(tree.nodes[symbol.id].node, InterpSymbol):
        # we can overwrite the node if it's not a statement
        # (e.g. type nodes and real Types share the same id)
        return tree.nodes[symbol.id]
    # if this is a reference, walk the referenced symbol directly (can only be definition for now)
    if symbol.reference is not None and symbol.reference != symbol:
        map_instruction_node(tree, symbol.reference)
        return tree.nodes[symbol.reference.id]

    if op is None:
        # if not explicitly given, figure out instruction type from symbol
        if symbol.symbol_type == SymbolType.BUILD:
            op = InstructionOp.BuildDefinition
        elif symbol.symbol_type == SymbolType.TYPE:
            op = InstructionOp.TypeDefinition
        elif symbol.symbol_type == SymbolType.TASK:
            op = InstructionOp.TaskDefinition
        elif symbol.symbol_type == SymbolType.EXPECTATION:
            op = InstructionOp.ExpectationDefinition
        elif symbol.symbol_type == SymbolType.DATA:
            if symbol.modifier in (StatementModifier.LIKE, StatementModifier.UNLIKE):
                op = InstructionOp.SampleData
            else:
                op = InstructionOp.DataDefinition
        elif symbol.symbol_type == SymbolType.CODE:
            if symbol.modifier in (StatementModifier.LIKE, StatementModifier.UNLIKE):
                op = InstructionOp.SampleCode
            elif symbol.modifier == StatementModifier.CHECK:
                op = InstructionOp.CheckCode
            else:
                op = InstructionOp.CodeDefinition
        else:
            raise ValueError(f"unexpected symbol {symbol}")

    node = InstructionNode(op=op, node=symbol, id=symbol.id)
    tree.nodes[symbol.id] = node

    if isinstance(symbol, Dataset):
        for record in symbol.records:
            # this will have to change later, see :NaiveTreeTracking
            tree.nodes[record.id] = InstructionNode(
                op=InstructionOp.DataDefinition, node=record, id=record.id
            )
            node.children.append(tree.nodes[record.id])

    # track types and their subsymbols
    if isinstance(symbol, (Task, Code, Type, Dataset)):
        if isinstance(symbol, Type):
            type = symbol
        else:
            type = symbol.type
        for type_node in type.walk():
            if isinstance(type_node, InterpSymbol):
                child = map_instruction_node(tree, type_node)
                node.children.append(child)
            elif type_node.id not in tree.nodes:  # id may be re-used for Type, prefer symbol node
                # this will have to change later, see :NaiveTreeTracking
                if type_node.source_reference is not None:
                    if not isinstance(type_node.reference, Type):
                        raise RuntimeError(f"type node references must be imputed: {type_node}")
                    child = map_instruction_node(tree, type_node.reference)
                    node.children.append(child)
                else:
                    child = InstructionNode(
                        op=InstructionOp.TypeDefinition, node=type_node, id=type_node.id
                    )
                    tree.nodes[type_node.id] = child
                    node.children.append(child)

    # context symbols
    for context_symbol in symbol.context.values():
        child = map_instruction_node(tree, context_symbol)
        node.children.append(child)

    # walk expectations
    if isinstance(symbol, (Task, Expectation, Type)):
        for expectation in symbol.expectations:
            child = map_instruction_node(tree, expectation)
            node.children.append(child)

    # walk task steps & implementation
    if isinstance(symbol, Task):
        for step in symbol.steps:
            child = map_instruction_node(tree, step, op=InstructionOp.TaskStep)
            node.children.append(child)

    return node


def instruction_tree_from_symbol(
    symbol: InterpSymbol, tree: InstructionTree = None
) -> InstructionTree:
    """Build a tree of instructions from a symbol and its referenced symbols (and sub-symbols)."""
    tree = tree or InstructionTree(nodes={})
    map_instruction_node(tree, symbol)
    return tree


def instruction_tree_from_module(
    module: ModuleIndex, tree: InstructionTree = None
) -> InstructionTree:
    """Build a tree of instructions from a module and its referenced symbols (and sub-symbols)."""
    tree = tree or InstructionTree(nodes={})
    for symbol in module.symbols.values():
        map_instruction_node(tree, symbol)
    return tree


def source(func):
    return dataclass(repr=False, slots=True)(func)


def anonymous_dataset(type: Type, n_records: int = 0) -> Dataset:
    order_keys = generate_n_keys_between(None, None, n_records)
    records = [Record(order_key=order_key, data={}) for order_key in order_keys]
    return Dataset(
        name="", type=type, type_node=type, description="", records=records, language="jsonl"
    )


@source
class SampleSource:
    async def __call__(self) -> Dataset:
        raise NotImplementedError


@source
class SampleSourceDataset(SampleSource):
    """Samples the given dataset"""

    source_dataset: Dataset
    count: int
    seed: int

    async def __call__(self) -> Dataset:
        target_dataset = anonymous_dataset(self.source_dataset.type, self.count)
        sample_indices = random.sample(range(len(self.source_dataset.records)), self.count)
        for i in sample_indices:
            target_dataset.records[i].data = self.source_dataset.records[i].data
        return target_dataset


@source
class SampleSourceFabricator(SampleSource):
    """
    Generates a dataset of the given type by fabricating values
    TODO @Feature: fabricated values are static, use directed probing strategy!
    """

    type: Type
    count: int
    seed: int

    async def __call__(self) -> Dataset:
        target_dataset = anonymous_dataset(self.type, self.count)
        for i in range(self.count):
            target_dataset.records[i].data = fabricate_value(self.type)
        return target_dataset


@source
class SampleSourceGenerator(SampleSource):
    """Generates a dataset of the given type using a model"""

    type: Type
    model: Model
    count: int
    seed: int

    async def __call__(self) -> Dataset:
        from bench.runtime.build import (  # prevent circular import
            TaskPlan,
            XEmitInput,
            XEmitOutput,
            XEmitSettings,
            XEmitSystem,
            XEmitTask,
            XEmitTypeExplanation,
            XEmitTypeSample,
            do_build_task_plan,
        )

        # manually build the task plan (later this will be in symbolx/bench (?))
        output_type = self.type.deepcopy(keep_id=False)
        output_type.name = "output"
        generation_task_type = Type(
            name="generate examples",
            tag=TypeTag.FUNCTION,
            children=[
                TypeNode(
                    name="input",
                    tag=TypeTag.STRUCT,
                    children=[TypeNode(name="count", tag=TypeTag.NUMBER)],
                ),
                TypeNode(name="output", tag=TypeTag.ARRAY, children=[output_type]),
            ],
        )
        generation_task = Task(
            name="generate examples",
            type=generation_task_type,
            type_node=generation_task_type,
            description="Generate diverse, useful and instructive examples of the given type",
        )
        plan = TaskPlan(task=generation_task, model=self.model, modality=Modality.GenerateText)
        plan.emit(
            XEmitSystem(),
            XEmitTask(task=generation_task),
            XEmitInput(input_type=generation_task.type.input),
            XEmitSettings(
                # TODO @Build: tune model sample generation settings (and adapt to model context size)
                base_settings=TextGenerationSettings(
                    temperature=0.8, max_tokens=2048, top_p=1.0
                ).__dict__
            ),
            XEmitTypeExplanation(
                type=self.type, type_label="Output", include_descriptions=True, recursive=True
            ),
            XEmitTypeSample(type=generation_task_type.output, type_label="Output"),
            XEmitOutput(output_type=generation_task_type.output, output_label="Output"),
        )
        implementation = await do_build_task_plan(plan)
        implementation.context[self.model.name] = self.model
        implementation_instance = instantiate(implementation)

        generated_samples = await run(implementation_instance, {"count": self.count})
        target_dataset = anonymous_dataset(self.type, self.count)
        for i, sample in enumerate(generated_samples):
            target_dataset.records[i].data = sample
        return target_dataset
