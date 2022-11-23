from __future__ import annotations

import enum
from collections import defaultdict
from itertools import chain
from typing import AsyncIterable, Iterable, Optional, Union
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from attr import dataclass
from django.db.models import QuerySet

from bench.executor import Executor
from bench.models import (
    Dataset,
    Execution,
    ExecutionType,
    Instruction,
    InstructionArgument,
    InstructionParameter,
    Model,
    Organization,
    ProjectFileType,
)
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
    backends_library = organization.projects.get(slug="backends")
    backends_version = backends_library.head
    if backends_version is None:
        raise ValueError(f"backends library {backends_library} has no head")
    model = backends_version.files.get(type=ProjectFileType.MODEL, name=model_name).model
    if model is None:
        raise ValueError(f"backends library {backends_version} has no model {model_name}")
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
    examples: Dataset
    type: StatementType = StatementType.GENERATE


@dataclass
class InstructionStatement:
    expectation: Expectation
    instruction: Instruction
    type: Optional[StatementType]


Statement = Union[ExamplesStatement, InstructionStatement]


@dataclass
class TaskDefinition:
    expectations: list[Expectation]
    statements: dict[UUID, list[Statement]]  # by expectation id
    examples: dict[UUID, RecordBatch]  # by dataset id
    parent: Optional[TaskDefinition]
    children: dict[UUID, TaskDefinition]  # by task id
    template_implementation: Optional[Instruction]
    task: Task

    @property
    def id(self):
        return self.task.id

    @property
    def name(self):
        return self.task.name

    @staticmethod
    async def _collect_examples(statements: Iterable[Statement]) -> dict[UUID, RecordBatch]:
        examples = {}
        for statement in statements:
            if not isinstance(statement, ExamplesStatement):
                continue
            examples_records = await _acollect(statement.examples)
            examples[statement.expectation.id] = RecordList(examples_records)
        return examples

    @classmethod
    async def _from_task_rec(cls, task: Task, parent: Optional[TaskDefinition]) -> TaskDefinition:
        expectations = await _acollect(
            task.expectations.all().select_related("example_datasets", "instructions")
        )

        # collect and cache statements (examples/instructions)
        statements: dict = defaultdict(list)
        for expectation in expectations:
            async for instruction in expectation.instructions.all():
                # type isn't known yet?
                statement = InstructionStatement(
                    expectation=expectation, instruction=instruction, type=None
                )
                statements[expectation.id].append(statement)
            async for examples in expectation.example_datasets.all():
                statement = ExamplesStatement(
                    expectation=expectation, examples=examples, type=StatementType.GENERATE
                )
                statements[expectation.id].append(statement)
        examples = await cls._collect_examples(chain(*statements.values()))

        # build task definition and recurse
        self = cls(
            expectations=expectations,
            statements=statements,
            examples=examples,
            template_implementation=task.template_implementation,
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
           - Use backend statistics for task heuristics
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
        logger.info("compile.task.layout.done", task=task)

        if options.optimize_task:
            # optimize the task tree
            logger.info("compile.task.optimize.start", task=task)
            task_def = await self._optimize_task(task_def, backends)
            logger.info("compile.task.optimize.done", task=task, optimized=task_def.task)

        # build the instruction tree
        logger.info("compile.instruct.build.start", task=task_def.task, original=task)
        main_instruction = await self._build_instruction(task_def, backends)
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

    async def _build_instruction(
        self, task_def: TaskDefinition, backends: list[Model]
    ) -> Instruction:
        """
        Builds an instruction from a task definition.

        Basic model instruction compilation:
        (ignoring subtasks and source instruction)
        0. Collect all expectations, their statements and their recursive dependencies.
        1. Render explanation descriptions into basic task description.
        2. Render more examples from expectations and examples.
        3. Convert task descriptions and examples to backend model format.
        4. Build prompt and bake into model instruction.
        """

        # render expectations into examples
        compiled_examples: Dataset = await Dataset.objects.acreate(
            name=f"{task_def.name}_compiled_examples",
        )

        # render explanations and examples into prompt and apply (super basic)
        prompt_prefix: str = "\n".join(explanation_texts) + "\n"
        examples_keys = rendered_examples[0].keys()
        prompt_example = "\n".join(f"{key}: {{{key}}}" for key in examples_keys) + "\n"

        llm_instruction = await Instruction.objects.acreate(
            name=task_def.name,
            task=task_def,
            scope=InstructionScope.PROGRAM,
            builtin_id="llm_fewshot",
        )
        arguments = {
            "model": backend_model,
            "prompt_prefix": prompt_prefix,
            "prompt_example": prompt_example,
            "prompt_input": "Input: {input}\nOutput: ",
            "examples": compiled_examples,
        }
        # add parameter for {input} string
        await InstructionParameter.objects.acreate(
            instruction=llm_instruction, name="input", type=InstructionParameterType.JSON
        )
        # add bound arguments
        for argument_name, argument_value in arguments.items():
            argument_type = InstructionParameterType.from_obj(argument_value)
            if argument_type == InstructionParameterType.DATASET:
                argument_kwargs = {"dataset": argument_value}
            elif argument_type == InstructionParameterType.MODEL:
                argument_kwargs = {"model": argument_value}
            elif argument_type == InstructionParameterType.JSON:
                argument_kwargs = {"value": argument_value}
            else:
                raise RuntimeError(f"unsupported argument type {argument_type}")

            # create parameter and corresponding argument
            await InstructionParameter.objects.acreate(
                instruction=llm_instruction,
                name=argument_name,
                type=argument_type,
            )
            await InstructionArgument.objects.acreate(
                instruction_bound=llm_instruction,
                name=argument_name,
                type=argument_type,
                **argument_kwargs,
            )
        return llm_instruction

    async def compile(self, compilation: Compilation) -> None:
        """
        Compile a task into an executable instruction.

        A task has explanations describing the task, expectations defining what should happen and examples showing that.
        A task may define a template instruction which is used to guide the compilation.
        A task may define subtasks, which are compiled recursively and may be folded into the main task.
        """

        logger.info("compile.start", compilation=compilation)
        compilation_execution: Execution = await Execution.objects.acreate(
            type=ExecutionType.COMPILATION, compilation=compilation
        )
        async with compilation_execution.capture():
            backends = await _acollect(compilation.backends.all())
            _, main_instruction = await self.compile_task(compilation.task, backends)
            compilation.target = main_instruction
            await sync_to_async(compilation.save)()
        logger.info("compile.done", compilation=compilation)
