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
    from bench.models import Code, Dataset, Expectation, File, Model, ProjectVersion, Task

logger = structlog.get_logger(__name__)


class StatementType(models.TextChoices):
    """
    The type of Bench statement.
    """

    IMPORT = "import"
    DEFINITION = "define"
    REFERENCE = "ref"
    COMMENT = "comment"


class StatementManager(models.Manager["Statement"]):
    @transaction.atomic
    def create_statement(
        self,
        project_version: ProjectVersion,
        file: File,
        type: StatementType,
        parent: Optional[Statement],
        index: Optional[int],
        content: SymbolContent,
        name: str,
    ) -> Statement:
        # auto set index if not passed
        if index is None:
            if parent:
                index = parent.children.count()
            else:
                index = file.symbols.count()

        statement = self.create(
            project_version=project_version, file=file, type=type, parent=parent, index=index
        )
        statement.symbol = Symbol.objects.create_symbol(
            project_version=project_version,
            file=file,
            statement=statement,
            content=content,
            name=name,
        )
        return statement


class Statement(UUIDModel):
    """
    A statement in a file. Can import, define or reference a symbol.
    Statements may be nested and have semantic meaning (parent-child relationships, comments, etc.).
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="statements"
    )
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="statements")
    type = TextChoicesField(choices_enum=StatementType)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    commented = models.BooleanField(default=False)
    generated = models.BooleanField(default=False)
    parent = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    children: models.QuerySet[Statement]  # noqa via Statement.parent
    index = models.IntegerField(null=True)  # index into file or parent statement

    symbol: Optional[Symbol]  # noqa via Symbol.statement
    reference = models.ForeignKey(
        "Symbol",
        on_delete=models.CASCADE,
        null=True,
        blank=True,
        related_name="referencing_statements",
    )
    arguments: models.QuerySet[SymbolArgument]  # noqa via SymbolArgument.statement
    text = models.TextField(null=True, blank=True)  # as markdown

    def deepcopy(self, to: Statement, refs: dict[UUID, SymbolContent | Symbol | Statement]):
        raise NotImplementedError

    @property
    def symbol_(self) -> Symbol:
        if self.symbol is None:
            raise ValueError(f"{self} has no symbol")
        return self.symbol

    @property
    def reference_(self) -> Symbol:
        if self.reference is None:
            raise ValueError(f"{self} has no reference")
        return self.reference

    @property
    def absolute_index(self):
        return self.index

    def __str__(self):
        path = self.file.path + ":" + str(self.absolute_index)
        if self.type == StatementType.DEFINITION:
            content_str = f"{self.type} {self.symbol}"
        elif self.type in (StatementType.IMPORT, StatementType.REFERENCE):
            content_str = f"{self.type} {self.reference}"
        elif self.type == StatementType.COMMENT:
            content_str = f"{self.type} {len(self.text)}"
        else:
            raise ValueError(f"unknown statement type {self.type}")
        return f"{path} {content_str}"

    objects: StatementManager = StatementManager()

    class Meta:
        ordering = ["index"]
        default_manager_name = "objects"
        constraints = [
            # ensure unique index within file or parent
            models.UniqueConstraint(
                name="bench_statement_file_index_ak",
                fields=["file", "index"],
                condition=models.Q(parent__isnull=True),
            ),
            models.UniqueConstraint(
                name="bench_statement_parent_index_ak",
                fields=["parent", "index"],
                condition=models.Q(parent__isnull=False),
            ),
        ]


class SymbolType(models.TextChoices):
    """
    The type of symbol to define in a project.
    """

    TASK = "task", "Task"
    EXPECTATION = "expect", "Expectation"
    CODE = "code", "Code"
    MODEL = "model", "Model"
    DATASET = "data", "Dataset"

    @staticmethod
    def from_content(content: SymbolContent) -> SymbolType:
        # re-import for real to avoid circular import (above is only for type checking)
        from bench.models import Code, Dataset, Expectation, Model, Task  # noqa

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
        else:
            raise ValueError(f"invalid symbol content type {type(content)}")


class SymbolManager(models.Manager["Symbol"]):
    # Note that this manager is applied to _every_ SymbolContent query (related or not)!

    def get_queryset(self):
        # always select related symbol
        return super().get_queryset().select_related("statement")

    def create_symbol(
        self,
        content: SymbolContent,
        project_version: ProjectVersion,
        file: File,
        statement: Statement,
        name: str,
    ):
        # re-import for real (not just for type checking) to avoid circular import
        from bench.models import Code, Dataset, Expectation, Model, Task  # noqa: F401

        # save content if it's not loaded from the db
        # (don't test via pk since we set that automatically)
        if content._state.adding:
            content.save()

        content_type = SymbolType.from_content(content)
        kwargs = {Symbol.type_to_field(content_type): content}
        return self.create(
            project_version=project_version,
            file=file,
            statement=statement,
            type=content_type,
            name=name,
            **kwargs,
        )


SYMBOL_TYPE_TO_FIELD = {
    SymbolType.TASK: "task",
    SymbolType.EXPECTATION: "expectation",
    SymbolType.CODE: "code",
    SymbolType.MODEL: "model",
    SymbolType.DATASET: "dataset",
}
SYMBOL_CONTENT_FIELDS = set(SYMBOL_TYPE_TO_FIELD.values())


class Symbol(TaggableMixin, UUIDModel):
    """
    A symbol defining a primitive element in a specific project version and file.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="symbols"
    )
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="symbols")
    statement = models.OneToOneField("Statement", on_delete=models.CASCADE, related_name="symbol")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    type = TextChoicesField(choices_enum=SymbolType)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    extends = models.ManyToManyField("Symbol", related_name="extended_by", blank=True)
    parameters: models.QuerySet["SymbolParameter"]  # noqa via SymbolParameter.symbol
    arguments: models.QuerySet["SymbolArgument"]  # noqa via SymbolArgument.symbol
    source_mappings: models.QuerySet[SourceMapping]  # noqa via SourceMapping.symbol

    task = models.OneToOneField(
        "Task", on_delete=models.RESTRICT, null=True, blank=True, related_name="symbol"
    )
    expectation = models.OneToOneField(
        "Expectation", on_delete=models.RESTRICT, null=True, blank=True, related_name="symbol"
    )
    code = models.OneToOneField(
        "Code", on_delete=models.RESTRICT, null=True, blank=True, related_name="symbol"
    )
    model = models.OneToOneField(
        "Model", on_delete=models.RESTRICT, null=True, blank=True, related_name="symbol"
    )
    dataset = models.OneToOneField(
        "Dataset", on_delete=models.RESTRICT, null=True, blank=True, related_name="symbol"
    )

    def deepcopy(self, to: Symbol, refs: dict[UUID, SymbolContent | Symbol]):
        """
        Deep copy this symbol to another symbol, replacing non-symbol references
         (incl. references to symbols within non-symbol relations like args/params)
        """
        # copy content
        new_content = refs[self.content_id]
        self.content.deepcopy(to=new_content, refs=refs)
        new_content.save()

        # copy parameters
        for parameter in self.parameters.all():
            parameter.id = None
            parameter.symbol = to
            parameter.save()
        # copy arguments
        for argument in self.arguments.all():
            argument.id = None
            argument.symbol = to
            # replace ref (default to same ref if not in refs since library refs are not copied)
            new_reference = refs.get(argument.reference_id, argument.reference)
            argument.reference = cast(Symbol, new_reference)
            argument.save()

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
        only=["type", "task", "expectation", "code", "model", "dataset"],
        select_related=["task", "expectation", "code", "model", "dataset"],
    )
    def content(self) -> Union[Task, Expectation, Code, Model, Dataset]:
        content: Union[Task, Expectation, Code, Model, Dataset, None] = getattr(
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
            parameter, _ = SymbolParameter.objects.update_or_create(
                symbol=self, name=name, defaults={"type": type, "schema": schema}
            )
            return parameter
        else:
            return SymbolParameter.objects.create(symbol=self, name=name, type=type, schema=schema)

    @transaction.atomic
    def bind_argument(
        self, name: str, value: Any | Symbol, exists_ok: bool = False
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
        argument, created = SymbolArgument.objects.update_or_create(
            symbol=self,
            name=name,
            defaults=dict(type=argument_type, value=value, reference=reference),
        )
        if not created and not exists_ok:
            # (transaction will be rolled back)
            raise RuntimeError(f"{self} argument {argument} already exists")
        return parameter, argument

    def bind_arguments(self, exists_ok: bool = False, **arguments: Any | Symbol):
        for name, value in arguments.items():
            self.bind_argument(name, value, exists_ok)

    objects: SymbolManager = SymbolManager()

    class Meta:
        default_manager_name = "objects"
        # set base manager so that _every_ query to Symbol goes through SymbolManager
        # which ensures that we always select the related statement
        base_manager_name = "objects"
        ordering = ["name"]


# auto delete symbol content if symbol is deleted
@receiver(models.signals.post_delete, sender=Symbol)
def auto_delete_symbol_content(sender, instance: Symbol, **kwargs):
    if "content" in instance._state.fields_cache:
        instance.content.delete()
    else:
        # this shouldn't happen but just in case
        pass


class SymbolContentManager(models.Manager):
    # Note that this manager is applied to _every_ SymbolContent query (related or not)!

    def get_queryset(self):
        # always select related symbol
        return super().get_queryset().select_related("symbol")


class SymbolContent(UUIDModel):
    """
    The content of a symbol. This is the interface that Symbol.content points to.
    Symbol symbols are mutable until the containing project is committed.
    """

    symbol: Symbol  # noqa via Symbol.content

    @property
    def symbol_str(self) -> str:
        """Gets a symbol str for logging that handles not yet defined symbol contents"""
        # check if symbol is in model cache
        if "symbol" in self._state.fields_cache:  # type: ignore
            return str(self.symbol)
        else:
            return "<undefined>"

    @gql.model_property(only=["symbol"], select_related=["symbol"])
    def type(self) -> SymbolType:
        return self.symbol.type

    @gql.model_property(only=["symbol"], select_related=["symbol"])
    def type_name_declaration(self) -> str:
        return self.symbol.type_name_declaration

    @gql.model_property(only=["symbol"], select_related=["symbol", "symbol__parameters"])
    def parameters(self) -> models.QuerySet[SymbolParameter]:
        return self.symbol.parameters

    @gql.model_property(only=["symbol"], select_related=["symbol", "symbol__arguments"])
    def arguments(self) -> models.QuerySet[SymbolArgument]:
        return self.symbol.arguments

    def deepcopy(self, to: Any, refs: dict[UUID, Symbol | SymbolContent]):
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

        # replace all relations referencing symbols or contents with copies
        replace_refs(self, to, refs)

    objects: SymbolContentManager = SymbolContentManager()

    class Meta:
        default_manager_name = "objects"
        # set base manager so that _every_ query to SymbolContent goes through SymbolContentManager
        # which ensures that we always select the related symbol
        base_manager_name = "objects"
        abstract = True


class SymbolParameterType(models.TextChoices):
    DATA = "dataset"
    MODEL = "model"
    CODE = "code"
    VALUE = "value"

    @staticmethod
    def from_value(obj: Any | Symbol) -> SymbolParameterType:
        if isinstance(obj, Symbol):
            if obj.type == SymbolType.DATASET:
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
    A parameter is a named argument to a symbol.
    Parameters are typed using SchemaElements.

    All symbols can be parameterized, but not all symbols implement parameterization yet.
    """

    symbol = models.ForeignKey(Symbol, on_delete=models.CASCADE, related_name="parameters")
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    type = TextChoicesField(choices_enum=SymbolParameterType)
    schema = SchemaElementField(null=True, blank=True)  # models need not have a schema

    def __str__(self):
        return f"parameter {self.name}(type={self.type})"

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
    Arguments can be bound directly to a symbol (for definitions) or to a statement (for references)
    """

    symbol = models.ForeignKey(
        Symbol, on_delete=models.CASCADE, null=True, blank=True, related_name="arguments"
    )
    statement = models.ForeignKey(
        Statement, on_delete=models.CASCADE, null=True, blank=True, related_name="arguments"
    )
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    type = models.CharField(max_length=64, choices=SymbolParameterType.choices)
    reference = models.ForeignKey("Symbol", on_delete=models.CASCADE, null=True, blank=True)
    value = models.JSONField(null=True, blank=True)

    def __str__(self):
        if self.value:
            content_str = f"value={self.value}"
        elif self.reference:
            content_str = f"reference={self.reference}"
        else:
            content_str = "<undefined>"
        return f"{self.name}(type={self.type}, {content_str})"

    class Meta:
        constraints = [
            # ensure that only one name is set (per symbol or statement)
            models.UniqueConstraint(
                name="bench_symbol_argument_symbol_name_ak",
                fields=["symbol", "name"],
                condition=models.Q(symbol__isnull=False),
            ),
            models.UniqueConstraint(
                name="bench_symbol_argument_statement_name_ak",
                fields=["statement", "name"],
                condition=models.Q(statement__isnull=False),
            ),
        ]


def replace_refs(
    obj: models.Model,
    to: models.Model,
    refs: dict[UUID, Symbol | SymbolContent],
    include_one_to_many: bool = True,
    include_many_to_many: bool = True,
):
    """Replaces all references to symbols with the given refs (refs need not be complete)."""
    for field in obj._meta.get_fields():
        if field.related_model is None:
            continue
        if not issubclass(field.related_model, (Symbol, SymbolContent)):
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
