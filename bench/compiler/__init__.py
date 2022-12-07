from __future__ import annotations

import enum
import random
from collections import defaultdict
from functools import cached_property
from itertools import chain
from typing import AsyncIterable, Iterable, Optional, cast
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from attr import dataclass
from django.db.models import QuerySet

from bench.executor import Executor
from bench.models import (
    Code,
    Dataset,
    File,
    Model,
    ModelInferenceSettings,
    Organization,
    Project,
    ProjectVersion,
    SymbolDefinition,
    SymbolType,
)
from bench.models.code import SymbolParameterType
from bench.models.symbol import SYMBOL_CONTENT_FIELDS
from bench.models.task import Compilation, Expectation, Task
from bench.utils.record import RecordBatch, RecordList
from bench.utils.schema import SchemaElement

logger = structlog.get_logger(__name__)


async def _acollect(collectable: QuerySet | AsyncIterable | Dataset) -> list:
    items = []
    async for item in collectable:
        items.append(item)
    return items


def get_stdlib_model(path: str) -> Model:
    """
    Gets the backend model from a stdlib library where backend=owner/model
    """
    owner_slug, model_name = path.split("/")
    organization = Organization.objects.get(slug=owner_slug)
    stdlib: Project = organization.projects.get(slug="stdlib")
    return stdlib.head_.symbol_definition(model_name, SymbolType.MODEL).model_


