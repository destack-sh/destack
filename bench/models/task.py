from __future__ import annotations

from typing import TYPE_CHECKING
from uuid import UUID

from django.db import models, transaction

from bench.models.schema import SchemaElementField, SchemaField
from bench.models.symbol import Symbol, SymbolContent, SymbolContentManager, replace_refs
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel

if TYPE_CHECKING:
    from bench.models import Model


class TaskManager(SymbolContentManager, models.Manager["Task"]):
    pass


class Task(SymbolContent):
    """
    A task describes an interface and desired behaviour.

    A task may have sub-tasks, forming a task tree.
    Sub-tasks define smaller tasks which are composed or represented by the parent task.
    """

    input_schema = SchemaField("input")
    output_schema = SchemaElementField("output")
    expectations = models.ManyToManyField("Expectation", related_name="tasks")
    template_implementation = models.ForeignKey(
        "Code", on_delete=models.CASCADE, null=True, blank=True, related_name="templates"
    )

    compilations: models.QuerySet["Compilation"]  # noqa via Compilation.task

    def deepcopy(self, to: SymbolContent, refs: dict[UUID, Symbol | SymbolContent]):
        super().deepcopy(to, refs)
        # deep copy compilations
        for compilation in self.compilations.all():
            compilation.pk = None
            compilation.project_version = to.symbol.project_version
            compilation.deepcopy(to=compilation, refs=refs)
            compilation.save()

    @transaction.atomic
    def add_compilation(self, name: str, backends: list[Model]) -> Compilation:
        compilation = Compilation.objects.create(
            project_version=self.symbol.project_version,
            task=self,
            name=name,
        )
        compilation.backends.set(backends)
        return compilation

    def __str__(self):
        return f"{self.symbol_str}({self.input_schema}->{self.output_schema})"

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
        "Task", on_delete=models.SET_NULL, null=True, blank=True, related_name="source_compilation+"
    )
    target_code = models.ForeignKey(
        "Code", on_delete=models.SET_NULL, null=True, blank=True, related_name="source_compilation+"
    )

    mappings: models.QuerySet["SourceMapping"]  # noqa via SourceMapping.compilation

    def deepcopy(self, to: Compilation, refs: dict[UUID, Symbol | SymbolContent]):
        replace_refs(self, to, refs, include_many_to_many=False)
        to.save()
        replace_refs(self, to, refs, include_one_to_many=False)
        to.save()
        # deep copy mappings
        for mapping in self.mappings.all():
            mapping.pk = None
            mapping.compilation = to
            replace_refs(mapping, mapping, refs)
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
    source = models.ForeignKey("Symbol", on_delete=models.CASCADE, related_name="target_mappings")
    source_path = models.JSONField()
    target = models.ForeignKey("Symbol", on_delete=models.CASCADE, related_name="source_mappings")
    target_path = models.JSONField()


class ExpectationManager(SymbolContentManager, models.Manager["Expectation"]):
    pass


class Expectation(SymbolContent):
    """
    An expectation specifies expected behavior in the form of statements.
    """

    description = models.TextField()
    statements = models.ManyToManyField("Symbol", related_name="references_in_expectations+")

    def deepcopy(self, to: Expectation, refs: dict[UUID, Symbol | SymbolContent]):
        super().deepcopy(to, refs)
        # nothing custom to do yet (statements are just symbols for now)

    def __str__(self):
        return f"{self.symbol_str}(description={self.description})"

    objects = ExpectationManager()
