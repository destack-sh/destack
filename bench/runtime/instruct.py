from __future__ import annotations

import enum
import random
import uuid
from dataclasses import dataclass, field
from typing import Any, Union

import structlog

from bench.language import ModuleIndex
from bench.language.type import (
    Build,
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
from bench.runtime.run import instantiate, run
from bench.runtime.tracing import tracer_blocker
from bench.runtime.type import Modality, TextGenerationSettings
from bench.utils.fractional import generate_n_keys_between

logger = structlog.get_logger(__name__)

Expect = Union[Task, Code, Dataset, Expectation]

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
    node: InterpSymbol | TypeNode, tree: InstructionTree, op: InstructionOp = None
) -> Instruction:
    """
    Maps out the instruction tree starting from the given symbol.
    If nodes are already present, they are skipped (including the given node).
    """

    if node.id in tree.nodes and isinstance(tree.nodes[node.id].node, InterpSymbol):
        # we can overwrite the node if it's not a statement
        # (e.g. type nodes and real Types share the same id)
        return tree.nodes[node.id]
    # if this is a reference, walk the referenced symbol directly (can only be definition for now)
    if node.reference is not None and node.reference != node:
        pseudo_link = Instruction(
            op=InstructionOp.Pseudo,
            node=node,
            id=node.id,
        )
        reference = map_instruction(node.reference, tree)
        pseudo_link.children.append(reference)
        return pseudo_link

    if op is None:
        # if not explicitly given, figure out instruction type from symbol
        # yeah this kind of feels like it should be in the symbol/language, see :InstructionOps
        if isinstance(node, TypeNode):
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
        elif node.symbol_type in (SymbolType.MODEL, SymbolType.REQUIREMENT, SymbolType.RUNCONFIG):
            op = InstructionOp.Pseudo
        else:
            raise ValueError(f"unexpected symbol {node}")

    instruction = Instruction(op=op, node=node, id=node.id)
    tree.nodes[node.id] = instruction

    if isinstance(node, Build):
        for task in node.tasks:
            child = map_instruction(task, tree)
            instruction.children.append(child)
        for model in node.models:
            child = map_instruction(model, tree)
            instruction.children.append(child)

    if isinstance(node, Dataset):
        for record in node.records:
            # this will have to change later, see :NaiveTreeTracking
            tree.nodes[record.id] = Instruction(
                op=InstructionOp.RecordDefinition, node=record, id=record.id
            )
            instruction.children.append(tree.nodes[record.id])

    # track types and their subsymbols
    if isinstance(node, (TypeNode, Task, Code, Type, Dataset)):
        if isinstance(node, (TypeNode, Type)):
            type = node
        else:
            type = node.type
        # skip type wrapper nodes
        if type.tag == TypeTag.FUNCTION:
            children = [*(type.input.children or [])]
            if type.output.tag != TypeTag.NULL:
                children.append(type.output)
        elif type.tag == TypeTag.ENUM:
            children = type.members
        else:
            children = type.children or []
        for type_node in children:
            # this will have to change later, see :NaiveTreeTracking
            if type_node.source_reference is not None:
                if type_node.reference is None or isinstance(type_node.reference, uuid.UUID):
                    continue  # ignore unresolved references
                elif not isinstance(type_node.reference, Type):
                    raise RuntimeError(f"type node references must be imputed: {type_node}")
                type_node = type_node.reference
            child = map_instruction(type_node, tree)
            instruction.children.append(child)

    # context symbols
    if isinstance(node, InterpSymbol):
        for context_symbol in node.context.values():
            child = map_instruction(context_symbol, tree)
            instruction.children.append(child)

    # walk expectations
    if isinstance(node, (Task, Expectation, Type)):
        for expectation in node.expectations:
            child = map_instruction(expectation, tree)
            instruction.children.append(child)

    # walk task steps & implementation
    if isinstance(node, Task):
        for step in node.steps:
            child = map_instruction(step, tree, op=InstructionOp.TaskStep)
            instruction.children.append(child)

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
class SampleDatasetRandom(SampleSource):
    """Samples the given dataset"""

    source_dataset: Dataset
    count: int
    seed: int

    async def __call__(self) -> Dataset:
        rng = random.Random(self.seed)
        target_dataset = anonymous_dataset(self.source_dataset.type, self.count)
        n_records = len(self.source_dataset.records)
        sample_indices = rng.sample(range(n_records), min(n_records, self.count))
        for target_i, source_i in enumerate(sample_indices):
            target_dataset.records[target_i].data = self.source_dataset.records[source_i].data
        return target_dataset


