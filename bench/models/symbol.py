from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

import pytz
import structlog
from django.db import models, transaction
from django.dispatch import receiver
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.models.schema_field import SchemaElementField
from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel, is_jsonable
from bench.utils.schema import SchemaElement

if TYPE_CHECKING:
    from bench.models import (
        Code,
        Dataset,
        Expectation,
        File,
        Model,
        ProjectVersion,
        Schema,
        Task,
    )

logger = structlog.get_logger(__name__)


class StatementType(models.TextChoices):
    """
    The type of Bench statement.
    """

    IMPORT = "import"
    DEFINITION = "def"
    REFERENCE = "ref"
    COMMENT = "comment"


class StatementModifier(models.TextChoices):
    """
    A modifier to a Bench statement.
    """

    MAIN = "main"
    SUGGEST = "suggest"
    LIKE = "like"
    UNLIKE = "unlike"
    VERIFY = "verify"


class StatementManager(models.Manager["Statement"]):
    def get_queryset(self) -> models.QuerySet[Statement]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)

    def _prep_insert_index(
        self, file: File, parent: Optional[Statement], index: Optional[int]
    ) -> int:
        if index is None:
            index = parent.children.count() if parent else file.root_statements.count()
        else:
            # make space
            self.filter(file=file, parent=parent, index__gte=index).update(
                index=models.F("index") + 1
            )
        return index

    @transaction.atomic
    def create_definition(
        self,
        project_version: ProjectVersion,
        file: File,
        parent: Optional[Statement],
        index: Optional[int],
        content: SymbolContent,
        name: str,
    ) -> Statement:
        # auto set index if not passed
        index = self._prep_insert_index(file, parent, index)
        statement = self.create(
            project_version=project_version,
            file=file,
            type=StatementType.DEFINITION,
            parent=parent,
            index=index,
            name=name,
        )
        statement.symbol = Symbol.objects.create_symbol(
            project_version=project_version, file=file, definition=statement, content=content
        )
        return statement

    @transaction.atomic
    def create_import(
        self,
        project_version: ProjectVersion,
        file: File,
        parent: Optional[Statement],
        index: Optional[int],
        name: str,
        statement: Statement,
    ) -> Statement:
        # auto set index if not passed
        index = self._prep_insert_index(file, parent, index)
        return self.create(
            project_version=project_version,
            file=file,
            type=StatementType.IMPORT,
            parent=parent,
            index=index,
            name=name,
            reference=statement,
        )


