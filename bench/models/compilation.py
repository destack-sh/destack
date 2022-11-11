from django.db import models

from bench.models.utils import UUIDModel


class Compilation(UUIDModel):
    """
    A compilation translates a task with a template instruction tree (flow) into a runnable flow.

    Depending on the compilation target and options, various optimizations may be applied.
    """

    task = models.ForeignKey("Task", on_delete=models.CASCADE, related_name="compilations")
    source_flow = models.ForeignKey("Flow", on_delete=models.CASCADE, related_name="compilations")
    target_flow = models.OneToOneField(
        "Flow", on_delete=models.CASCADE, related_name="source_compilation"
    )
