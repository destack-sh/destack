from django.db import models

from bench.models.utils import UUIDModel


class Compilation(UUIDModel):
    """
    A compilation translates a task with a template instruction tree (instruction) into a runnable instruction.

    Depending on the compilation target and options, various optimizations may be applied.
    """

    task = models.ForeignKey("Symbol", on_delete=models.CASCADE, related_name="compilations+")
    backends = models.ManyToManyField("Symbol", related_name="compilations+")
    source = models.ForeignKey(
        "Symbol", on_delete=models.CASCADE, null=True, related_name="compilations+"
    )
    target = models.ForeignKey(
        "Symbol", on_delete=models.CASCADE, null=True, related_name="source_compilation"
    )

    def __str__(self):
        return f"{self.id.hex}.compilation"


class SourceMapping(UUIDModel):
    """
    A source mapping records how the source tree was compiled into the target tree.
    """

    source = models.ForeignKey("Symbol", on_delete=models.CASCADE, related_name="mappings+")
    source_path = models.JSONField(null=True)
    target = models.ForeignKey("Symbol", on_delete=models.CASCADE, related_name="source_mappings")
    target_path = models.JSONField(null=True)

    def __str__(self):
        source_str = f"{self.source}[{self.source_path}]" if self.source_path else str(self.source)
        target_str = f"{self.target}[{self.target_path}]" if self.target_path else str(self.target)
        return f"{self.id.hex}.map({source_str} -> {target_str})"
