from __future__ import annotations

from typing import TYPE_CHECKING, Any, Union, cast
from uuid import UUID

from django.db import models
from django.dispatch import receiver
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel

if TYPE_CHECKING:
    from bench.models import Code, Dataset, DatasetView, Expectation, Model, Task
    from bench.models.project import ProjectVersion


class SymbolType(models.TextChoices):
    """
    The type of symbol to define in a project.
    """

    TASK = "task", "Task"
    EXPECTATION = "expect", "Expectation"
    CODE = "code", "Code"
    MODEL = "model", "Model"
    DATASET = "data", "Dataset"
    DATASET_VIEW = "view", "DatasetView"

    @staticmethod
    def from_content(content: SymbolContent) -> SymbolType:
        # re-import for real to avoid circular import (above is only for type checking)
        from bench.models import Code, Dataset, DatasetView, Expectation, Model, Task  # noqa

        if isinstance(content, Task):
            return SymbolType.TASK
        elif isinstance(content, Expectation):
            return SymbolType.EXPECTATION
        elif isinstance(content, Code):
            return SymbolType.CODE
        elif isinstance(content, Model):
            return SymbolType.MODEL
        elif isinstance(content, Dataset):
            return SymbolType.DATASET
        elif isinstance(content, DatasetView):
            return SymbolType.DATASET_VIEW
        else:
            raise ValueError(f"invalid symbol content type {type(content)}")


class SymbolDefinitionManager(models.Manager["SymbolDefinition"]):
    def create_definition(
        self,
        content: SymbolContent,
        project_version: ProjectVersion,
        **kwargs,
    ):
        # re-import for real (not just for type checking) to avoid circular import
        from bench.models import Code, Dataset, DatasetView, Expectation, Model, Task  # noqa: F401

        # save content if it's not loaded from the db
        # (don't test via pk since we set that automatically)
        if content._state.adding:
            content.save()

        content_type = SymbolType.from_content(content)
        kwargs = {**kwargs, SymbolDefinition.type_to_field(content_type): content}
        return self.create(project_version=project_version, type=content_type, **kwargs)


