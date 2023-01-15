from __future__ import annotations

import contextlib
from datetime import datetime
from typing import TYPE_CHECKING, Optional
from uuid import UUID

import pytz
import structlog
from django.db import models, transaction
from django.db.models import Q
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench import language
from bench.language.type import (
    MOCK_FILE,
    MOCK_STATEMENT,
    StatementModifier,
    StatementType,
    SymbolType,
    get_default_symbol_content,
)
from bench.models.data import DatasetContentMixin
from bench.models.model import ModelContentMixin
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel

if TYPE_CHECKING:
    from bench.models import File, ProjectVersion

logger = structlog.get_logger(__name__)


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


# sync with actual symbol content fields of Statement
SYMBOL_CONTENT_VALUE_FIELDS = {
    "code",
    "code_builtin_id",
    "description",
    "btl",
    "value",
}
SYMBOL_CONTENT_RELATION_1TOM_FIELDS = {
    "reference_project_version",
}
SYMBOL_CONTENT_RELATION_MTOM_FIELDS = {
    "mappings",
    "compilations",
}


class Statement(UUIDModel, DatasetContentMixin, ModelContentMixin):
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
    text = models.TextField(null=True, blank=True)  # for comment
    # symbol contents (sync with SYMBOL_CONTENT_FIELDS)
    code = models.TextField(null=True, blank=True)  # for code content
    code_builtin_id = models.CharField(
        max_length=MAX_NAME_LENGTH, null=True, blank=True
    )  # for code content
    description = models.TextField(null=True, blank=True)  # for task and expectation content
    reference_project_version = models.ForeignKey(  # for requirement
        "ProjectVersion", on_delete=models.SET_NULL, null=True, blank=True
    )
    value = models.JSONField(null=True, blank=True)  # for value
    btl = models.TextField(null=True, blank=True)  # for any type nodes
    # ... other contents currently via DatasetContentMixin and ModelContentMixin

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

    def content(self) -> Optional[language.SymbolContent]:
        from bench.models.mapper import rmap_statement  # avoid circular import

        if self.symbol_type is None:
            return None
        try:
            lang_statement = rmap_statement(self, MOCK_FILE)
            return lang_statement.content
        except ValueError as e:  # invalid/partial content
            return None

    def content_(self) -> language.SymbolContent:
        content = self.content
        if content is None:
            raise ValueError(f"{self} has no content for {self.symbol_type}")
        return content

    def set_content(self, content: language.SymbolContent | None):
        from bench.models.mapper import wmap_symbol  # avoid circular import

        if content is None:
            self.symbol_type = None
        else:
            self.symbol_type = content.type
            wmap_symbol(self, content)

    @gql.model_property(only=["type", "reference"], select_related=["reference"])
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

    def morph_to(
        self,
        type: StatementType,
        symbol_type: SymbolType | None,
    ):
        from bench.models.mapper import wmap_symbol  # avoid circular import

        """Changes the type of the statement without clearing any old content fields."""
        self.type = type
        # set default content if not already set
        self.symbol_type = symbol_type
        # create default content for the given type if not already set
        if (
            self.symbol_type is not None
            and self.type == StatementType.DEFINITION
            and self.content is None  # 'content' is dynamically created on symbol_type
        ):
            lang_symbol = get_default_symbol_content(MOCK_STATEMENT, symbol_type)
            # ignore created relations since defaults are empty
            _ = wmap_symbol(self, lang_symbol)
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