def get_stdlib_code(path: str) -> Code:
    """
    Gets the code from a stdlib where path=owner/code
    """
    owner_slug, code_name = path.split("/")
    organization = Organization.objects.get(slug=owner_slug)
    stdlib: Project = organization.projects.get(slug="stdlib")
    return stdlib.head_.symbol_definition(code_name, SymbolType.CODE).code_


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
    def description(self):
        return self.expectation.description

    @property
    def dataset(self) -> Dataset:
        if self.symbol_type != SymbolType.DATASET:
            raise ValueError(f"expectation statement is not a dataset: {self}")
        return cast(Dataset, self.definition.content)

    @property
    def code(self) -> Code:
        if self.symbol_type != SymbolType.CODE:
            raise ValueError(f"expectation statement is not an code: {self}")
        return cast(Code, self.definition.code)

    @cached_property
    def type(self) -> ExpectationStatementType:
        # (naive implementation: guess statement type)
        # TODO @Feature: define expectation statement type in statement
        if self.symbol_type == SymbolType.CODE:
            if self.code.code_function_name is None:
                raise ValueError(f"expectation statement has no code function: {self}")
            if "gen" in self.code.code_function_name:
                return ExpectationStatementType.GENERATE
            elif "verif" in self.code.code_function_name:
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
    template_implementation: Optional[Code]
    optimal_backend: Optional[Model]
    task: Task

    @property
    def definition(self) -> SymbolDefinition:
        return self.task.definition

    def __str__(self):
        return f"TaskData({self.task})"

    def __repr__(self):
        return f"TaskData({self.task})"

    @property
    def id(self):
        return self.task.id

    @property
    def input_schema(self) -> SchemaElement:
        return self.task.input_schema

    @property
    def output_schema(self) -> SchemaElement:
        return self.task.output_schema

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
    def code_statements(self) -> list[tuple[UUID, StatementData]]:
        return [
            (expectation_id, statement)
            for expectation_id, statements in self.statements.items()
            for statement in statements
            if statement.symbol_type == SymbolType.CODE
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
    def _from_task_rec(cls, task: Task, parent: Optional[TaskData]) -> TaskData:
        # collect and cache statements (examples/code)
        expectations: list[Expectation] = []
        statements: dict = defaultdict(list)
        for expectation in task.expectations.all():
            for statement_def in expectation.statements.all().select_related(
                *SYMBOL_CONTENT_FIELDS
            ):
                statement_data = StatementData(expectation=expectation, definition=statement_def)
                statements[expectation.id].append(statement_data)
            expectations.append(expectation)
        examples = cls._collect_examples(chain(*statements.values()))

        # build task data and recurse
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
        for child in task.definition.children.all().select_related("task"):
            if child.task is None:
                continue
            self.children[child.id] = cls._from_task_rec(child.task, parent=self)

        return self

    @classmethod
    def from_task(cls, task: Task) -> TaskData:
        """
        Collects all the information needed to compile a task into a definition.

        As tasks can be nested, this method is recursive.
        """
        return cls._from_task_rec(task, parent=None)


class Compiler:
    """
    Transforms and optimizes a task definition into a set of executable code (incl. arguments).
    """

    def __init__(self, executor: Executor):
        self.executor = executor
        # TODO @Cleanup: make compiler backend model configurable?
        self.compiler_model = get_stdlib_model("openai/text-davinci-003")
        self.get_temperature = get_stdlib_code("symbolx/get_temperature")

    async def compile(self, compilation: Compilation) -> None:
        """
        Compiles a task into an executable code.
        """
        # get data
        project_v = compilation.project_version
        backends: list[Model] = await _acollect(compilation.backends.all())

        # run compile
        main_task, main_code = await self.compile_task(project_v, compilation, backends)

        # save result
        compilation.target_task = main_task
        compilation.target_code = main_code
        await sync_to_async(compilation.save)()

    async def compile_task(
        self,
        project_v: ProjectVersion,
        compilation: Compilation,
        backends: list[Model],
    ) -> tuple[Task, Code]:
        """
        Compiles a task into an executable code.

        General compile:
        1. Lay out task tree (tasks may have recursive subtasks)
        2. Optimize task tree for backends and options
           - Expand, refactor and merge subtasks for backend
           - Use backend statistics for task heuristics, choosing optimal backends
           - May build (partial) code to update/gather statistics
        3. Build parallel code tree (map tasks to code)
           Respect code templates where defined.
           - Replace task references with compiled code
           - Build model code from task definitions without templates
           - Use default code for parent tasks
           - Copy code if explicitly defined (and requested)
        4. Optimize code tree for backends and options
        """
        log = logger.bind(compilation=compilation, task=compilation.task)
        log.info("compile.started")

        # 1. lay out the task tree
        log.info("compile.layout.started")
        task_data = await sync_to_async(TaskData.from_task)(compilation.task)
        log.info("compile.layout.finished")

        # 2. naively set optimal backend for all tasks
        # TODO @Performance: use proper heuristics to decide optimal backend
        if len(backends) > 1:
            raise NotImplementedError("cannot choose backends yet")
        for t in task_data.walk_tree_dfs():
            t.optimal_backend = backends[0]

        # 3. build the code tree
        log.info("compile.build.started", task=task_data.task)
        target_code = await self._build_code(project_v, compilation, task_data)
        log.info("compile.build.finished", target_task=task_data.task, target_code=target_code)

        # 4. no further code optimization yet

        log.info("compile.finished", target_task=task_data, target_code=target_code)
        return task_data.task, target_code

    async def _build_code(
        self, project_v: ProjectVersion, compilation: Compilation, task_data: TaskData
    ) -> Code:
        """
        Generates target code from a task definition.

        Basic model code compilation:
        (ignoring subtasks and source code)
        1. Render explanation descriptions into basic task description.
        2. Collect and render examples from expectations and examples.
        3. Convert task descriptions and examples to backend model format.
        4. Determine optimal settings for backend model.
        5. Build prompt and bake into model code.
        """

        # 1. render expectation statements
        expect_descriptions = [expect.description for expect in task_data.expectations]
        # (naive render: join all into one string)
        task_description = "\n".join(expect_descriptions)

        # 2. render expectations into examples
        compiled_examples = await self._compile_examples(task_data)
        # order compiled examples optimally
        # (naive implementation: random order)
        random.shuffle(compiled_examples)

        genfile = await sync_to_async(self._get_clean_genfile)(project_v, compilation)
        # write examples to file
        compiled_examples_dataset = await sync_to_async(self._gen_llm_examples)(
            genfile, compiled_examples, task_data
        )

        # 3. convert task descriptions and examples to backend model format
        pass  # (naive implementation: noop i.e. no adaptation)

        # 4. determine optimal settings for backend model
        # (naive implementation: guess settings without optimization)
        settings = await self._guess_settings(
            task_data, task_description, compiled_examples_dataset
        )

        # 5. build prompt and bake into model code
        # (naive implementation)
        llm_code = await sync_to_async(self._gen_llm_code)(
            genfile, task_data, task_description, compiled_examples_dataset, settings
        )
        return llm_code

    async def _compile_examples(self, task_data: TaskData) -> list[dict]:
        """
        Compiles examples for the given task.

        Examples are generated using code and examples from expectation statements.
        """

        # 2.1 collect static examples
        # (naive implementation: collect all static examples indiscriminately)
        static_examples: list[dict] = list(chain(*task_data.examples.values()))

        # 2.2 collect dynamic examples from expectations
        # (naive implementation: should be done iteratively & in parallel, picking optimal examples)
        dynamic_examples: list[dict] = []
        for expect_id, statement in task_data.code_statements:
            expectation = task_data.expectations_by_id[expect_id]
            # select relevant examples for statement
            # (naive implementation: random sample)
            local_random = random.Random(expectation.description.encode())
            n_samples = 3

            if statement.type == ExpectationStatementType.TRANSFORM:
                relevant_examples = local_random.sample(static_examples, n_samples)
                for example in relevant_examples:
                    transformed = await self.executor.run(
                        statement.code, arguments={"example": example}
                    )
                    if isinstance(transformed, dict):
                        dynamic_examples.append(transformed)
                    elif isinstance(transformed, list):
                        # (naive implementation: use all transformed examples)
                        for transformed_example in transformed:
                            dynamic_examples.append(transformed_example)
                    # ignore other types of results
            elif statement.type == ExpectationStatementType.GENERATE:
                examples = await self.executor.run(
                    statement.code, arguments={"n_samples": n_samples}
                )
                if isinstance(examples, list):
                    # (naive implementation: use all generated examples)
                    dynamic_examples.extend(examples)
                # ignore other types of results
            elif statement.type == ExpectationStatementType.VERIFY:
                # TODO @Feature: render verify expectations into example code
                pass
            else:
                raise NotImplementedError(f"{expectation} statement {statement} not supported")
        compiled_examples = [*static_examples, *dynamic_examples]
        return compiled_examples

    def _get_clean_genfile(self, project_v: ProjectVersion, compilation: Compilation) -> File:
        source_path = compilation.task.definition.file.path
        file = project_v.create_file_from_path(
            source_path + "." + compilation.name + ".gen", exists_ok=True
        )
        file.definitions.all().delete()
        return file

    def _gen_llm_examples(
        self, genfile: File, compiled_examples: list[dict], task_data: TaskData
    ) -> Dataset:
        dataset = Dataset.objects.from_list(compiled_examples, schema="derive")
        genfile.create_definition("examples", dataset, generated=True)
        task_schema_keys = {*task_data.input_schema.keys, *task_data.output_schema.keys}
        if set(dataset.schema.keys) != task_schema_keys:
            raise ValueError(
                f"{task_data.definition} compiled examples {dataset.schema.keys} do not match "
                f"input/output keys {task_data.input_schema.keys, task_data.output_schema.keys}"
            )
        return dataset

    async def _guess_settings(
        self, task_data: TaskData, task_description: str, examples_dataset: Dataset
    ) -> ModelInferenceSettings:
        # (naive implementation: set only temperature and max_tokens)
        temperature = await self.executor.run(
            self.get_temperature,
            arguments=dict(
                model=self.compiler_model, description=task_description, examples=examples_dataset
            ),
        )
        if not isinstance(temperature, float):
            raise ValueError(
                f"temperature from {self.get_temperature} is not a float: {temperature}"
            )

        examples = await _acollect(examples_dataset)
        # set max tokens to sum of max length of output keys across examples plus 20%
        max_tokens = int(
            sum(
                max(len(example[key]) for example in examples)
                for key in task_data.output_schema.keys
            )
        )

        if task_data.optimal_backend is None:
            raise ValueError(f"task {task_data.definition} has no optimal backend set")
        settings = task_data.optimal_backend.default_settings
        settings.pk = None
        settings.temperature = temperature
        settings.max_tokens = max_tokens
        return settings

    def _gen_llm_code(
        self,
        genfile: File,
        task_data: TaskData,
        task_description: str,
        task_examples: Dataset,
        settings: ModelInferenceSettings,
    ) -> Code:
        if not task_data.optimal_backend:
            raise ValueError(f"cannot build code for {task_data.task}: optimal backend not set")
        # build prompt template
        prompt_prefix: str = task_description + "\n"
        prompt_example = "".join(
            f"{key}: {{{key}}}\n"
            for key in chain(task_data.input_schema.keys, task_data.output_schema.keys)
        )
        prompt_input = "".join(f"{key}: {{{key}}}\n" for key in task_data.input_schema.keys)
        if len(task_data.output_schema.keys) == 1:
            # also add output key prefix if there is only one
            main_output_key = tuple(task_data.output_schema.keys)[0]
            prompt_input = prompt_input + f"{main_output_key}: "

        llm_code = Code(
            task=task_data.task,
            input_schema=task_data.input_schema,
            output_schema=task_data.output_schema,
            builtin_id="llm_fewshot",
        )
        genfile.create_definition(task_data.definition.name, llm_code, generated=True)
        llm_code.bind_arguments(
            model=task_data.optimal_backend.definition,
            prompt_prefix=prompt_prefix,
            prompt_example=prompt_example,
            prompt_input=prompt_input,
            return_structured=True,  # get a dict back
            examples=task_examples.definition,
            settings=settings.as_dict(omit_empty=True),
        )
        # add parameter for input keys
        for key in task_data.input_schema.keys:
            llm_code.add_parameter(key, SymbolParameterType.VALUE)
        return llm_code
