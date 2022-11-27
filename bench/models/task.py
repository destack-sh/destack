from django.db import models

from bench.models.symbol import SymbolContent
from bench.models.utils import UUIDModel


class TaskManager(models.Manager["Task"]):
    pass


class Task(SymbolContent):
    """
    A task describes the interface and desired behaviour of an instruction.

    A task may have sub-tasks, forming a task tree.
    Sub-tasks define smaller tasks which are composed or represented by the parent task.
    """

    schema = models.JSONField()
    expectations = models.ManyToManyField("Symbol", related_name="tasks")
    template_implementation = models.ForeignKey(
        "Symbol", on_delete=models.CASCADE, null=True, related_name="templates"
    )
    # implementations from/to Instruction (via Symbol)

    objects: TaskManager = TaskManager()

    def __str__(self):
        return f"task@{self.id.hex}"

    class Meta:
        default_manager_name = "objects"


class Expectation(SymbolContent):
    """
    An expectation specifies a task's expected behavior.
    It can instruct or explain the context and relations of expected behavior.

    The main text of an expectation is its description and instructions & examples are statements.
    """

    description = models.TextField()
    statements = models.ManyToManyField(
        "Symbol", through="ExpectationStatement", related_name="references_in_expectations+"
    )

    def __str__(self):
        return f"expect(description={self.description})@{self.id.hex}"


class ExpectationStatement(UUIDModel):
    """
    The statement specifies how an expectation should be interpreted.
    Currently, statements can be examples or instructions to generate, transform or verify data.
    """

    expectation = models.ForeignKey(
        "Expectation", on_delete=models.CASCADE, related_name="statements+"
    )
    statement = models.ForeignKey("Symbol", on_delete=models.CASCADE, related_name="expectations")
