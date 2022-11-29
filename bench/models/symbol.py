from __future__ import annotations

from typing import TYPE_CHECKING, Union
from uuid import UUID

from django.db import models, transaction
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel

if TYPE_CHECKING:
    from bench.models import Dataset, DatasetView, Expectation, Instruction, Model, Task
    from bench.models.project import ProjectVersion


class SymbolType(models.TextChoices):
    """
    The type of symbol to define in a project.
    """

    TASK = "task", "Task"
    EXPECTATION = "expect", "Expectation"
    INSTRUCTION = "instruct", "Instruction"
    MODEL = "model", "Model"
    DATASET = "data", "Dataset"
    DATASET_VIEW = "view", "DatasetView"

    @staticmethod
    def from_content(content: SymbolContent) -> SymbolType:
        # re-import for real to avoid circular import (above is only for type checking)
        from bench.models import Dataset, DatasetView, Expectation, Instruction, Model, Task  # noqa

        if isinstance(content, Task):
            return SymbolType.TASK
        elif isinstance(content, Expectation):
            return SymbolType.EXPECTATION
        elif isinstance(content, Instruction):
            return SymbolType.INSTRUCTION
        elif isinstance(content, Model):
            return SymbolType.MODEL
        elif isinstance(content, Dataset):
            return SymbolType.DATASET
        elif isinstance(content, DatasetView):
            return SymbolType.DATASET_VIEW
        else:
            raise ValueError(f"invalid symbol content type {type(content)}")


class SymbolDefinitionManager(models.Manager["SymbolDefinition"]):
    @transaction.atomic
    def create_definition(
        self,
        content: SymbolContent,
        project_version: ProjectVersion,
        **kwargs,
    ):
        # re-import for real (not just for type checking) to avoid circular import
        from bench.models import (  # noqa: F401
            Dataset,
            DatasetView,
            Expectation,
            Instruction,
            Model,
            Task,
        )

        # charade to avoid nullable definition in SymbolContent
        content_type = SymbolType.from_content(content)
        kwargs = {**kwargs, SymbolDefinition.type_to_field(content_type): content}
        definition = self.create(project_version=project_version, type=content_type, **kwargs)
        content.definition = definition
        content.save()
        definition.save()
        return definition


SYMBOL_TYPE_TO_FIELD = {
    SymbolType.TASK: "task",
    SymbolType.EXPECTATION: "expectation",
    SymbolType.INSTRUCTION: "instruction",
    SymbolType.MODEL: "model",
    SymbolType.DATASET: "dataset",
    SymbolType.DATASET_VIEW: "dataset_view",
}
SYMBOL_CONTENT_FIELDS = set(SYMBOL_TYPE_TO_FIELD.values())


