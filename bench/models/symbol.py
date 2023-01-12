from __future__ import annotations

import contextlib
from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Type, Union, cast
from uuid import UUID

import cachetools
import pytz
import structlog
from django.db import models, transaction
from django.db.models import Q
from django.dispatch import receiver
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.language.type import StatementModifier, StatementType, SymbolType
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel

if TYPE_CHECKING:
    from bench.models import (
        Code,
        Compilation,
        Dataset,
        Expectation,
        File,
        Model,
        ProjectVersion,
        Schema,
        Task,
        Value,
    )

logger = structlog.get_logger(__name__)


# Symbol type helpers


@cachetools.cached({})
def symbol_type_to_content_type_map():
    # re-import for real to avoid circular import (above is only for type checking)
    from bench.models import Code, Dataset, Expectation, Model, Schema, Task, Value  # noqa

    return {
        SymbolType.SCHEMA: Schema,
        SymbolType.TASK: Task,
        SymbolType.EXPECTATION: Expectation,
        SymbolType.CODE: Code,
        SymbolType.MODEL: Model,
        SymbolType.DATASET: Dataset,
        SymbolType.VALUE: Value,
    }


def symbol_type_to_content_type(type: SymbolType) -> Type[SymbolContent]:
    content_type = symbol_type_to_content_type_map()[type]
    if content_type is None:
        raise ValueError(f"unexpected symbol type {type}")
    return content_type


def content_type_to_symbol_type(content_type: Type[SymbolContent]) -> SymbolType:
    content_type_type_map = {k: v for v, k in symbol_type_to_content_type_map().items()}
    if content_type not in content_type_type_map:
        raise ValueError(f"unexpected symbol content type {content_type}")
    return content_type_type_map[content_type]


def symbol_type_from_content(content: SymbolContent) -> SymbolType:
    return content_type_to_symbol_type(type(content))


def get_default_symbol_content(symbol_type: SymbolType) -> SymbolContent:
    cls = symbol_type_to_content_type(symbol_type)
    return cls()


class StatementManager(models.Manager["Statement"]):
    def get_queryset(self) -> models.QuerySet[Statement]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)

    @transaction.atomic
    def create_statement(
        self,
        project_version: ProjectVersion,
        file: File,
        parent: Optional[Statement],
        index: Optional[int],
        type: StatementType,
        name: Optional[str],
        **kwargs,
    ) -> Statement:
        # auto set index if not passed
        if index is None:
            index = parent.children.count() if parent else file.root_statements.count()
        else:
            # make space
            self.filter(file=file, parent=parent, index__gte=index).update(
                index=models.F("index") + 1
            )
        return self.create(
            project_version=project_version,
            file=file,
            parent=parent,
            index=index,
            type=type,
            name=name,
            **kwargs,
        )

    def create_definition(
        self,
        project_version: ProjectVersion,
        file: File,
        parent: Optional[Statement],
        index: Optional[int],
        content: SymbolContent,
        name: str,
    ) -> Statement:
        # save content if it's not loaded from the db
        # (don't test via pk since we set that automatically)
        if content._state.adding:
            content.save()
        symbol_type = symbol_type_from_content(content)
        return self.create_statement(
            project_version=project_version,
            file=file,
            type=StatementType.DEFINITION,
            parent=parent,
            index=index,
            name=name,
            symbol_type=symbol_type,
            **{SYMBOL_TYPE_TO_FIELD[symbol_type]: content},
        )

    def create_import(
        self,
        project_version: ProjectVersion,
        file: File,
        parent: Optional[Statement],
        index: Optional[int],
        name: str,
        statement: Statement,
    ) -> Statement:
        return self.create_statement(
            project_version=project_version,
            file=file,
            parent=parent,
            index=index,
            type=StatementType.IMPORT,
            name=name,
            reference=statement,
            symbol_type=statement.symbol_type,
        )

    def create_reference(
        self,
        project_version: ProjectVersion,
        file: File,
        parent: Optional[Statement],
        index: Optional[int],
        name: str,
        statement: Statement,
    ) -> Statement:
        return self.create_statement(
            project_version=project_version,
            file=file,
            type=StatementType.REFERENCE,
            symbol_type=statement.symbol_type,
            parent=parent,
            index=index,
            name=name,
            reference=statement,
        )


