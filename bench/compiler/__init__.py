import structlog
from asgiref.sync import sync_to_async
from django.db.models import QuerySet

from bench.executor import Executor
from bench.models import (
    Dataset,
    Instruction,
    InstructionArgument,
    InstructionParameter,
    Model,
    Organization,
    ProjectFileType,
)
from bench.models.compilation import Compilation
from bench.models.instruction import InstructionParameterType, InstructionScope
from bench.models.task import Example, Expectation, ExpectationType, Explanation, Task

logger = structlog.get_logger(__name__)


async def _acollect(qs: QuerySet) -> list:
    items = []
    async for item in qs:
        items.append(item)
    return items


def get_backend_model(backend: str) -> Model:
    """
    Gets the backend model from a backends library where backend=owner/model
    """
    owner, model = backend.split("/")
    organization = Organization.objects.get(slug=owner)
    backends_library = organization.projects.get(slug="backends")
    model = backends_library.head.files.get(type=ProjectFileType.MODEL, name=model).model
    return model


class Compiler:
    def __init__(self, executor: Executor):
        self.executor = executor
        self.compiler_model = get_backend_model("openai/text-davinci-002")

    async def compile_task(self, task: Task, backend_model: Model) -> Instruction:
        """
        Compiles a task into an executable instruction.
        """
        logger.info("compiling task", task=task)
        explanations: list[Explanation] = await _acollect(
            task.explanations.select_related("dataset")
        )
        expectations: list[Expectation] = await _acollect(
            task.expectations.select_related("instruction")
        )
        examples: list[Example] = await _acollect(
            task.examples.select_related("dataset").prefetch_related("dataset__records")
        )

        # render explanations and expectations into prompts
        explanation_texts = []
        for explanation in explanations:
            statements: list[dict] = await _acollect(explanation.dataset)
            for statement in statements:
                # TODO @Feature: render explanations, use dataset view to get fields
                rendered_explanation = statement["text"]
                explanation_texts.append(rendered_explanation)
        expectation_texts = []
        for expectation in expectations:
            if expectation.text is not None:
                expectation_texts.append(expectation.text)

        # render expectations into examples
        compiled_examples: Dataset = await Dataset.objects.acreate(
            name=f"{task.name}_compiled_examples",
        )

        # This is a boring example of the most basic example-based expectation renders.
        rendered_examples = []
        # TODO @Broken: remove naive many to many render of expectation x example
        for expectation in expectations:
            if expectation.type in (ExpectationType.INVARIANCE, ExpectationType.VARIANCE):
                for example in examples:
                    example_records = await _acollect(example.dataset)
                    for example_record in example_records:
                        rendered_example = await self.executor.run(
                            expectation.instruction, arguments=dict(example=example_record)
                        )
                        rendered_examples.append(rendered_example)
            elif expectation.type == ExpectationType.VERIFICATION:
                raise NotImplementedError(f"verification expectation {expectation} not supported")
        if len(rendered_examples) < 3:
            raise RuntimeError(f"not enough examples for {task}")
        await compiled_examples.aextend(rendered_examples)

        # render explanations and examples into prompt and apply (super basic)
        prompt_prefix: str = "\n".join(explanation_texts) + "\n"
        examples_keys = rendered_examples[0].keys()
        prompt_example = "\n".join(f"{key}: {{{key}}}" for key in examples_keys) + "\n"

        main_instruction = await Instruction.objects.acreate(
            name=task.name, task=task, scope=InstructionScope.PROGRAM, builtin_id="llm_fewshot"
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
            instruction=main_instruction, name="input", type=InstructionParameterType.JSON
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
                instruction=main_instruction,
                name=argument_name,
                type=argument_type,
            )
            await InstructionArgument.objects.acreate(
                instruction_bound=main_instruction,
                name=argument_name,
                type=argument_type,
                **argument_kwargs,
            )

    async def compile(self, compilation: Compilation) -> None:
        """
        Compile a task into an executable instruction.

        A task has explanations describing the task, expectations defining what should happen and examples showing that.
        A task may define a template instruction which is used to guide the compilation.
        A task may define subtasks, which are compiled recursively and may be folded into the main task.
        """

        logger.info("compile.start", compilation=compilation)
        # TODO @Feature: implement proper compile
        #  We assume a single task with basic explanations, expectations, basic examples and no source instruction.

        # just use first without any conversion for now (also super basic)
        backend_model = await compilation.backends.afirst()
        if backend_model is None:
            raise ValueError(f"no backends provided in compilation {compilation}")
        main_instruction = await self.compile_task(compilation.task, backend_model)
        compilation.target_instruction = main_instruction
        await sync_to_async(compilation.save)()
        logger.info("compile.done", compilation=compilation)
