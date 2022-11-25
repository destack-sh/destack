from __future__ import annotations

import abc
from typing import TYPE_CHECKING, Optional, Union

from django.db import models, transaction
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel

if TYPE_CHECKING:
    from bench.models import Dataset, DatasetView, Instruction, Model, Task
    from bench.models.project import ProjectVersion


class SymbolType(models.TextChoices):
    """
    The type of symbol to define in a project.
    """

    TASK = "task", "Task"
    INSTRUCTION = "instruct", "Instruction"
    MODEL = "model", "Model"
    DATASET = "data", "Dataset"
    DATASET_VIEW = "view", "DatasetView"


class Symbol(TaggableMixin, UUIDModel):
    """
    A generic symbol of a specific immutable type.
    Symbols are used to reference tasks, instructions, models, and datasets.
    References are resolved to definitions within a project version.
    """

    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="symbols")
    type = TextChoicesField(choices_enum=SymbolType)
    # definitions via SymbolDefinition

    def __str__(self):
        return f"{self.project}/{self.id.hex}.{self.type}"

    def resolve(self, project_version: ProjectVersion) -> Optional[SymbolDefinition]:
        """
        Resolve the symbol to a definition in the given project version.
        """
        return self.definitions.filter(project_version=project_version).first()


class SymbolDefinitionManager(models.Manager["SymbolDefinition"]):
    @transaction.atomic
    def create_definition(
        self,
        content: SymbolContent,
        project_version: ProjectVersion,
        symbol: Optional[Symbol] = None,
        **kwargs,
    ):
        # re-import for real (not just for type checking) to avoid circular import
        from bench.models import Dataset, DatasetView, Instruction, Model, Task  # noqa: F401

        # map content to kwargs depending on type
        if isinstance(content, Task):
            kwargs = {**kwargs, "task": content, "type": SymbolType.TASK}
        elif isinstance(content, Instruction):
            kwargs = {**kwargs, "instruction": content, "type": SymbolType.INSTRUCTION}
        elif isinstance(content, Model):
            kwargs = {**kwargs, "model": content, "type": SymbolType.MODEL}
        elif isinstance(content, Dataset):
            kwargs = {**kwargs, "dataset": content, "type": SymbolType.DATASET}
        elif isinstance(content, DatasetView):
            kwargs = {**kwargs, "dataset_view": content, "type": SymbolType.DATASET_VIEW}
        else:
            raise ValueError(f"unexpected symbol content: {content}")
        if symbol is None:
            symbol = Symbol.objects.create(project=project_version.project, type=kwargs["type"])
        return self.create(
            name=content.name, symbol=symbol, project_version=project_version, **kwargs
        )

    def resolve(
        self, project_version: ProjectVersion, symbol: Symbol
    ) -> Optional[SymbolDefinition]:
        """
        Resolve the symbol to a definition in the given project version.
        """
        return self.filter(project_version=project_version, symbol=symbol).first()


class SymbolDefinition(TaggableMixin, UUIDModel):
    """
    A definition of a symbol for a specific project version, living in a specific file.

    In this implementation symbols can only be top-level definitions, not nested, which seems fine.
    """

    symbol = models.ForeignKey("Symbol", on_delete=models.CASCADE, related_name="definitions")
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="definitions"
    )
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    type = TextChoicesField(choices_enum=SymbolType)
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="definitions")
    index = models.IntegerField(null=True)  # index into file
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    task = models.ForeignKey(
        "Task", on_delete=models.RESTRICT, null=True, related_name="definitions"
    )
    instruction = models.ForeignKey(
        "Instruction", on_delete=models.RESTRICT, null=True, related_name="definitions"
    )
    model = models.ForeignKey(
        "Model", on_delete=models.RESTRICT, null=True, related_name="definitions"
    )
    dataset = models.ForeignKey(
        "Dataset", on_delete=models.RESTRICT, null=True, related_name="definitions"
    )
    dataset_view = models.ForeignKey(
        "DatasetView", on_delete=models.RESTRICT, null=True, related_name="definitions"
    )

    def __str__(self):
        return f"{self.symbol}={self.name_dot_type}@{self.id.hex}"

    @gql.model_property(only=["name", "type"])
    def name_dot_type(self) -> str:
        return f"{self.name}.{self.type}"

    @gql.model_cached_property(
        only=["type", "task", "instruction", "model", "dataset", "dataset_view"],
        select_related=["task", "instruction", "model", "dataset", "dataset_view"],
    )
    def content(self) -> Union[Task, Instruction, Model, Dataset, DatasetView]:
        if self.type == SymbolType.TASK:
            content = self.task
        elif self.type == SymbolType.INSTRUCTION:
            content = self.instruction
        elif self.type == SymbolType.MODEL:
            content = self.model
        elif self.type == SymbolType.DATASET:
            content = self.dataset
        elif self.type == SymbolType.DATASET_VIEW:
            content = self.dataset_view
        else:
            raise ValueError(f"{self} has unknown type: {self.type}")
        if content is None:
            raise ValueError(f"{self} has no content for {self.type}")
        else:
            return content

    objects: SymbolDefinitionManager = SymbolDefinitionManager()

    class Meta:
        default_manager_name = "objects"
        constraints = [
            # symbol can only be defined once per project version
            models.UniqueConstraint(
                name="bench_symbol_definition_symbol_project_version_ak",
                fields=["symbol", "project_version"],
            ),
            # each content object can only be defined once per project version
            models.UniqueConstraint(
                name="bench_symbol_definition_task_project_version_ak",
                fields=["task", "project_version"],
            ),
            models.UniqueConstraint(
                name="bench_symbol_definition_instruction_project_version_ak",
                fields=["instruction", "project_version"],
            ),
            models.UniqueConstraint(
                name="bench_symbol_definition_model_project_version_ak",
                fields=["model", "project_version"],
            ),
            models.UniqueConstraint(
                name="bench_symbol_definition_dataset_project_version_ak",
                fields=["dataset", "project_version"],
            ),
            models.UniqueConstraint(
                name="bench_symbol_definition_dataset_view_project_version_ak",
                fields=["dataset_view", "project_version"],
            ),
        ]


class SymbolContent(UUIDModel):
    """
    The content of a symbol definition. This is what SymbolDefinition.content points to.
    Symbol definitions are mutable until committed.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    committed_in = models.ForeignKey("ProjectVersion", on_delete=models.SET_NULL, null=True)

    @abc.abstractmethod
    def copy(self) -> SymbolContent:
        raise NotImplementedError

    @gql.model_property(only=["committed_in"])
    def committed(self) -> bool:
        return self.committed_in is not None

    class Meta:
        abstract = True
