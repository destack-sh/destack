from __future__ import annotations

import enum
import random
from collections import defaultdict
from functools import cached_property
from itertools import chain
from typing import AsyncIterable, Iterable, Mapping, Optional, Union
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from attr import dataclass
from django.db.models import QuerySet

from bench.executor import Executor
from bench.models import Dataset, Instruction, Model, Organization, SymbolType
from bench.models.compilation import Compilation
from bench.models.instruction import InstructionParameterType, InstructionScope
from bench.models.task import Expectation, Task
from bench.utils.record import RecordBatch, RecordList

logger = structlog.get_logger(__name__)


async def _acollect(collectable: QuerySet | AsyncIterable | Dataset) -> list:
    items = []
    async for item in collectable:
        items.append(item)
    return items


def get_backend_model(backend: str) -> Model:
    """
    Gets the backend model from a backends library where backend=owner/model
    """
    owner_slug, model_name = backend.split("/")
    organization = Organization.objects.get(slug=owner_slug)
    stdlib = organization.projects.get(slug="stdlib")
    stdlib_v = stdlib.head
    if stdlib_v is None:
        raise ValueError(f"backends library {stdlib} has no head")
    model = stdlib_v.files.get(type=SymbolType.MODEL, name=model_name).model
    if model is None:
        raise ValueError(f"backends library {stdlib_v} has no model {model_name}")
    return model


class StatementType(enum.Enum):
    """
    The type of expectation defines its semantics.
    """

    GENERATE = "generate"
    TRANSFORM = "transform"
    VERIFY = "verify"


@dataclass
class ExamplesStatement:
    expectation: Expectation
    examples_dataset: Dataset
    type: StatementType = StatementType.GENERATE


@dataclass
class InstructionStatement:
    expectation: Expectation
    instruction: Instruction
    type: Optional[StatementType]


Statement = Union[ExamplesStatement, InstructionStatement]


@dataclass(repr=False)
class TaskDefinition:
    expectations: list[Expectation]
    statements: dict[UUID, list[Statement]]  # by expectation id
    examples: dict[UUID, RecordBatch]  # by dataset id
    parent: Optional[TaskDefinition]
    children: dict[UUID, TaskDefinition]  # by task id
    template_implementation: Optional[Instruction]
    optimal_backend: Optional[Model]
    task: Task

    def __str__(self):
        return f"TaskDefinition({self.task})"

    def __repr__(self):
        return f"TaskDefinition({self.task})"

    @property
    def id(self):
        return self.task.id

    @property
    def name(self):
        return self.task.name

    @property
    def input_keys(self) -> set[str]:
        return self.task.schema["input"].keys()

    @property
    def output_keys(self) -> set[str]:
        return self.task.schema["output"].keys()

    def walk_tree_dfs(self) -> Iterable[TaskDefinition]:
        yield self
        for child in self.children.values():
            yield from child.walk_tree_dfs()

    def walk_tree_bfs(self) -> Iterable[TaskDefinition]:
        queue = [self]
        while queue:
            task = queue.pop(0)
            yield task
            queue.extend(task.children.values())

    @cached_property
    def expectations_by_id(self) -> dict[UUID, Expectation]:
        return {e.id: e for e in self.expectations}

    @cached_property
    def instruction_statements(self) -> list[tuple[UUID, InstructionStatement]]:
        return [
            (expectation_id, statement)
            for expectation_id, statements in self.statements.items()
            for statement in statements
            if isinstance(statement, InstructionStatement)
        ]

    @staticmethod
    async def _collect_examples(statements: Iterable[Statement]) -> Mapping[UUID, RecordBatch]:
        examples: dict[UUID, RecordBatch] = {}
        for statement in statements:
            if not isinstance(statement, ExamplesStatement):
                continue
            examples_records = await _acollect(statement.examples_dataset)
            examples[statement.expectation.id] = RecordList(examples_records)
        return examples

    @classmethod
    async def _from_task_rec(cls, task: Task, parent: Optional[TaskDefinition]) -> TaskDefinition:
        expectations = await _acollect(
            task.expectations.all().prefetch_related("examples_datasets", "instructions")
        )

        # collect and cache statements (examples/instructions)
        statements: dict = defaultdict(list)
        for expectation in expectations:
            async for instruction in expectation.instructions.all():
                # type isn't known yet?
                instruction_statement = InstructionStatement(
                    expectation=expectation, instruction=instruction, type=None
                )
                statements[expectation.id].append(instruction_statement)
            async for examples in expectation.examples_datasets.all():
                examples_statement = ExamplesStatement(
                    expectation=expectation, examples_dataset=examples, type=StatementType.GENERATE
                )
                statements[expectation.id].append(examples_statement)
        examples = await cls._collect_examples(chain(*statements.values()))

        # build task definition and recurse
        self = cls(
            expectations=expectations,
            statements=statements,
            examples=examples,
            template_implementation=task.template_implementation,
            optimal_backend=None,
            parent=parent,
            children={},
            task=task,
        )
        # TODO @Performance: re-use caches for instructions and datasets across tree
        async for child in task.children.all():
            self.children[child.id] = await cls._from_task_rec(child, parent=self)

        return self

    @classmethod
    async def from_task(cls, task: Task) -> TaskDefinition:
        """
        Collects all the information needed to compile a task into a definition.

        As tasks can be nested, this method is recursive.
        """
        return await cls._from_task_rec(task, parent=None)


