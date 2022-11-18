import structlog
from asgiref.sync import sync_to_async
from django.db.models import QuerySet

from bench.executor import Executor
from bench.models import (
    Dataset,
    Instruction,
    InstructionArgument,
    Model,
    Organization,
    ProjectFileType,
)
from bench.models.compilation import Compilation
from bench.models.task import Example, Expectation, Explanation

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

    async def compile(self, compilation: Compilation) -> None:
        logger.info("compile.start", compilation=compilation)
        # TODO @Feature: implement proper compile
        #  We assume a single task with basic explanations, expectations, basic examples and no source instruction.
        task = compilation.task
        explanations: list[Explanation] = await _acollect(
            task.explanations.select_related("dataset")
        )
        expectations: list[Expectation] = await _acollect(
            task.expectations.select_related("instruction")
        )
        examples: list[Example] = await _acollect(
            task.examples.select_related("dataset").prefetch_related("dataset__records")
        )

        # render explanations into prompts
        rendered_explanations = []
        for explanation in explanations:
            statements: list[dict] = await _acollect(explanation.dataset)
            for statement in statements:
                # TODO @Feature: render explanations, use dataset view to get fields
                rendered_explanation = statement["text"]
                rendered_explanations.append(rendered_explanation)

        # render expectations into examples
        compiled_examples: Dataset = await Dataset.objects.acreate(
            type="examples",
            name=f"{task.name}_compiled_examples-{compilation.id.hex}",
        )

        # This is a boring example of the most basic example-based expectation renders.
        rendered_examples = []
        # naive many to many render of expectation x example
        for expectation in expectations:
            for example in examples:
                example_records = await _acollect(example.dataset)
                for example_record in example_records:
                    rendered_example = await self.executor.run(
                        expectation.instruction, arguments=dict(example=example_record)
                    )
                    rendered_examples.append(rendered_example)
        if not rendered_examples:
            raise RuntimeError(f"no output generated for {compilation}")
        await compiled_examples.aextend(rendered_examples)

        # render explanations and examples into prompt and apply (super basic)
        prompt_prefix: str = "\n".join(rendered_explanations)
        examples_keys = rendered_examples[0].keys()
        prompt_example = "\n".join(f"{key}: {{{key}}}" for key in examples_keys)

        model_instruction = await Instruction.objects.acreate(
            name="llm_fewshot", task=task, code_id="llm_fewshot"
        )
        arguments = {
            "prompt_prefix": {"value": prompt_prefix},
            "prompt_example": {"value": prompt_example},
            "examples": {"dataset": compiled_examples},
        }
        for argument_name, argument_value in arguments.items():
            await InstructionArgument.objects.acreate(
                instruction_bound=model_instruction, name=argument_name, **argument_value
            )

        compilation.target_instruction = model_instruction
        await sync_to_async(compilation.save)()
        logger.info("compile.done", compilation=compilation)
