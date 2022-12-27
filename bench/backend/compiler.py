from __future__ import annotations

import enum
import random
from itertools import chain
from typing import AsyncIterable

import structlog
from asgiref.sync import sync_to_async
from django.db.models import QuerySet

from bench.backend.executor import Executor
from bench.language.types import Compilation, Task
from bench.models import (
    Code,
    Compilation,
    Dataset,
    File,
    Model,
    ModelInferenceSettings,
    Organization,
    Project,
    ProjectVersion,
    SymbolType,
)

logger = structlog.get_logger(__name__)


async def _acollect(collectable: QuerySet | AsyncIterable | Dataset) -> list:
    items = []
    async for item in collectable:
        items.append(item)
    return items


def get_stdlib_model(owner_slug: str, model_name: str) -> Model:
    """
    Gets the backend model from a stdlib library where backend=owner/model
    """
    organization = Organization.objects.get(slug=owner_slug)
    stdlib: Project = organization.projects.get(slug="stdlib")
    return stdlib.head_.statement(file=None, name=model_name, symbol_type=SymbolType.MODEL).model_


def get_stdlib_code(owner_slug: str, code_name) -> Code:
    """
    Gets the code from a stdlib where path=owner/code
    """
    organization = Organization.objects.get(slug=owner_slug)
    stdlib: Project = organization.projects.get(slug="stdlib")
    return stdlib.head_.statement(file=None, name=code_name, symbol_type=SymbolType.CODE).code_


class ExpectationStatementType(enum.Enum):
    """
    The type of expectation defines its semantics.
    """

    GENERATE = "generate"
    TRANSFORM = "transform"
    VERIFY = "verify"


class Compiler:
    """
    Worker-side compiler to transform and optimize statements.
    """

    def __init__(self, executor: Executor):
        self.executor = executor
        # TODO @Cleanup: make compiler backend model configurable?
        self.compiler_model = get_stdlib_model("openai", "text-davinci-003")
        self.get_temperature = get_stdlib_code("symbolx", "get_temperature")

    async def compile(self, compilation: Compilation) -> Compilation:
        # run compile
        log = logger.bind(compilation=compilation, task=compilation.task)
        log.info("compile.started")

        # 1. lay out the task tree
        log.info("compile.layout.started")
        task_data = await sync_to_async(Task.from_task)(compilation.task)
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
        self, project_v: ProjectVersion, compilation: Compilation, task_data: Task
    ) -> Code:
        # 1. render expectation statements (naive render: join all into one string)
        expect_descriptions = [expect.description for expect in task_data.expectations]
        task_description = "\n".join(expect_descriptions)

        # 2. render expectations into examples
        compiled_examples = await self._compile_examples(task_data)
        # order compiled examples optimally (naive implementation: random order)
        random.shuffle(compiled_examples)

        # write examples to file
        compiled_examples_dataset = await sync_to_async(self._gen_llm_examples)(
            genfile, compiled_examples, task_data
        )
        # 3. convert task descriptions and examples to backend model format (naive: noop)
        # 4. determine optimal settings for backend model (naive: guess)
        settings = await self._guess_settings(
            task_data, task_description, compiled_examples_dataset
        )
        # 5. build prompt and bake into statements
        llm_code = await sync_to_async(self._gen_llm_code)(
            genfile, task_data, task_description, compiled_examples_dataset, settings
        )
        return llm_code

    async def _compile_examples(self, task_data: Task) -> list[dict]:
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
                    transformed = await self.executor.resolve_and_run(
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
                examples = await self.executor.resolve_and_run(
                    statement.code, arguments={"n_samples": n_samples}
                )
                if isinstance(examples, list):
                    # (naive implementation: use all compiled examples)
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
        file.symbols.all().delete()
        return file

    def _gen_llm_examples(
        self, genfile: File, compiled_examples: list[dict], task_data: Task
    ) -> Dataset:
        dataset = Dataset.objects.from_list(compiled_examples)
        genfile.define_symbol("examples", dataset, compiled=True)
        task_schema_keys = {*task_data.input_schema.keys, *task_data.output_schema.keys}
        if set(dataset.schema.keys) != task_schema_keys:
            raise ValueError(
                f"{task_data.symbol} compiled examples {dataset.schema.keys} do not match "
                f"input/output keys {task_data.input_schema.keys, task_data.output_schema.keys}"
            )
        return dataset

    async def _guess_settings(
        self, task_data: Task, task_description: str, examples_dataset: Dataset
    ) -> ModelInferenceSettings:
        # (naive implementation: set only temperature and max_tokens)
        temperature = await self.executor.resolve_and_run(
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
        task_data: Task,
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

        llm_code = Code.objects.create(
            input_schema=task_data.input_schema,
            output_schema=task_data.output_schema,
            builtin_id="llm_fewshot",
        )
        llm_code.tasks.add(task_data.task)
        genfile.define_symbol(task_data.definition.name, llm_code, compiled=True)
        llm_code.definition.bind_arguments(
            model=task_data.optimal_backend.definition,
            prompt_prefix=prompt_prefix,
            prompt_example=prompt_example,
            prompt_input=prompt_input,
            return_structured=True,
            examples=task_examples.definition,
            settings=settings.as_dict(omit_empty=True),
        )
        return llm_code
