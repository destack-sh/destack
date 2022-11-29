from django.db import models

from bench.models.symbol import SymbolContent
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel


class TaskManager(models.Manager["Task"]):
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

    objects: TaskManager = TaskManager()

    def __str__(self):
        return f"{self.id.hex}.task"

    class Meta:
        default_manager_name = "objects"


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


class Expectation(SymbolContent):
    """
    An expectation specifies a task's expected behavior.
    It can instruct or explain the context and relations of expected behavior.

    The main text of an expectation is its description and instructions & examples are statements.
    """

    description = models.TextField()
    statements = models.ManyToManyField(
        "SymbolDefinition",
        through="ExpectationStatement",
        related_name="references_in_expectations+",
    )

    def __str__(self):
        return f"{self.id.hex}.expect(description={self.description})"


class ExpectationStatement(UUIDModel):
    """
    The statement specifies how an expectation should be interpreted.
    Currently, statements can be examples or instructions to generate, transform or verify data.
    """

    expectation = models.ForeignKey(
        "Expectation", on_delete=models.CASCADE, related_name="statements+"
    )
    statement = models.ForeignKey(
        "SymbolDefinition", on_delete=models.CASCADE, related_name="expectations"
    )