@dataclass
class CompilerOptions:
    optimize_task: bool
    optimize_instruction: bool


class Compiler:
    def __init__(self, executor: Executor):
        self.executor = executor
        # TODO @Cleanup: make compiler backend model configurable?
        self.compiler_model = get_backend_model("openai/text-davinci-002")

    async def compile_task(
        self, task: Task, backends: list[Model], options: CompilerOptions
    ) -> tuple[Task, Instruction]:
        """
        Compiles a task into an executable instruction (may contain other instructions).

        General compile:
        1. Lay out task tree (tasks may have recursive subtasks)
        2. Optimize task tree for backends and options
           - Expand, refactor and merge subtasks for backend
           - Use backend statistics for task heuristics, choosing optimal backends
           - May build (partial) instructions to update/gather statistics
        3. Build parallel instruction tree (map tasks to instructions)
           Respect instruction templates where defined.
           - Replace task references with compiled instructions
           - Build model instructions from task definitions without templates
           - Use default instructions for parent tasks
           - Copy instructions if explicitly defined (and requested)
        4. Optimize instruction tree for backends and options
        """
        logger.info("compile.task.start", task=task)

        # lay out the task tree
        logger.info("compile.task.layout.start", task=task)
        task_def = await TaskDefinition.from_task(task)
        logger.info("compile.task.layout.done", task=task, task_def=task_def)

        if options.optimize_task:
            # optimize the task tree
            logger.info("compile.task.optimize.start", task=task)
            task_def = await self._optimize_task(task_def, backends)
            logger.info("compile.task.optimize.done", task=task, optimized=task_def.task)
        else:
            # naively set optimal backend for all tasks
            # TODO @Performance: use proper heuristics to decide optimal backend
            if len(backends) > 1:
                raise NotImplementedError("cannot choose backends yet")
            for task_def in task_def.walk_tree_dfs():
                task_def.optimal_backend = backends[0]

        # build the instruction tree
        logger.info("compile.instruct.build.start", task=task_def.task, original=task)
        main_instruction = await self._build_instruction(task_def)
        logger.info("compile.instruct.build.done", task=task_def.task, original=task)

        if options.optimize_instruction:
            # optimize the instruction tree
            logger.info("compile.instruct.optimize.start", task=task_def.task, original=task)
            main_instruction = await self._optimize_instruction(main_instruction, backends)
            logger.info(
                "compile.instruct.optimize.done",
                task_def.task,
                original=task,
                main=main_instruction,
            )

        logger.info("compile.task.done", task=task, optimized=task_def.task, main=main_instruction)
        return task_def.task, main_instruction

    async def _optimize_task(
        self, task_def: TaskDefinition, backends: list[Model]
    ) -> TaskDefinition:
        """
        Optimizes the task tree for backends and options.
        """
        # TODO @Feature @Performance: optimize task tree
        return task_def

    async def _optimize_instruction(
        self, instruction: Instruction, backends: list[Model]
    ) -> Instruction:
        """
        Optimizes the instruction tree for backends and options.
        """
        # TODO @Feature @Performance: optimize instruction tree
        return instruction

    async def _build_instruction(self, task_def: TaskDefinition) -> Instruction:
        """
        Builds an instruction from a task definition.

        Basic model instruction compilation:
        (ignoring subtasks and source instruction)
        1. Render explanation descriptions into basic task description.
        2. Collect and render examples from expectations and examples.
        3. Convert task descriptions and examples to backend model format.
        4. Build prompt and bake into model instruction.
        """

        # 1. render expectation statements
        expect_descriptions = [expect.description for expect in task_def.expectations]
        # (naive render: join all into one string)
        task_description = "\n".join(expect_descriptions)

        # 2. render expectations into examples
        # 2.1 collect static examples
        static_examples: list[dict] = list(chain(*task_def.examples.values()))

        # 2.2 determine expectation type for instruction
        for expect_id, instruction_statement in task_def.instruction_statements:
            # TODO @Feature: determine expectation type properly (resolve instruction def)
            if "verif" in instruction_statement.instruction.name:
                instruction_statement.type = StatementType.VERIFY
            else:
                instruction_statement.type = StatementType.TRANSFORM

        # 2.3 collect dynamic examples from expectations
        # (naive implementation: should be done in parallel and pick optimal examples)
        dynamic_examples: list[dict] = []
        for expect_id, instruction_statement in task_def.instruction_statements:
            expectation = task_def.expectations_by_id[expect_id]
            # select relevant examples for statement
            # (naive implementation: random sample)
            local_random = random.Random(expectation.description.encode())
            relevant_examples = local_random.sample(static_examples, 3)

            if instruction_statement.type == StatementType.TRANSFORM:
                for example in relevant_examples:
                    transformed = await self.executor.run(
                        instruction_statement.instruction, arguments={"example": example}
                    )
                    if isinstance(transformed, dict):
                        dynamic_examples.append(transformed)
                    elif isinstance(transformed, list):
                        # (naive implementation: use all transformed examples)
                        for transformed_example in transformed:
                            dynamic_examples.append(transformed_example)
                    else:
                        raise ValueError(
                            f"unexpected transform statement {instruction_statement.instruction} output: {transformed}"
                        )
            elif instruction_statement.type == StatementType.VERIFY:
                # TODO @Feature: render verify expectations into example instructions
                pass
                # for example in relevant_examples:
                #     good = await self.executor.run(
                #         instruction_statement.instruction,
                #         arguments={"example": example, "expectation": expectation},
                #     )
            else:
                raise NotImplementedError(
                    f"{expectation} statement {instruction_statement} not supported"
                )

        compiled_examples = [*static_examples, *dynamic_examples]
        # order compiled examples optimally
        # (naive implementation: random order)
        random.shuffle(compiled_examples)
        compiled_examples_dataset = await Dataset.objects.afrom_list(
            f"compiled_examples_{task_def.id}", compiled_examples
        )
        if compiled_examples_dataset.length == 0:
            raise ValueError(f"no examples for {task_def.task}")
        compiled_examples_keys = compiled_examples_dataset.schema.keys()
        if compiled_examples_keys != {*task_def.input_keys, *task_def.output_keys}:
            raise ValueError(
                f"{task_def.task} compiled examples {compiled_examples_keys} do not match task "
                f"input/output keys {task_def.input_keys, task_def.output_keys}"
            )

        # 3. convert task descriptions and examples to backend model format
        pass  # naive implementation: noop (no conversion)

        # 4. build prompt and bake into model instruction
        # (naive implementation)
        prompt_prefix: str = task_description + "\n"
        prompt_example = "".join(
            f"{key}: {{{key}}}\n" for key in chain(task_def.input_keys, task_def.output_keys)
        )
        prompt_input = "".join(f"{key}: {{{key}}}\n" for key in task_def.input_keys)
        if len(task_def.output_keys) == 1:
            # also add output key prefix if there is only one
            main_output_key = tuple(task_def.output_keys)[0]
            prompt_input = prompt_input + f"{main_output_key}: "

        if not task_def.optimal_backend:
            raise ValueError(
                f"cannot build instruction for {task_def.task}: optimal backend not set"
            )
        llm_instruction = await Instruction.objects.acreate(
            name=task_def.name,
            task=task_def.task,
            scope=InstructionScope.PROGRAM,
            builtin_id="llm_fewshot",
        )
        # add bound arguments
        for argument_name, argument_value in {
            "model": task_def.optimal_backend,
            "prompt_prefix": prompt_prefix,
            "prompt_example": prompt_example,
            "prompt_input": prompt_input,
            "examples": compiled_examples_dataset,
        }.items():
            await llm_instruction.abind_argument(argument_name, argument_value)
        # add parameter for {input} string
        await llm_instruction.aadd_parameter("input", type=InstructionParameterType.JSON)
        return llm_instruction

    async def compile(self, compilation: Compilation) -> None:
        """
        Compile a task into an executable instruction.

        A task has explanations describing the task, expectations defining what should happen and examples showing that.
        A task may define a template instruction which is used to guide the compilation.
        A task may define subtasks, which are compiled recursively and may be folded into the main task.
        """

        logger.info("compile.start", compilation=compilation)
        backends = await _acollect(compilation.backends.all())
        options = CompilerOptions(optimize_task=False, optimize_instruction=False)
        _, main_instruction = await self.compile_task(compilation.task, backends, options)
        compilation.target = main_instruction
        await sync_to_async(compilation.save)()
        logger.info("compile.done", compilation=compilation)
