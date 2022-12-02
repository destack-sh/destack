from __future__ import annotations

from uuid import UUID

from django.db import models

from bench.models.symbol import SymbolContent, SymbolContentManager, SymbolDefinition, replace_refs
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel


class TaskManager(SymbolContentManager, models.Manager["Task"]):
    pass


class Task(SymbolContent):
    """
    A task describes the interface and desired behaviour of an code.

    A task may have sub-tasks, forming a task tree.
    Sub-tasks define smaller tasks which are composed or represented by the parent task.
    """

    schema = models.JSONField()
    expectations = models.ManyToManyField("Expectation", related_name="tasks")
    template_implementation = models.ForeignKey(
        "Code", on_delete=models.CASCADE, null=True, related_name="templates"
    )
    # compilations via Compilation

    def deepcopy(self, to: SymbolContent, refs: dict[UUID, SymbolDefinition | SymbolContent]):
        super().deepcopy(to, refs)
        # deep copy compilations
        for compilation in self.compilations.all():
            compilation.pk = None
            compilation.project_version = to.definition.project_version
            compilation.deepcopy(to=compilation, refs=refs)

    def __str__(self):
        return f"{self.definition_str}(schema={self.schema})"

    objects = TaskManager()


class Compilation(UUIDModel):
    """
    A compilation translates a task with a template code tree (code) into a runnable code.

    Depending on the compilation target and options, various optimizations may be applied.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="compilations"
    )
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    task = models.ForeignKey("Task", on_delete=models.CASCADE, related_name="compilations")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    backends = models.ManyToManyField("Model", related_name="compilations+")
    target_task = models.ForeignKey(
        "Task", on_delete=models.CASCADE, null=True, related_name="compilations+"
    )
    target_code = models.ForeignKey(
        "Code", on_delete=models.CASCADE, null=True, related_name="source_compilation+"
    )
    # mappings via SourceMapping

    def deepcopy(self, to: Compilation, refs: dict[UUID, SymbolDefinition | SymbolContent]):
        replace_refs(self, to, refs, include_many_to_many=False)
        to.save()
        replace_refs(self, to, refs, include_one_to_many=False)
        to.save()
        # deep copy mappings
        for mapping in self.mappings.all():
            mapping.pk = None
            mapping.compilation = to
            mapping.save()

    def __str__(self):
        return f"{self.id.hex}.compilation"

    class Meta:
        constraints = [
            # ensure only one compilation per task+name
            models.UniqueConstraint(fields=["task", "name"], name="bench_compilation_task_name_ak"),
        ]


class SourceMapping(UUIDModel):
    """
    A source map for compilations to track the mapping between source and target instructions.
    """

    compilation = models.ForeignKey(
        "Compilation", on_delete=models.CASCADE, related_name="mappings"
    )
    source = models.ForeignKey(
        "SymbolDefinition", on_delete=models.CASCADE, related_name="target_mappings"
    )
    source_path = models.JSONField()
    target = models.ForeignKey(
        "SymbolDefinition", on_delete=models.CASCADE, related_name="source_mappings"
    )
    target_path = models.JSONField()


class Expectation(SymbolContent):
    """
    An expectation specifies a task's expected behavior.
    It can instruct or explain the context and relations of expected behavior.

    The main text of an expectation is its description and code & examples are statements.
    """

    description = models.TextField()
    statements = models.ManyToManyField(
        "SymbolDefinition", related_name="references_in_expectations+"
    )

    def __str__(self):
        return f"{self.definition_str}(description={self.description})"
