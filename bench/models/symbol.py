from __future__ import annotations

from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

import structlog
from django.db import models, transaction
from django.dispatch import receiver
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.models.schema import SchemaElementField
from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel, is_jsonable
from bench.utils.schema import SchemaElement

if TYPE_CHECKING:
    from bench.models import Code, Dataset, DatasetView, Expectation, Model, Task
    from bench.models.project import ProjectVersion

logger = structlog.get_logger(__name__)


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
        "SymbolDefinition", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    # children via SymbolDefinition
    index = models.IntegerField(null=True)  # index into file or parent if nested
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    generated = models.BooleanField(default=False)
    # source_mappings via SourceMapping

    task = models.OneToOneField(
        "Task", on_delete=models.RESTRICT, null=True, blank=True, related_name="definition"
    )
    expectation = models.OneToOneField(
        "Expectation", on_delete=models.RESTRICT, null=True, blank=True, related_name="definition"
    )
    code = models.OneToOneField(
        "Code", on_delete=models.RESTRICT, null=True, blank=True, related_name="definition"
    )
    model = models.OneToOneField(
        "Model", on_delete=models.RESTRICT, null=True, blank=True, related_name="definition"
    )
    dataset = models.OneToOneField(
        "Dataset", on_delete=models.RESTRICT, null=True, blank=True, related_name="definition"
    )
    dataset_view = models.OneToOneField(
        "DatasetView", on_delete=models.RESTRICT, null=True, blank=True, related_name="definition"
    )

    def deepcopy(self, to: SymbolDefinition, refs: dict[UUID, SymbolContent | SymbolDefinition]):
        """
        Deep copy this symbol definition to another symbol, replacing non-symbol references
         (incl. references to symbols within non-symbol relations like args/params)
        """
        # copy parameters
        for parameter in self.parameters.all():
            parameter.id = None
            parameter.symbol = to
            parameter.save()
        # copy arguments
        for argument in self.arguments.all():
            if argument.reference_id not in refs:
                continue
            argument.id = None
            argument.symbol = to
            argument.reference = cast(SymbolDefinition, refs[argument.reference_id])
            argument.save()

    # parameters to/from SymbolParameter
    @property
    def parameters(self) -> models.QuerySet["SymbolParameter"]:  # noqa
        pass

    # arguments to/from SymbolArgument
    @property
    def arguments(self) -> models.QuerySet["SymbolArgument"]:  # noqa
        pass

    def __str__(self):
        return f"{self.type_name_declaration}@{self.id.hex}"

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
    def type_name_declaration(self) -> str:
        return f"{self.type} {self.name}"

    @gql.model_property(only=["type"])
    def type_shortname(self) -> str:
        return self.type

    @gql.model_cached_property(
        only=["type", "task", "expectation", "code", "model", "dataset", "dataset_view"],
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

    def add_parameter(
        self,
        name: str,
        type: SymbolParameterType,
        exists_ok: bool = False,
        schema: Optional[SchemaElement] = None,
    ) -> SymbolParameter:
        if exists_ok:
            parameter, created = SymbolParameter.objects.get_or_create(
                symbol=self, name=name, defaults={"type": type, "schema": schema}
            )
            if not created:
                parameter.type = type
                parameter.schema = schema
                parameter.save()
            return parameter
        else:
            return SymbolParameter.objects.create(symbol=self, name=name, type=type, schema=schema)

    @transaction.atomic
    def bind_argument(
        self, name: str, value: Any | SymbolDefinition, exists_ok: bool = False
    ) -> tuple[SymbolParameter, SymbolArgument]:
        if value is None:
            raise ValueError(f"cannot bind {self} argument {name} to None")
        # get/create parameter and corresponding argument
        argument_type = SymbolParameterType.from_value(value)
        if argument_type == SymbolParameterType.VALUE:
            value, reference = value, None
        else:
            value, reference = None, value

        parameter = self.add_parameter(name, argument_type, exists_ok=True)
        argument, created = SymbolArgument.objects.get_or_create(
            symbol=self,
            name=name,
            defaults=dict(type=argument_type, value=value, reference=reference),
        )
        if not created and not exists_ok:
            raise RuntimeError(f"{self} argument {argument} already exists")
        elif not created:
            # update parameter type and kwargs
            argument.type = argument_type
            argument.value = value
            argument.reference = reference
            argument.save()

        return parameter, argument

    def bind_arguments(self, exists_ok: bool = False, **arguments: Any | SymbolDefinition):
        for name, value in arguments.items():
            self.bind_argument(name, value, exists_ok)

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
    if "content" in instance._state.fields_cache:
        instance.content.delete()
    else:
        # this shouldn't happen but just in case
        pass


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
    def definition(self) -> SymbolDefinition:  # noqa
        pass

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
    def type_name_declaration(self) -> str:
        return self.definition.type_name_declaration

    @gql.model_property(
        only=["definition"], select_related=["definition", "definition__parameters"]
    )
    def parameters(self) -> models.QuerySet[SymbolParameter]:
        return self.definition.parameters

    @gql.model_property(only=["definition"], select_related=["definition", "definition__arguments"])
    def arguments(self) -> models.QuerySet[SymbolArgument]:
        return self.definition.arguments

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


class SymbolParameterType(models.TextChoices):
    DATA = "dataset"
    MODEL = "model"
    CODE = "code"
    VALUE = "value"

    @staticmethod
    def from_value(obj: Any | SymbolDefinition) -> SymbolParameterType:
        if isinstance(obj, SymbolDefinition):
            if obj.type == SymbolType.DATASET or obj.type == SymbolType.DATASET_VIEW:
                return SymbolParameterType.DATA
            elif obj.type == SymbolType.MODEL:
                return SymbolParameterType.MODEL
            elif obj.type == SymbolType.CODE:
                return SymbolParameterType.CODE
            else:
                raise ValueError(f"unexpected symbol type for code parameter: {obj}")
        elif is_jsonable(obj):
            return SymbolParameterType.VALUE
        else:
            raise ValueError(f"unknown object {obj} to code parameter")


class SymbolParameter(UUIDModel):
    """
    A parameter is a named argument to a symbol definition.
    Parameters are typed using SchemaElements.

    All symbols can be parameterized, but not all symbols implement parameterization yet.
    """

    symbol = models.ForeignKey(
        SymbolDefinition, on_delete=models.CASCADE, related_name="parameters"
    )
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    type = TextChoicesField(choices_enum=SymbolParameterType)
    schema = SchemaElementField(null=True, blank=True)  # models need not have a schema

    def __str__(self):
        return f"{self.symbol}/parameters/{self.name}(type={self.type})"

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_symbol_parameter_name_ak",
                fields=["symbol", "name"],
            )
        ]


class SymbolArgument(UUIDModel):
    """
    An argument is a bound value to some parameter, either as symbol reference to a symbol or a JSON value.

    Note: if there is no corresponding parameter the argument is an anonymous import. This
     doesn't seem perfect, but works for now.
    """

    symbol = models.ForeignKey(SymbolDefinition, on_delete=models.CASCADE, related_name="arguments")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    type = models.CharField(max_length=64, choices=SymbolParameterType.choices)
    reference = models.ForeignKey(
        "SymbolDefinition", on_delete=models.CASCADE, null=True, blank=True
    )
    value = models.JSONField(null=True, blank=True)

    def __str__(self):
        return f"{self.symbol}/arguments/{self.name}(type={self.type})"

    class Meta:
        constraints = [
            # ensure that only one name per symbol is set
            models.UniqueConstraint(
                name="bench_symbol_argument_bound_name_ak",
                fields=["symbol", "name"],
            ),
        ]


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
