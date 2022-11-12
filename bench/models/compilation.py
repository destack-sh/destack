from django.db import models

from bench.models.utils import UUIDModel


class Compilation(UUIDModel):
    """
    A compilation translates a task with a template instruction tree (instruction) into a runnable instruction.

    Depending on the compilation target and options, various optimizations may be applied.
    """

    task = models.ForeignKey("Task", on_delete=models.CASCADE, related_name="compilations")
    source_instruction = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, related_name="compilations"
    )
    target_instruction = models.OneToOneField(
        "Instruction", on_delete=models.CASCADE, related_name="source_compilation"
    )