class SymbolDefinition(TaggableMixin, UUIDModel):
    """
    A definition of a symbol for a specific project version, living in a specific file.

    In this implementation symbols can only be top-level definitions, not nested, which seems fine.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="definitions"
    )
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    type = TextChoicesField(choices_enum=SymbolType)
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="definitions")
    # TODO @Architecture: should symbol definition parent/children relation be on symbol definition or symbol?
    #  Currently, it's on symbol definition, which we copy for every project version.
    parent = models.ForeignKey(
        "SymbolDefinition", on_delete=models.CASCADE, null=True, related_name="children"
    )
    # children via SymbolDefinition
    index = models.IntegerField(null=True)  # index into file or parent if nested
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    committed_in = models.ForeignKey("ProjectVersion", on_delete=models.SET_NULL, null=True)

    task = models.ForeignKey(
        "Task", on_delete=models.RESTRICT, null=True, related_name="definition+"
    )
    expectation = models.ForeignKey(
        "Expectation", on_delete=models.RESTRICT, null=True, related_name="definition+"
    )
    instruction = models.ForeignKey(
        "Instruction", on_delete=models.RESTRICT, null=True, related_name="definition+"
    )
    model = models.ForeignKey(
        "Model", on_delete=models.RESTRICT, null=True, related_name="definition+"
    )
    dataset = models.ForeignKey(
        "Dataset", on_delete=models.RESTRICT, null=True, related_name="definition+"
    )
    dataset_view = models.ForeignKey(
        "DatasetView", on_delete=models.RESTRICT, null=True, related_name="definition+"
    )

    @gql.model_property(only=["committed_in"])
    def committed(self) -> bool:
        return self.committed_in is not None

    def __str__(self):
        return f"{self.name_dot_type}@{self.id.hex}"

    @staticmethod
    def type_to_field(type: SymbolType) -> str:
        return SYMBOL_TYPE_TO_FIELD[type]

    @gql.model_property(only=["name", "type"])
    def name_dot_type(self) -> str:
        return f"{self.name}.{self.type}"

    @gql.model_cached_property(
        only=["type"],
        select_related=["task", "expectation", "instruction", "model", "dataset", "dataset_view"],
    )
    def content(self) -> Union[Task, Expectation, Instruction, Model, Dataset, DatasetView]:
        content: Union[Task, Expectation, Instruction, Model, Dataset, DatasetView, None] = getattr(
            self, self.type_to_field(self.type)
        )
        if content is None:
            raise ValueError(f"{self} has no content for {self.type}")
        return content

    def set_content(self, content: SymbolContent | None):
        setattr(self, self.type_to_field(self.type), content)
        if content is not None:
            content.definition = self
            content.save()

    @property
    def content_id(self) -> UUID:
        return self.content.id

    objects: SymbolDefinitionManager = SymbolDefinitionManager()

    class Meta:
        default_manager_name = "objects"
        ordering = ["index", "created_at"]
        constraints = [
            # ensure unique index within file or parent
            models.UniqueConstraint(
                name="bench_symbol_definition_file_index_ak",
                fields=["file", "index"],
                condition=models.Q(parent__isnull=True),
            ),
            models.UniqueConstraint(
                name="bench_symbol_definition_parent_index_ak",
                fields=["parent", "index"],
                condition=models.Q(parent__isnull=False),
            ),
        ]


class SymbolContent(UUIDModel):
    """
    The content of a symbol definition. This is the interface that SymbolDefinition.content points to.
    Symbol definitions are mutable until committed.
    """

    definition = models.OneToOneField(
        "SymbolDefinition", on_delete=models.CASCADE, related_name="content+"
    )

    def deepcopy(self, to: SymbolContent, refs: dict[UUID, SymbolDefinition | SymbolContent]):
        """
        Deep copy this symbol to another symbol, replacing all references.
        The other symbol must be of the same type and is assumed to be created using:

        ```example
        to = from
        to.pk = None
        to.save()
        ```

        With the above, all value fields are automatically copied. Here we copy all relations and
        nested tables that are not symbols. Subclasses should override and extend this method.
        """
        if type(self) != type(to):
            raise ValueError(f"cannot copy {self} to {to}")

        # replace all relations referencing symbol definitions or contents with copies
        replace_refs(self, to, refs)

    class Meta:
        abstract = True


def replace_refs(
    obj: models.Model, to: models.Model, refs: dict[UUID, SymbolDefinition | SymbolContent]
):
    """Replaces all references to symbols with the given refs (refs need not be complete)."""
    for field in obj._meta.get_fields():
        if not field.is_relation:
            continue
        if not issubclass(field.related_model, (SymbolDefinition, SymbolContent)):
            continue
        # if many to one
        if field.many_to_one:
            value = getattr(obj, field.name)
            if value is not None and value.pk in refs:
                setattr(to, field.name, refs[value.id])
        # if many to many
        elif field.many_to_many:
            values = getattr(obj, field.name).all().values_list("id", flat=True)
            getattr(to, field.name).set(refs.get(id, id) for id in values)
