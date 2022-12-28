from __future__ import annotations

from typing import TYPE_CHECKING

from django.db import models, transaction

from bench.models.schema_field import Schemad
from bench.models.symbol import Statement, SymbolContent, SymbolContentManager, SymbolType

if TYPE_CHECKING:
    from bench.models import Compilation, Model


class TaskManager(SymbolContentManager, models.Manager["Task"]):
    pass


class Task(Schemad, SymbolContent):
    """
    A task describes an interface and its desired behaviour.

    A task may have sub-tasks, forming a task tree.
    Sub-tasks define smaller tasks which are composed or represented by the parent task.
    """

    description = models.TextField()

    @property
    def expectations(self) -> models.QuerySet["Statement"]:
        return self.definition.active_children_like(SymbolType.EXPECTATION)

    @property
    def subtasks(self) -> models.QuerySet["Task"]:
        return self.definition.active_children_like(SymbolType.TASK)

    @transaction.atomic
    def add_compilation(self, name: str, backends: list[Model]) -> Compilation:
        compilation = Compilation.objects.create(
            project_version=self.definition.project_version,
            task=self,
            name=name,
        )
        compilation.backends.set(backends)
        return compilation

    def __str__(self):
        return f"({self.description})"

    objects = TaskManager()


class ExpectationManager(SymbolContentManager, models.Manager["Expectation"]):
    pass


class Expectation(SymbolContent):
    """
    An expectation specifies expected behavior of a task (and other symbols?).
    """

    description = models.TextField()

    def __str__(self):
        return f"(description={self.description})"

    objects = ExpectationManager()
