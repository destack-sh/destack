from __future__ import annotations

import enum
import random
from collections import defaultdict
from functools import cached_property
from itertools import chain
from typing import AsyncIterable, Iterable, Optional
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from attr import dataclass
from django.db.models import QuerySet

from bench.executor import Executor
from bench.models import (
    Dataset,
    File,
    Instruction,
    Organization,
    ProjectVersion,
    Symbol,
    SymbolDefinition,
    SymbolType,
)
from bench.models.instruction import InstructionParameterType, InstructionScope
from bench.models.task import Expectation, Task
from bench.utils.record import RecordBatch, RecordList

logger = structlog.get_logger(__name__)


async def _acollect(collectable: QuerySet | AsyncIterable | Dataset) -> list:
    items = []
    async for item in collectable:
        items.append(item)
    return items


def get_stdlib_model_def(backend: str) -> SymbolDefinition:
    """
    Gets the backend model from a backends library where backend=owner/model
    """
    owner_slug, model_name = backend.split("/")
    organization = Organization.objects.get(slug=owner_slug)
    stdlib = organization.projects.get(slug="stdlib")
    stdlib_v: Optional[ProjectVersion] = stdlib.head
    if stdlib_v is None:
        raise ValueError(f"library {stdlib} has no head")
    return stdlib_v.symbol_definition(model_name, SymbolType.MODEL)


class ExpectationStatementType(enum.Enum):
    """
    The type of expectation defines its semantics.
    """

    GENERATE = "generate"
    TRANSFORM = "transform"
    VERIFY = "verify"


@dataclass
class StatementData:
    expectation: Expectation
    definition: SymbolDefinition

    @property
    def symbol_type(self):
        return self.definition.type

    @property
    def symbol(self) -> Symbol:
        return self.definition.symbol

    @property
    def description(self):
        return self.expectation.description

    @property
    def dataset(self) -> Dataset:
        if self.symbol_type != SymbolType.DATASET:
            raise ValueError(f"expectation statement is not a dataset: {self}")
        return self.definition.dataset

    @property
    def instruction(self) -> Instruction:
        if self.symbol_type != SymbolType.INSTRUCTION:
            raise ValueError(f"expectation statement is not an instruction: {self}")
        return self.definition.instruction

    @cached_property
    def type(self) -> ExpectationStatementType:
        # (naive implementation: guess statement type)
        # TODO @Feature: define expectation statement type in statement
        if self.symbol_type == SymbolType.INSTRUCTION:
            if "verif" in self.instruction.code_function_name:
                return ExpectationStatementType.VERIFY
            else:
                return ExpectationStatementType.TRANSFORM
        elif self.symbol_type == SymbolType.DATASET:
            return ExpectationStatementType.GENERATE
        else:
            raise NotImplementedError(f"unknown statement type: {self}")


@dataclass(repr=False)
class TaskData:
    expectations: list[Expectation]
    statements: dict[UUID, list[StatementData]]  # by expectation id
    examples: dict[UUID, RecordBatch]  # by dataset id
    parent: Optional[TaskData]
    children: dict[UUID, TaskData]  # by task id
    template_implementation: Optional[Instruction]
    optimal_backend_ref: Optional[Symbol]
    definition: SymbolDefinition

    @property
    def task(self) -> Task:
        return self.definition.task

    @property
    def symbol(self) -> Symbol:
        return self.definition.symbol

    def __str__(self):
        return f"TaskData({self.task})"

    def __repr__(self):
        return f"TaskData({self.task})"

    @property
    def id(self):
        return self.task.id

    @property
    def input_keys(self) -> set[str]:
        return self.task.schema["input"].keys()

    @property
    def output_keys(self) -> set[str]:
        return self.task.schema["output"].keys()

    def walk_tree_dfs(self) -> Iterable[TaskData]:
        yield self
        for child in self.children.values():
            yield from child.walk_tree_dfs()

    def walk_tree_bfs(self) -> Iterable[TaskData]:
        queue = [self]
        while queue:
            task = queue.pop(0)
            yield task
            queue.extend(task.children.values())

    @cached_property
    def expectations_by_id(self) -> dict[UUID, Expectation]:
        return {e.id: e for e in self.expectations}

    @cached_property
    def instruction_statements(self) -> list[tuple[UUID, StatementData]]:
        return [
            (expectation_id, statement)
            for expectation_id, statements in self.statements.items()
            for statement in statements
            if statement.symbol_type == SymbolType.INSTRUCTION
        ]

    @staticmethod
    def _collect_examples(
        statements: Iterable[StatementData],
    ) -> dict[UUID, RecordBatch]:
        examples: dict[UUID, RecordBatch] = {}
        for statement in statements:
            if statement.symbol_type == SymbolType.DATASET_VIEW:
                raise NotImplementedError(f"dataset views are not yet supported: {statement}")
            if statement.symbol_type != SymbolType.DATASET:
                continue
            examples_records = list(statement.dataset)
            examples[statement.expectation.id] = RecordList(examples_records)
        return examples

    @classmethod
    def _from_task_rec(
        cls, project_v: ProjectVersion, task_def: SymbolDefinition, parent: Optional[TaskData]
    ) -> TaskData:
        expectations_refs = task_def.task.expectations.all()

        # collect and cache statements (examples/instructions)
        expectations: list[Expectation] = []
        statements: dict = defaultdict(list)
        for expectation_ref in expectations_refs:
            expectation_def = project_v.resolve_sure(expectation_ref)
            expectation: Expectation = expectation_def.expectation
            for statement in expectation.statements.all():
                statement_def = project_v.resolve_sure(
                    statement, prefetch=["dataset", "instruction", "symbol"]
                )
                statement_data = StatementData(expectation=expectation, definition=statement_def)
                statements[expectation.id].append(statement_data)
            expectations.append(expectation)
        examples = cls._collect_examples(chain(*statements.values()))

        if task_def.task.template_implementation is not None:
            template_implementation = project_v.resolve(task_def.task.template_implementation)
        else:
            template_implementation = None

        # build task data and recurse
        self = cls(
            expectations=expectations,
            statements=statements,
            examples=examples,
            template_implementation=template_implementation,
            optimal_backend_ref=None,
            parent=parent,
            children={},
            definition=task_def,
        )
        for child in task_def.children.all().select_related("symbol"):
            self.children[child.id] = cls._from_task_rec(project_v, child, parent=self)

        return self

    @classmethod
    def from_task(cls, project_v: ProjectVersion, task_def: SymbolDefinition) -> TaskData:
        """
        Collects all the information needed to compile a task into a definition.

        As tasks can be nested, this method is recursive.
        """
        return cls._from_task_rec(project_v, task_def, parent=None)