class Statement(UUIDModel):
    """
    A statement in a file. Can import, define or reference a symbol.
    Statements are semantic and may be nested (parent-child relationships, comments, etc.).
    Statements and the files that contain them can be soft-deleted.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="statements"
    )
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="statements")
    type = TextChoicesField(choices_enum=StatementType)
    modifier = TextChoicesField(choices_enum=StatementModifier, null=True, blank=True)
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True, blank=True)
    commented = models.BooleanField(default=False)
    compiled = models.BooleanField(default=False)

    parent = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    children: models.QuerySet[Statement]  # noqa via Statement.parent
    index = models.IntegerField(null=True)  # index into file or parent statement

    symbol: Optional[Symbol]  # noqa via Symbol.definition
    reference = models.ForeignKey(
        "Statement",
        on_delete=models.CASCADE,
        null=True,
        blank=True,
        related_name="referenced_by",
    )
    referenced_by: models.QuerySet[Statement]  # noqa via Statement.reference
    parameters: models.QuerySet[Parameter]  # noqa via Parameter.symbol
    arguments: models.QuerySet[Argument]  # noqa via Argument.statement
    text = models.TextField(null=True, blank=True)  # as markdown

    def deepcopy(self, to: Statement, refs: dict[UUID, SymbolContent | Symbol | Statement]):
        # copy symbol
        if self.type == StatementType.DEFINITION:
            self.symbol.deepcopy(to.symbol, refs)
        # copy reference
        to.reference = refs.get(self.reference_id)
        # copy parameters
        for parameter in self.parameters.all():
            parameter.id = None
            parameter.statement = to
            parameter.save()
        # copy arguments
        for argument in self.arguments.all():
            argument.id = None
            argument.statement = to
            # replace ref (default to same ref if not in refs since library refs are not copied)
            new_reference = refs.get(argument.reference_id, argument.reference)
            argument.reference = cast(Statement, new_reference)
            argument.save()

    @property
    def siblings(self) -> models.QuerySet[Statement]:
        return self.parent.children if self.parent else self.file.root_statements

    @property
    def descendants(self) -> models.QuerySet[Statement]:
        return Statement.objects.filter(parent__in=self.children.all())

    @property
    def active_children(self) -> models.QuerySet[Statement]:
        return self.children.filter(deleted_at__isnull=True, commented=False)

    @property
    def active_descendants(self) -> models.QuerySet[Statement]:
        return self.descendants.filter(deleted_at__isnull=True, commented=False)

    @gql.model_property(only=["type"])
    def type_shortname(self) -> str:
        return self.type

    @property
    def symbol_(self) -> Symbol:
        if self.symbol is None:
            raise ValueError(f"{self} has no symbol")
        return self.symbol

    @gql.model_property(
        only=["type", "symbol", "reference"],
        select_related=["symbol", "reference", "reference__symbol"],
    )
    def source_symbol(self) -> Symbol:
        """Traverses references to get the underlying symbol (at the definition statement)."""
        if self.type == StatementType.DEFINITION:
            return self.symbol_
        elif self.reference is None:
            raise ValueError(f"{self} has no reference")
        else:
            return self.reference.symbol_

    @property
    def absolute_index(self) -> str:
        if self.parent:
            return f"{self.parent.absolute_index}.{self.index}"
        return str(self.index)

    @transaction.atomic
    def move_to(self, file: File, parent: Optional[Statement], index: Optional[int] = None) -> None:
        """Moves this statement to a new file and/or parent statement. Updates children at both the old and new locations."""
        # reload self to get the latest location within transaction
        self.refresh_from_db(fields=["file", "parent", "index"])

        old_siblings = self.siblings
        siblings = parent.children if parent else file.root_statements
        if index is None:
            # if index not passed then insert at the end
            index = siblings.count()

        if self.file == file and self.parent == parent:
            # if file and parent are the same just swap
            if self.index == index:
                # if index is the same then do nothing
                return
            other_statement = siblings.get(index=index)
            self.index, other_statement.index = other_statement.index, self.index
            self.save()
            other_statement.save()
            print(f"swap {self} with {other_statement}")
        else:  # remove from old location and insert at new location
            # make space at new location
            siblings.filter(index__gte=index).update(index=models.F("index") + 1)
            # fill space at old location
            old_siblings.filter(index__gt=self.index).update(index=models.F("index") - 1)
            # update self
            self.file = file
            self.parent = parent
            self.index = index
            self.save()

    @transaction.atomic
    def soft_delete(self):
        self.deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        # soft delete descendants
        self.descendants.update(deleted_at=self.deleted_at)
        # move siblings up
        self.siblings.filter(index__gt=self.index).update(index=models.F("index") - 1)
        self.save()

    @transaction.atomic
    def restore(self):
        self.refresh_from_db(fields=["deleted_at", "file"])
        if self.file.deleted_at:
            raise ValueError(f"cannot restore {self} because containing {self.file} is deleted")
        self.deleted_at = None
        # restore descendants
        self.descendants.update(deleted_at=None)
        # move siblings down
        # TODO @Robustness: statement restore assumes siblings were not changed - correct?
        self.siblings.filter(index__gte=self.index).update(index=models.F("index") + 1)
        self.save()

    @transaction.atomic
    def set_commented(self, commented: bool):
        """Sets the commented flag on this statement and all descendants."""
        self.commented = commented
        self.descendants.update(commented=commented)
        self.save()

    def add_child(
        self, type: StatementType, content: Symbol, modifier: Optional[StatementModifier] = None
    ) -> Statement:
        index = self.children.count()
        kwargs = (
            {"symbol": content}
            if type == StatementType.DEFINITION
            else {"reference": content.definition}
        )
        return Statement.objects.create(
            project_version=self.project_version,
            file=self.file,
            type=type,
            parent=self,
            index=index,
            modifier=modifier,
            **kwargs,
        )

    def children_of_symbol_type(self, symbol_type: SymbolType) -> models.QuerySet[Statement]:
        return self.children.filter(symbol__type=symbol_type)

    def child_of_symbol_type(self, symbol_type: SymbolType) -> Optional[Statement]:
        return self.children_of_symbol_type(symbol_type).first()

    def add_parameter(
        self,
        name: str,
        type: ParameterType,
        exists_ok: bool = False,
        schema: Optional[SchemaElement] = None,
    ) -> Parameter:
        if exists_ok:
            parameter, _ = Parameter.objects.update_or_create(
                statement=self, name=name, defaults={"type": type, "schema": schema}
            )
            return parameter
        else:
            return Parameter.objects.create(statement=self, name=name, type=type, schema=schema)

    @transaction.atomic
    def bind_argument(
        self, name: str, value: Any | Statement, exists_ok: bool = False
    ) -> tuple[Parameter, Argument]:
        if value is None:
            raise ValueError(f"cannot bind {self} argument {name} to None")
        if isinstance(value, Statement) and value.file_id != self.file_id:
            raise ValueError(f"cannot bind {self} argument {name} to {value} in different file")

        # get/create parameter and corresponding argument
        argument_type = ParameterType.from_value(value)
        if argument_type == ParameterType.VALUE:
            value, reference = value, None
        else:
            value, reference = None, value

        parameter = self.add_parameter(name, argument_type, exists_ok=True)
        argument, created = Argument.objects.update_or_create(
            statement=self,
            name=name,
            defaults=dict(value=value, reference=reference),
        )
        if not created and not exists_ok:
            # (transaction will be rolled back)
            raise RuntimeError(f"{self} argument {argument} already exists")
        return parameter, argument

    def bind_arguments(self, exists_ok: bool = False, **arguments: Any | Statement):
        for name, value in arguments.items():
            self.bind_argument(name, value, exists_ok)

    def __str__(self):
        path = self.file.path + ":" + str(self.absolute_index)
        if self.type == StatementType.DEFINITION:
            content_str = f"{self.symbol}"
        elif self.type in (StatementType.IMPORT, StatementType.REFERENCE):
            content_str = f"{self.reference}"
        elif self.type == StatementType.COMMENT:
            content_str = f"{len(self.text)}"
        else:
            raise ValueError(f"unknown statement type {self.type}")
        modifier_str = f" {self.modifier}" if self.modifier else ""
        name_str = f" {self.name}" if self.name else ""
        return f"{path}{modifier_str} {self.type}{name_str} {content_str}"

    objects: StatementManager = StatementManager()

    class Meta:
        ordering = ["index"]
        default_manager_name = "objects"
        # TODO @Robustness: unique constraint on index when we switch to fractional indexes


class SymbolType(models.TextChoices):
    """
    The type of symbol to define in a project.
    """

    SCHEMA = "schema", "Schema"
    TASK = "task", "Task"
    EXPECTATION = "expect", "Expectation"
    CODE = "code", "Code"
    MODEL = "model", "Model"
    DATASET = "data", "Dataset"

    @staticmethod
    def from_content(content: SymbolContent) -> SymbolType:
        # re-import for real to avoid circular import (above is only for type checking)
        from bench.models import Code, Dataset, Expectation, Model, Schema, Task  # noqa

        if isinstance(content, Schema):
            return SymbolType.SCHEMA
        elif isinstance(content, Task):
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
        return super().get_queryset().select_related("definition")

    def create_symbol(
        self,
        content: SymbolContent,
        project_version: ProjectVersion,
        file: File,
        definition: Statement,
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
            definition=definition,
            type=content_type,
            **kwargs,
        )


SYMBOL_TYPE_TO_FIELD = {
    SymbolType.SCHEMA: "schema",
    SymbolType.TASK: "task",
    SymbolType.EXPECTATION: "expectation",
    SymbolType.CODE: "code",
    SymbolType.MODEL: "model",
    SymbolType.DATASET: "dataset",
}
SYMBOL_CONTENT_FIELDS = set(SYMBOL_TYPE_TO_FIELD.values())


class Symbol(TaggableMixin, UUIDModel):
    """
    A symbol defining a Bench primitive.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="symbols"
    )
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="symbols")
    definition = models.OneToOneField("Statement", on_delete=models.CASCADE, related_name="symbol")
    type = TextChoicesField(choices_enum=SymbolType)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    extends = models.ManyToManyField("Symbol", related_name="extended_by", blank=True)

    schema = models.OneToOneField(
        "Schema", on_delete=models.RESTRICT, null=True, blank=True, related_name="symbol"
    )
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

    def __str__(self):
        return f"{self.type}@{self.id.hex}"

    @property
    def path(self) -> str:
        return self.definition.path

    @staticmethod
    def type_to_field(type: SymbolType) -> str:
        return SYMBOL_TYPE_TO_FIELD[type]

    @gql.model_property(only=["type"])
    def type_shortname(self) -> str:
        return self.type

    @gql.model_cached_property(
        only=["type", "schema", "task", "expectation", "code", "model", "dataset"],
        select_related=["schema", "task", "expectation", "code", "model", "dataset"],
    )
    def content(self) -> Union[Schema, Task, Expectation, Code, Model, Dataset]:
        content: Union[Schema, Task, Expectation, Code, Model, Dataset, None] = getattr(
            self, self.type_to_field(self.type)
        )
        if content is None:
            raise ValueError(f"{self} has no content for {self.type}")
        return content

    def set_content(self, content: SymbolContent | None):
        setattr(self, self.type_to_field(self.type), content)

    def extend(self, symbol: Symbol):
        self.extends.add(symbol)

    @property
    def task_(self) -> Task:
        if TYPE_CHECKING:
            return cast(Task, self.content)
        else:
            return self.content  # noqa

    @property
    def schema_(self) -> Schema:
        if TYPE_CHECKING:
            return cast(Schema, self.content)
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

    objects: SymbolManager = SymbolManager()

    class Meta:
        default_manager_name = "objects"
        # set base manager so that _every_ query to Symbol goes through SymbolManager
        # which ensures that we always select the related statement
        base_manager_name = "objects"


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
    def definition(self) -> Statement:
        return self.symbol.definition

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

    @property
    def parameters(self) -> models.QuerySet[Parameter]:
        return self.symbol.definition.parameters

    @property
    def arguments(self) -> models.QuerySet[Argument]:
        return self.symbol.definition.arguments

    def deepcopy(self, to: Any, refs: dict[UUID, Symbol | SymbolContent]):
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