SYMBOL_TYPE_TO_FIELD = {
    SymbolType.TASK: "task",
    SymbolType.EXPECTATION: "expectation",
    SymbolType.CODE: "code",
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
    parent = models.ForeignKey(
        "SymbolDefinition", on_delete=models.CASCADE, null=True, related_name="children"
    )
    # children via SymbolDefinition
    index = models.IntegerField(null=True)  # index into file or parent if nested
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    committed_in = models.ForeignKey("ProjectVersion", on_delete=models.SET_NULL, null=True)
    generated = models.BooleanField(default=False)
    # source_mappings via SourceMapping

    task = models.OneToOneField(
        "Task", on_delete=models.RESTRICT, null=True, related_name="definition"
    )
    expectation = models.OneToOneField(
        "Expectation", on_delete=models.RESTRICT, null=True, related_name="definition"
    )
    code = models.OneToOneField(
        "Code", on_delete=models.RESTRICT, null=True, related_name="definition"
    )
    model = models.OneToOneField(
        "Model", on_delete=models.RESTRICT, null=True, related_name="definition"
    )
    dataset = models.OneToOneField(
        "Dataset", on_delete=models.RESTRICT, null=True, related_name="definition"
    )
    dataset_view = models.OneToOneField(
        "DatasetView", on_delete=models.RESTRICT, null=True, related_name="definition"
    )

    @gql.model_property(only=["committed_in"])
    def committed(self) -> bool:
        return self.committed_in is not None

    def __str__(self):
        return f"{self.name_dot_type}@{self.id.hex}"

    @property
    def path(self) -> str:
        return f"{self.file.path}/{self.local_path}"

    @property
    def local_path(self):
        """
        Path relative to the current file
        """
        if self.parent is not None:
            return f"{self.parent.local_path}.{self.name}"
        else:
            return self.name

    @staticmethod
    def type_to_field(type: SymbolType) -> str:
        return SYMBOL_TYPE_TO_FIELD[type]

    @gql.model_property(only=["name", "type"])
    def name_dot_type(self) -> str:
        return f"{self.name}.{self.type}"

    @gql.model_property(only=["name", "type"])
    def type_name_declaration(self) -> str:
        return f"{self.type} {self.name}"

    @gql.model_property(only=["type"])
    def type_shortname(self) -> str:
        return self.type

    @gql.model_cached_property(
        only=["type"],
        select_related=["task", "expectation", "code", "model", "dataset", "dataset_view"],
    )
    def content(self) -> Union[Task, Expectation, Code, Model, Dataset, DatasetView]:
        content: Union[Task, Expectation, Code, Model, Dataset, DatasetView, None] = getattr(
            self, self.type_to_field(self.type)
        )
        if content is None:
            raise ValueError(f"{self} has no content for {self.type}")
        return content

    def set_content(self, content: SymbolContent | None):
        setattr(self, self.type_to_field(self.type), content)

    @property
    def task_(self) -> Task:
        if TYPE_CHECKING:
            return cast(Task, self.content)
        else:
            return self.content  # noqa

    @property
    def expectation_(self) -> Expectation:
        if TYPE_CHECKING:
            return cast(Expectation, self.content)
        else:
            return self.content  # noqa

    @property
    def code_(self) -> Code:
        if TYPE_CHECKING:
            return cast(Code, self.content)
        else:
            return self.content  # noqa

    @property
    def model_(self) -> Model:
        if TYPE_CHECKING:
            return cast(Model, self.content)
        else:
            return self.content  # noqa

    @property
    def dataset_(self) -> Dataset:
        if TYPE_CHECKING:
            return cast(Dataset, self.content)
        else:
            return self.content  # noqa

    @property
    def dataset_view_(self) -> DatasetView:
        if TYPE_CHECKING:
            return cast(DatasetView, self.content)
        else:
            return self.content  # noqa

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


# auto delete symbol content if symbol definition is deleted
@receiver(models.signals.post_delete, sender=SymbolDefinition)
def auto_delete_symbol_content(sender, instance: SymbolDefinition, **kwargs):
    instance.content.delete()


class SymbolContentManager(models.Manager):
    # Note that this manager is applied to _every_ SymbolContent query (related or not)!

    def get_queryset(self):
        # always select related definition
        return super().get_queryset().select_related("definition")


class SymbolContent(UUIDModel):
    """
    The content of a symbol definition. This is the interface that SymbolDefinition.content points to.
    Symbol definitions are mutable until committed.
    """

    # definition is a one to one field via SymbolDefinition
    # (and set automatically when creating a SymbolDefinition)
    @property
    def definition(self) -> SymbolDefinition:
        raise NotImplementedError

    @property
    def definition_str(self) -> str:
        """Gets a definition str for logging that handles not yet defined symbol contents"""
        # check if definition is in model cache
        if "definition" in self._state.fields_cache:  # type: ignore
            return str(self.definition)
        else:
            return "<undefined>"

    @gql.model_property(only=["definition"], select_related=["definition"])
    def type(self) -> SymbolType:
        return self.definition.type

    @gql.model_property(only=["definition"], select_related=["definition"])
    def name_dot_type(self) -> str:
        return self.definition.name_dot_type

    @gql.model_property(only=["definition"], select_related=["definition"])
    def type_name_declaration(self) -> str:
        return self.definition.type_name_declaration

    def deepcopy(self, to: Any, refs: dict[UUID, SymbolDefinition | SymbolContent]):
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

    objects: SymbolContentManager = SymbolContentManager()

    class Meta:
        default_manager_name = "objects"
        # set base manager so that _every_ query to SymbolContent goes through SymbolContentManager
        # which ensures that we always select related definition
        base_manager_name = "objects"
        abstract = True


def replace_refs(
    obj: models.Model,
    to: models.Model,
    refs: dict[UUID, SymbolDefinition | SymbolContent],
    include_one_to_many: bool = True,
    include_many_to_many: bool = True,
):
    """Replaces all references to symbols with the given refs (refs need not be complete)."""
    for field in obj._meta.get_fields():
        if field.related_model is None:
            continue
        if not issubclass(field.related_model, (SymbolDefinition, SymbolContent)):
            continue
        # if many to one
        if include_one_to_many and field.many_to_one:
            value = getattr(obj, field.name)
            if value is not None and value.pk in refs:
                setattr(to, field.name, refs[value.id])
        # if many to many
        if include_many_to_many and field.many_to_many:
            values = getattr(obj, field.name).all().values_list("id", flat=True)
            getattr(to, field.name).set(refs.get(id, id) for id in values)
