from django.db import models

from bench.models.utils import UUIDModel


class Compilation(UUIDModel):
    """
    A compilation translates a task with a template instruction tree (instruction) into a runnable instruction.

    Depending on the compilation target and options, various optimizations may be applied.
    """

    task = models.ForeignKey("Task", on_delete=models.CASCADE, related_name="compilations")
    backends = models.ManyToManyField("Model", related_name="compilations")
    source = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="compilations"
    )
    target = models.OneToOneField(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="source_compilation"
    )

    def __str__(self):
        return f"{self.task}.compilation@{self.id.hex}"
