from bench.executor import Executor
from bench.models import Dataset, Instruction
from bench.models.compilation import Compilation


class Compiler:
    def __init__(self, executor: Executor):
        self.executor = executor

    async def compile(self, compilation: Compilation) -> None:
        # TODO @Feature: implement proper compile
        #  We assume single task with basic expectations, basic examples and no source instruction.

        task = compilation.task
        source_instruction = compilation.source_instruction
        if source_instruction.task is not None and source_instruction.task != task:
            raise ValueError("Source instruction implements another task")

        # render expectations into examples
        compiled_examples: Dataset = await Dataset.objects.acreate(
            type="examples",
            name=f"{task.name}_compiled_examples-{compilation.id.hex}",
        )

        # This is a boring example of the most basic example-based expectation renders.
        rendered_examples = []
        async for expectation in task.expectations.all():
            async for example in task.examples.all():
                rendered_example = await self.executor.run(
                    expectation, arguments=dict(example=example)
                )
                rendered_examples.append(rendered_example)
        if not rendered_examples:
            raise ValueError("No rendered examples available")
        await compiled_examples.aextend(rendered_examples)

        # render examples into prompt and apply (super basic)
        prompt_prefix: str = f"{task.name}\n{task.description}\n"
        examples_keys = rendered_examples[0].keys()
        prompt_example = "\n".join(f"{key}: {{{key}}}" for key in examples_keys)

        model_instruction = await Instruction.objects.acreate(
            name="llm_fewshot", task=task, code_id="llm_fewshot"
        )
        await model_instruction.arguments.acreate(name="prompt_prefix", value=prompt_prefix)
        await model_instruction.arguments.acreate(name="prompt_example", value=prompt_example)
        await model_instruction.arguments.acreate(
            name="examples",
            dataset=compiled_examples,
        )

        compilation.target_instruction = model_instruction
        compilation.save()