class Compiler:
    """
    Transforms and optimizes a task definition into a set of executable instructions (incl. arguments).
    """

    def __init__(self, executor: Executor):
        self.executor = executor
        # TODO @Cleanup: make compiler backend model configurable?
        self.compiler_model = get_stdlib_model_def("openai/text-davinci-003").model

    async def compile(
        self, project_v: ProjectVersion, task_ref: Symbol, compilation_name: str
    ) -> None:
        """
        Compiles a task into an executable instruction.
        """

        # get data
        task_def = await project_v.aresolve(task_ref, prefetch=["task", "task__compilations"])
        compilation = await task_def.task.compilations.aget(name=compilation_name)
        backends: list[Symbol] = await _acollect(compilation.backends.all())

        # run compile
        main_task_def, main_instruct_def = await self.compile_task(project_v, task_def, backends)

        # save result
        compilation.output_task = main_task_def.symbol
        compilation.output_instruction = main_instruct_def.symbol
        await sync_to_async(compilation.save)()
        task_def.task.implementation = main_instruct_def.symbol
        await sync_to_async(task_def.task.save)()

    async def compile_task(
        self,
        project_v: ProjectVersion,
        task_def: SymbolDefinition,
        backends_refs: list[Symbol],
    ) -> tuple[SymbolDefinition, SymbolDefinition]:
        """
        Compiles a task into an executable instruction.

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
        logger.info("compile.started", task=task_def)

        # 1. lay out the task tree
        logger.info("compile.layout.started", task=task_def)
        task_data = await sync_to_async(TaskData.from_task)(project_v, task_def)
        logger.info("compile.layout.finished", task=task_def, task_def=task_def)

        # 2. naively set optimal backend for all tasks
        # TODO @Performance: use proper heuristics to decide optimal backend
        if len(backends_refs) > 1:
            raise NotImplementedError("cannot choose backends yet")
        for task_def in task_data.walk_tree_dfs():
            task_data.optimal_backend_ref = backends_refs[0]

        # 3. build the instruction tree
        logger.info("compile.build.started", task=task_data.task, original=task_def)
        main_instruct_def = await self._build_instruction(project_v, task_data)
        logger.info("compile.build.finished", task=task_data.task, original=task_def)

        # 4. no further optimization yet

        logger.info(
            "compile.finished", task=task_data, optimized=task_def.task, main=main_instruct_def
        )
        return task_data.definition, main_instruct_def

    async def _optimize_task(
        self, project_v: ProjectVersion, task_def: TaskData, backends: list[Symbol]
    ) -> TaskData:
        """
        Optimizes the task tree for backends and options.
        """
        # TODO @Feature @Performance: optimize task tree
        return task_def

    async def _optimize_instruction(
        self, project_v: ProjectVersion, instruction_def: SymbolDefinition, backends: list[Symbol]
    ) -> SymbolDefinition:
        """
        Optimizes the instruction tree for backends and options.
        """
        # TODO @Feature @Performance: optimize instruction tree
        return instruction_def

    async def _build_instruction(
        self, project_v: ProjectVersion, task_data: TaskData
    ) -> SymbolDefinition:
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
        expect_descriptions = [expect.description for expect in task_data.expectations]
        # (naive render: join all into one string)
        task_description = "\n".join(expect_descriptions)

        # 2. render expectations into examples
        compiled_examples = await self._compile_examples(project_v, task_data)
        # order compiled examples optimally
        # (naive implementation: random order)
        random.shuffle(compiled_examples)

        genfile = await sync_to_async(self._get_clean_genfile)(project_v, task_data)
        # write examples to file
        compiled_examples_dataset_def = await sync_to_async(self._write_llm_examples)(
            genfile, compiled_examples, task_data
        )

        # 3. convert task descriptions and examples to backend model format
        pass  # (naive implementation: noop i.e. no adaptation)

        # 4. build prompt and bake into model instruction
        # (naive implementation)
        llm_instruction = await sync_to_async(self._build_llm_instruction)(
            task_data, task_description, compiled_examples_dataset_def
        )
        llm_instruction_def = await sync_to_async(genfile.create_definition)(
            task_data.definition.name, llm_instruction
        )

        return llm_instruction_def

    async def _compile_examples(self, project_v: ProjectVersion, task_data: TaskData) -> list[dict]:
        # 2.1 collect static examples
        # (naive implementation collect all static examples indiscriminately)
        static_examples: list[dict] = list(chain(*task_data.examples.values()))

        # 2.2 collect dynamic examples from expectations
        # (naive implementation: should be done iteratively & in parallel, picking optimal examples)
        dynamic_examples: list[dict] = []
        for expect_id, statement in task_data.instruction_statements:
            expectation = task_data.expectations_by_id[expect_id]
            # select relevant examples for statement
            # (naive implementation: random sample)
            local_random = random.Random(expectation.description.encode())

            if statement.type == ExpectationStatementType.TRANSFORM:
                relevant_examples = local_random.sample(static_examples, 3)
                for example in relevant_examples:
                    transformed = await self.executor.run(
                        project_v, statement.symbol, arguments={"example": example}
                    )
                    if isinstance(transformed, dict):
                        dynamic_examples.append(transformed)
                    elif isinstance(transformed, list):
                        # (naive implementation: use all transformed examples)
                        for transformed_example in transformed:
                            dynamic_examples.append(transformed_example)
                    else:
                        raise ValueError(
                            f"unexpected transform statement {statement.definition}"
                            f" instruction {statement.instruction} output: {transformed}"
                        )
            elif statement.type == ExpectationStatementType.VERIFY:
                # TODO @Feature: render verify expectations into example instructions
                pass
            else:
                raise NotImplementedError(f"{expectation} statement {statement} not supported")
        compiled_examples = [*static_examples, *dynamic_examples]
        return compiled_examples

    def _get_clean_genfile(self, project_v: ProjectVersion, task_data: TaskData) -> File:
        file = project_v.create_file_from_path(
            task_data.definition.file.path + ".gen", exists_ok=True
        )
        file.definitions.set([])
        return file

    def _write_llm_examples(
        self, compilation_file: File, compiled_examples: list[dict], task_data: TaskData
    ):
        compiled_examples_dataset = Dataset.objects.from_list(compiled_examples)
        compiled_examples_keys = compiled_examples_dataset.schema.keys()
        if compiled_examples_keys != {*task_data.input_keys, *task_data.output_keys}:
            raise ValueError(
                f"{task_data.definition} compiled examples {compiled_examples_keys} do not match "
                f"input/output keys {task_data.input_keys, task_data.output_keys}"
            )
        compiled_examples_dataset_def = compilation_file.create_definition(
            "examples", compiled_examples_dataset
        )
        return compiled_examples_dataset_def

    def _build_llm_instruction(
        self,
        task_data: TaskData,
        task_description: str,
        task_examples_def: SymbolDefinition,
    ) -> Instruction:
        if not task_data.optimal_backend_ref:
            raise ValueError(
                f"cannot build instruction for {task_data.task}: optimal backend not set"
            )
        prompt_prefix: str = task_description + "\n"
        prompt_example = "".join(
            f"{key}: {{{key}}}\n" for key in chain(task_data.input_keys, task_data.output_keys)
        )
        prompt_input = "".join(f"{key}: {{{key}}}\n" for key in task_data.input_keys)
        if len(task_data.output_keys) == 1:
            # also add output key prefix if there is only one
            main_output_key = tuple(task_data.output_keys)[0]
            prompt_input = prompt_input + f"{main_output_key}: "

        scope = InstructionScope.PROGRAM if task_data.parent is None else InstructionScope.FUNCTION
        llm_instruction = Instruction.objects.create(
            task=task_data.symbol, scope=scope, builtin_id="llm_fewshot"
        )
        llm_instruction.bind_arguments(
            model=task_data.optimal_backend_ref,
            prompt_prefix=prompt_prefix,
            prompt_example=prompt_example,
            prompt_input=prompt_input,
            examples=task_examples_def.symbol,
        )
        # add parameter for {input} string
        llm_instruction.add_parameter("input", type=InstructionParameterType.JSON)
        return llm_instruction
