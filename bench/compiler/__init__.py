from bench.executor import Executor
from bench.models import Dataset, Flow, Instruction, Task
from bench.models.compilation import Compilation


class Compiler:
    def __init__(self, executor: Executor):
        self.executor = executor

    async def compile(self, task: Task, source_flow: Flow) -> Compilation:
        if source_flow.root_instruction.task != task:
            raise ValueError("Source flow does not implement task")
        compilation = Compilation.objects.create(task=task, source_flow=source_flow)
        target_flow = Flow.objects.create(
            type=source_flow.type,
            name=f"{source_flow.name}_compiled.temp-{compilation.id.hex}",
            organization=source_flow.organization,
            project=source_flow.project,
        )

        # render expectations into examples
        expectations: list[Flow] = task.expectations.all()
        examples: list[Dataset] = task.examples.all()
        compiled_examples: Dataset = Dataset.objects.create(
            type="examples",
            name=f"{task.name}_compiled_examples.temp-{compilation.id.hex}",
            organization=source_flow.organization,
            project=source_flow.project,
        )

        rendered_examples = []
        for expectation in expectations:
            for example in examples:
                # This is a boring example and only works for the most basic example-based expectation.
                rendered_example = await self.executor.run_flow(
                    expectation, arguments=dict(example=example)
                )
                rendered_examples.append(rendered_example)
        compiled_examples.extend(rendered_examples)

        # render examples into prompt and apply
        prompt: Dataset = Dataset()
        model_instruction = Instruction.objects.create(
            flow=target_flow, name="llm", task=task, code_id="llm"
        )
        model_instruction.arguments.create(
            name="examples",
            dataset=compiled_examples,
        )

        compilation.target_flow = target_flow
        compilation.save()
        return compilation
