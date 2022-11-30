from __future__ import annotations

from uuid import UUID

from django.db import models

from bench.models.symbol import SymbolContent, SymbolContentManager, SymbolDefinition, replace_refs
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel


class TaskManager(SymbolContentManager, models.Manager["Task"]):
    pass


class Task(SymbolContent):
    """
    A task describes the interface and desired behaviour of an instruction.

    A task may have sub-tasks, forming a task tree.
    Sub-tasks define smaller tasks which are composed or represented by the parent task.
    """

    schema = models.JSONField()
    expectations = models.ManyToManyField("Expectation", related_name="tasks")
    template_implementation = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="templates"
    )
    # compilations via Compilation

    def deepcopy(self, to: SymbolContent, refs: dict[UUID, SymbolDefinition | SymbolContent]):
        super().deepcopy(to, refs)
        # deep copy compilations
        for compilation in self.compilations.all():
            compilation.pk = None
            replace_refs(compilation, compilation, refs, include_many_to_many=False)
            compilation.save()
            replace_refs(compilation, compilation, refs, include_one_to_many=False)
            compilation.save()

    def __str__(self):
        return f"{self.definition}(schema={self.schema})"

    objects = TaskManager()


class Compilation(UUIDModel):
    """
    A compilation translates a task with a template instruction tree (instruction) into a runnable instruction.

    Depending on the compilation target and options, various optimizations may be applied.
    """

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    task = models.ForeignKey("Task", on_delete=models.CASCADE, related_name="compilations")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    backends = models.ManyToManyField("Model", related_name="compilations+")
    output_task = models.ForeignKey(
        "Task", on_delete=models.CASCADE, null=True, related_name="compilations+"
    )
    output_instruction = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="source_compilation"
    )

    def __str__(self):
        return f"{self.id.hex}.compilation"

    class Meta:
        constraints = [
            # ensure only one compilation per task+name
            models.UniqueConstraint(fields=["task", "name"], name="bench_compilation_task_name_ak"),
        ]


class ExpectationManager(SymbolContentManager, models.Manager["Expectation"]):
    pass


class Expectation(SymbolContent):
    """
    An expectation specifies a task's expected behavior.
    It can instruct or explain the context and relations of expected behavior.

    The main text of an expectation is its description and instructions & examples are statements.
    """

    description = models.TextField()
    statements = models.ManyToManyField(
        "SymbolDefinition", related_name="references_in_expectations+"
    )

    def __str__(self):
        return f"{self.definition}(description={self.description})"

    objects = ExpectationManager()