@source
class SampleFabricateRandom(SampleSource):
    """
    Generates a dataset of the given type by fabricating values
    TODO @Feature: fabricated values are static, use directed probing strategy!
    """

    type: Type
    count: int

    async def __call__(self) -> Dataset:
        target_dataset = anonymous_dataset(self.type, self.count)
        for i in range(self.count):
            target_dataset.records[i].data = fabricate_value(self.type)
        return target_dataset


@source
class SampleGenerateWithModel(SampleSource):
    """Generates a dataset of the given type using a model"""

    task: Task
    type: Type
    model: Model
    count: int
    seed: int

    async def __call__(self) -> Dataset:
        from bench.runtime.build import (  # prevent circular import
            TaskPlan,
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
            description=f"Generate diverse, useful and instructive examples "
            f' for the task "{self.task.name}: {self.task.description}".\n'
            "The examples should illustrate realistic use cases and edge cases.",
        )
        plan = TaskPlan(task=generation_task, model=self.model, modality=Modality.GenerateText)
        plan.emit(
            XEmitSystem(),
            XEmitTask(task=generation_task),
            XEmitTypeExplanation(
                type=self.type, type_label="Output", include_descriptions=True, recursive=True
            ),
            XEmitTypeSample(
                type=generation_task_type.output, type_label="Output (1 array element)"
            ),
            # TODO @Build: tune model sample generation settings (and adapt to model context size)
            XEmitSettings(TextGenerationSettings(temperature=0.9, max_tokens=2048, top_p=1.0)),
            XEmitOutput(
                type=generation_task_type.output,
                type_label=f"Output samples ({self.count} array elements)",
            ),
        )
        implementation = await do_build_task_plan(plan)
        implementation.context[self.model.name] = self.model
        implementation_instance = instantiate(implementation)

        with tracer_blocker():
            generated_samples = await run(implementation_instance, {"count": self.count})
        target_dataset = anonymous_dataset(self.type, len(generated_samples))
        if len(generated_samples) != self.count:
            raise RuntimeError(
                f"expected {self.count} samples, got {len(generated_samples)} samples"
            )
        for i, sample in enumerate(generated_samples):
            target_dataset.records[i].data = sample
        return target_dataset


def fabricate_value(type: TypeNode) -> Any:
    """Synthesizes a value of the given type with fake fields."""
    if type.tag == TypeTag.STRING:
        return "lorem ipsum"
    elif type.tag == TypeTag.NUMBER:
        return 42
    elif type.tag == TypeTag.BOOLEAN:
        return False
    elif type.tag == TypeTag.ARRAY:
        return [fabricate_value(type.children[0])]
    elif type.tag == TypeTag.ENUM:
        return type.members[0].value
    elif type.tag == TypeTag.STRUCT:
        return {subtype.name: fabricate_value(subtype) for subtype in type.children}
    elif type.tag == TypeTag.UNION:
        return fabricate_value(type.children[0])
    elif type.tag == TypeTag.NULL:
        return None
    elif type.tag == TypeTag.LITERAL:
        return type.value
    elif type.tag == TypeTag.ANY:
        return 42  # not sure what to do here
    elif type.tag == TypeTag.FUNCTION:
        return {**fabricate_value(type.input), "output": fabricate_value(type.output)}
    else:
        raise RuntimeError(f"unexpected type {type.tag}")
