from django.db.models import QuerySet

from bench.executor import Executor
from bench.models import Dataset, Instruction, Task
from bench.models.compilation import Compilation


class Compiler:
    def __init__(self, executor: Executor):
        self.executor = executor

    async def compile(self, task: Task, source_instruction: Instruction) -> Compilation:
        # TODO @Feature: implement proper compile
        #  We assume single task with basic expectations, basic examples and no source instruction.

        if source_instruction.task is not None and source_instruction.task != task:
            raise ValueError("Source instruction implements another task")

        compilation = Compilation.objects.create(task=task, source_instruction=source_instruction)

        # render expectations into examples
        expectations: QuerySet[Instruction] = task.expectations.all()
        examples: QuerySet[Dataset] = task.examples.all()
        compiled_examples: Dataset = Dataset.objects.create(
            type="examples",
            name=f"{task.name}_compiled_examples-{compilation.id.hex}",
        )

        rendered_examples = []
        for expectation in expectations:
            for example in examples:
                # This is a boring example and only works for the most basic example-based expectation.
                rendered_example = await self.executor.run(
                    expectation, arguments=dict(example=example)
                )
                rendered_examples.append(rendered_example)
        compiled_examples.extend(rendered_examples)

        # render examples into prompt and apply (super basic)
        prompt_prefix: str = f"{task.name}\n{task.description}\n"
        examples_keys = rendered_examples[0].keys()
        prompt_example = "\n".join(f"{key}: {{{key}}}" for key in examples_keys)

        model_instruction = Instruction.objects.create(
            name="llm_fewshot", task=task, code_id="llm_fewshot"
        )
        model_instruction.arguments.create(name="prompt_prefix", value=prompt_prefix)
        model_instruction.arguments.create(name="prompt_example", value=prompt_example)
        model_instruction.arguments.create(
            name="examples",
            dataset=compiled_examples,
        )

        compilation.target_instruction = model_instruction
        compilation.save()
        return compilation