class ParameterType(models.TextChoices):
    DATA = "dataset"
    MODEL = "model"
    CODE = "code"
    VALUE = "value"

    @staticmethod
    def from_value(obj: Any | Statement) -> ParameterType:
        if isinstance(obj, Statement):
            if obj.source_symbol.type == SymbolType.DATASET:
                return ParameterType.DATA
            elif obj.source_symbol.type == SymbolType.MODEL:
                return ParameterType.MODEL
            elif obj.source_symbol.type == SymbolType.CODE:
                return ParameterType.CODE
            else:
                raise ValueError(f"unexpected symbol type for code parameter: {obj}")
        elif is_jsonable(obj):
            return ParameterType.VALUE
        else:
            raise ValueError(f"unknown object {obj} to code parameter")


class Parameter(UUIDModel):
    """
    A parameter is a named argument to a statement typed with Schemas.

    All statements can be parameterized, but not all symbols support every argument type.
    """

    statement = models.ForeignKey(Statement, on_delete=models.CASCADE, related_name="parameters")
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    type = TextChoicesField(choices_enum=ParameterType)
    schema = SchemaElementField(null=True, blank=True)  # models need not have a schema

    def __str__(self):
        return f"{self.name}: {self.type}"

    class Meta:
        ordering = ["name"]
        constraints = [
            models.UniqueConstraint(
                name="bench_parameter_statement_name_ak",
                fields=["statement", "name"],
            )
        ]


class Argument(UUIDModel):
    """
    An argument is a bound value to some parameter, either a symbol through a statement or JSON.
    Arguments are bound to statements.
    """

    statement = models.ForeignKey(Statement, on_delete=models.CASCADE, related_name="arguments")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    reference = models.ForeignKey("Statement", on_delete=models.CASCADE, null=True, blank=True)
    value = models.JSONField(null=True, blank=True)

    def __str__(self):
        if self.value:
            content_str = f"value={self.value}"
        elif self.reference:
            content_str = f"reference={self.reference}"
        else:
            content_str = "<undefined>"
        return f"{self.name}={content_str}"

    class Meta:
        ordering = ["name"]
        constraints = [
            models.UniqueConstraint(
                name="bench_argument_statement_name_ak",
                fields=["statement", "name"],
            )
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