SYMBOL_TYPE_TO_FIELD = {
    SymbolType.SCHEMA: "schema",
    SymbolType.TASK: "task",
    SymbolType.EXPECTATION: "expectation",
    SymbolType.CODE: "code",
    SymbolType.MODEL: "model",
    SymbolType.DATASET: "dataset",
    SymbolType.VALUE: "value",
}
SYMBOL_CONTENT_FIELDS = set(SYMBOL_TYPE_TO_FIELD.values())
CONTENT_FIELDS = SYMBOL_CONTENT_FIELDS | {"compilation", "requirement", "runconfig"}


class Statement(UUIDModel):
    """
    A statement in a file to import, define, redefine, reference, comment.. symbols.
    Statements are semantic and may be nested (parent-child relationships, comments, etc.).
    Statements and the files that contain them can be soft-deleted.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="statements"
    )
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="statements")
    revision = models.IntegerField(default=1)
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

    symbol_type = TextChoicesField(choices_enum=SymbolType, null=True, blank=True)
    reference = models.ForeignKey(
        "Statement",
        on_delete=models.SET_NULL,
        null=True,
        blank=True,
        related_name="referenced_by",
    )
    reference_id: Optional[UUID]  # noqa via Statement.reference
    referenced_by: models.QuerySet[Statement]  # noqa via Statement.reference
    text = models.TextField(null=True, blank=True)  # as markdown
    schema = models.OneToOneField(
        "Schema", on_delete=models.RESTRICT, null=True, blank=True, related_name="definition"
    )
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
    value = models.OneToOneField(
        "Value", on_delete=models.RESTRICT, null=True, blank=True, related_name="definition"
    )
    compilation = models.OneToOneField(
        "Compilation", on_delete=models.RESTRICT, null=True, blank=True, related_name="definition"
    )
    compilation_id: Optional[UUID]  # noqa via Statement.compilation
    requirement = models.OneToOneField(
        "Requirement", on_delete=models.RESTRICT, null=True, blank=True, related_name="definition"
    )
    requirement_id: Optional[UUID]  # noqa via Statement.requirement
    runconfig = models.OneToOneField(
        "RunConfiguration",
        on_delete=models.RESTRICT,
        null=True,
        blank=True,
        related_name="definition",
    )
    runconfig_id: Optional[UUID]  # noqa via Statement.run

    def deepcopy(
        self,
        to: Statement,
        refs: dict[UUID, SymbolContent | Compilation | Requirement | RunConfiguration | Statement],
    ):
        # copy contents
        if self.type == StatementType.DEFINITION and self.content is not None:
            self.content.deepcopy(to=(refs[self.content_id]), refs=refs)
            refs[self.content_id].save()
        if self.type == StatementType.COMPILATION and self.compilation is not None:
            self.compilation.deepcopy(to=(refs[self.compilation_id]), refs=refs)
            refs[self.compilation_id].save()
        if self.type == StatementType.REQUIREMENT and self.requirement is not None:
            self.requirement.deepcopy(to=(refs[self.requirement_id]), refs=refs)
            refs[self.requirement_id].save()
        if self.type == StatementType.RUNCONFIG and self.runconfig is not None:
            self.runconfig.deepcopy(to=(refs[self.runconfig_id]), refs=refs)
            refs[self.runconfig_id].save()

        # reset symbol type since set_content nulls it
        to.symbol_type = self.symbol_type
        # replace ref (default to same ref if not in refs since library refs are not copied)
        to.reference = refs.get(self.reference_id, self.reference)

    def __str__(self):
        path = self.file.path + ":" + str(self.absolute_index)
        if self.type == StatementType.DEFINITION:
            content_str = f"{self.content}"
        elif self.type in (StatementType.IMPORT, StatementType.REFERENCE):
            content_str = f"{self.reference}"
        elif self.type == StatementType.COMMENT:
            content_str = f"{len(self.text)}"
        elif self.type == StatementType.BLANK:
            content_str = ""
        elif self.type == StatementType.REQUIREMENT:
            content_str = str(self.requirement)
        elif self.type == StatementType.COMPILATION:
            content_str = str(self.compilation)
        elif self.type == StatementType.RUNCONFIG:
            content_str = str(self.runconfig)
        else:
            raise ValueError(f"unknown statement type {self.type}")
        modifier_str = f" {self.modifier}" if self.modifier else ""
        return f"{path}{modifier_str} {self.type} {self.symbol_type} {self.name} {content_str}"

    @contextlib.contextmanager
    def edit(self):
        """Edits a statement and saves it with a new revision when done"""
        with transaction.atomic():
            yield self
            self.revision += 1
            self.save()

    @property
    def absolute_index(self) -> str:
        if self.parent:
            return f"{self.parent.absolute_index}.{self.index}"
        return str(self.index)

    @property
    def siblings(self) -> models.QuerySet[Statement]:
        return self.parent.children if self.parent else self.file.root_statements

    @property
    def descendants(self) -> models.QuerySet[Statement]:
        # TODO @Broken: get all descendants, not just children of children (recursive)
        return Statement._base_manager.filter(
            Q(parent=self) | Q(parent__parent=self) | Q(parent__parent__parent=self)
        )

    @property
    def active_children(self) -> models.QuerySet[Statement]:
        return self.children.filter(deleted_at__isnull=True, commented=False)

    @property
    def active_descendants(self) -> models.QuerySet[Statement]:
        return self.descendants.filter(deleted_at__isnull=True, commented=False)

    def active_children_like(self, symbol_type: SymbolType) -> models.QuerySet[Statement]:
        return self.children.filter(deleted_at=None, commented=False, symbol_type=symbol_type)

    def active_child_like(self, symbol_type: SymbolType) -> Optional[Statement]:
        return self.active_children_like(symbol_type=symbol_type).first()

    @staticmethod
    def symbol_type_to_field(type: SymbolType) -> str:
        return SYMBOL_TYPE_TO_FIELD[type]

    @gql.model_property(
        only=["symbol_type", *SYMBOL_CONTENT_FIELDS],
        select_related=list(SYMBOL_CONTENT_FIELDS),
    )
    def content(self) -> Union[None, Schema, Task, Expectation, Code, Model, Dataset, Value]:
        if self.symbol_type is None:
            return None
        content: SymbolContent | None = getattr(self, self.symbol_type_to_field(self.symbol_type))
        return content

    def content_(self) -> SymbolContent:
        content = self.content
        if content is None:
            raise ValueError(f"{self} has no content for {self.symbol_type}")
        return content

    def set_content(self, content: SymbolContent | None):
        if content is None:
            self.symbol_type = None
        else:
            self.symbol_type = symbol_type_from_content(content)
            setattr(self, self.symbol_type_to_field(self.symbol_type), content)

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

    @gql.model_property(
        only=["type", "reference"],
        select_related=["reference"],
    )
    def source_definition(self) -> Statement:
        """Traverses references to get the source definition."""
        if self.type == StatementType.DEFINITION:
            return self
        elif self.reference is None:
            raise ValueError(f"{self} has no reference")
        else:
            return self.reference.source_definition

    @transaction.atomic
    def move_to(self, file: File, parent: Optional[Statement], index: Optional[int] = None) -> None:
        """Moves this statement to a new file and/or parent statement. Updates children at both the old and new locations."""
        # check that we're keeping import semantics: can only refer to statements in the same file
        if (
            self.type == StatementType.REFERENCE
            and self.reference is not None
            and self.reference.file != file
        ):
            raise ValueError(f"can't move reference {self} to file {file}")

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
    def morph_to(
        self,
        type: StatementType,
        symbol: SymbolType | SymbolContent | None,
    ):
        """Changes the type of the statement. Only works if it doesn't have any content."""
        if self.type == StatementType.DEFINITION and self.content is not None:
            # we can't morph existing content because that would make it impossible to undo this operation
            # instead, we delete the content and create a new statement in its place on the frontend
            raise ValueError(f"{self} has content and cannot be morphed")
        self.type = type
        # delete existing content
        if self.content is not None:
            content = self.content
            self.set_content(None)
            content.delete()
        # set new content
        if symbol is not None:
            if self.type == StatementType.DEFINITION:
                if isinstance(symbol, SymbolType):
                    # create default content for the given type
                    symbol = get_default_symbol_content(symbol)
                    symbol.save()
                self.set_content(symbol)
                # not sure about setting reference on morph, should be another atomic operation?
                self.reference = None
            else:
                if isinstance(symbol, SymbolContent):
                    raise ValueError("can't set content for a reference")
                self.symbol_type = symbol
            self.text = None  # clear text
        else:
            self.symbol_type = None
        self.save()

    @transaction.atomic
    def soft_delete(self):
        self.deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        # soft delete descendants (that aren't yet deleted)
        self.descendants.filter(deleted_at=None).update(deleted_at=self.deleted_at)
        # move siblings up
        self.siblings.filter(index__gt=self.index).update(index=models.F("index") - 1)
        self.save()

    @transaction.atomic
    def restore(self):
        self.refresh_from_db(fields=["deleted_at", "file"])
        if self.file.deleted_at:
            raise ValueError(f"cannot restore {self} because containing {self.file} is deleted")
        # restore descendants (that were deleted at the same time)
        self.descendants.filter(deleted_at=self.deleted_at).update(deleted_at=None)
        self.deleted_at = None
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

    objects: StatementManager = StatementManager()

    class Meta:
        ordering = ["index"]
        default_manager_name = "objects"
        # TODO @Robustness: unique constraint on index when we switch to fractional indexes
        # no constraint on contents since statements may be partially defined
        #  (during creation, editing and after reference deletion)
        constraints = [
            # if reference is set symbol type must also be set
            models.CheckConstraint(
                check=models.Q(reference__isnull=True) | models.Q(symbol_type__isnull=False),
                name="bench_statement_reference_symbol_type_set",
            ),
            # if reference is set name must also be set
            models.CheckConstraint(
                check=models.Q(reference__isnull=True) | models.Q(name__isnull=False),
                name="bench_statement_reference_name_set",
            ),
        ]


# auto delete symbol content if statement is deleted
@receiver(models.signals.post_delete, sender=Statement)
def auto_delete_symbol_content(sender, instance: Statement, **kwargs):
    if "content" in instance._state.fields_cache:
        instance.content.delete()
    else:
        # this shouldn't happen but just in case
        pass


class SymbolContentManager(models.Manager):
    # Note that this manager is applied to _every_ SymbolContent query (related or not)!

    def get_queryset(self):
        # always select related symbol
        return super().get_queryset().select_related("definition")


class SymbolContent(UUIDModel):
    """
    The content of a symbol definition. This is the interface that Statement.content points to.
    Mutable until the containing project version is committed.
    """

    definition: Statement  # noqa via Statement.content

    @property
    def type(self) -> SymbolType:
        return symbol_type_from_content(self)

    def deepcopy(self, to: Any, refs: dict[UUID, Statement | SymbolContent]):
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


class Requirement(UUIDModel):
    """
    A requirement sets the version to use for a specific library (project).
    The derived set of dependencies is copied into ProjectVersion.dependencies for lookup speed.
    """

    project_version = models.ForeignKey("ProjectVersion", on_delete=models.CASCADE)
    definition: Statement  # noqa via Statement.requirement

    def deepcopy(self, to, refs: dict[str, Any]):
        pass  # nothing to do


class RunConfiguration(UUIDModel):
    """Configuration to run executable statements."""

    definition: Statement  # noqa via Statement.run

    def deepcopy(self, to, refs: dict[str, Any]):
        pass  # nothing to do


def replace_refs(
    obj: models.Model,
    to: models.Model,
    refs: dict[UUID, Statement | SymbolContent | Compilation | Requirement | RunConfiguration],
    include_one_to_many: bool = True,
    include_many_to_many: bool = True,
):
    from bench.models import Compilation, Requirement  # avoid circular import

    """Replaces all references to symbols with the given refs (refs need not be complete)."""
    for field in obj._meta.get_fields():
        if field.related_model is None:
            continue
        if not issubclass(
            field.related_model,
            (Statement, SymbolContent, Compilation, Requirement, RunConfiguration),
        ):
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
