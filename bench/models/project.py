from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union, cast

import pytz
from django.core.validators import validate_slug
from django.db import connection, models, transaction
from django_choices_field import TextChoicesField

from bench.models import Organization
from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel

if TYPE_CHECKING:
    from bench.models.dataset import Dataset
    from bench.models.instruction import Instruction
    from bench.models.model import Model
    from bench.models.task import Task


class ProjectType(models.TextChoices):
    EXECUTABLE = "executable", "Executable"
    LIBRARY = "library", "Library"


class ProjectManager(models.Manager):
    @transaction.atomic
    def create_project(
        self,
        organization: Organization,
        name: str,
        slug: str,
        type: ProjectType = ProjectType.EXECUTABLE,
    ):
        project: Project = cast(
            Project, super().create(organization=organization, name=name, slug=slug, type=type)
        )
        project.head = ProjectVersion.objects.create(project=project)
        project.save()
        return project

    def get_by_slug(self, organization: str, project: str):
        return self.get(organization__slug=organization, slug=project)


class Project(TaggableMixin, UUIDModel):
    """
    A project to instruct an AI to do something.

    Projects are the root of versioning, similar to repositories in Git.
    All versions are available in 'versions' and may not be linear (also like in Git).
    Project "files" (the contents of the project) are copy-on-write.

    Executable projects have a main program (the top-level task & instruction implementations).
    Library projects define reusable objects (like in software).
    """

    type = TextChoicesField(choices_enum=ProjectType, default=ProjectType.EXECUTABLE)
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    slug: models.SlugField = models.SlugField(max_length=128, validators=[validate_slug])
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    # TODO @Feature: use basic branching, move Project head into main_branch.head
    head = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, null=True, related_name="project+"
    )
    # branches via ProjectBranch

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="projects"
    )

    def __str__(self):
        return f"{self.organization.slug}/{self.slug}"

    @transaction.atomic
    def create_version(
        self,
        name: Optional[str] = None,
        description: Optional[str] = None,
        parent: Optional[ProjectVersion] = None,
        auto_commit: bool = True,
    ) -> "ProjectVersion":
        if parent is None:
            if self.head is None:
                raise ValueError(f"project does not have a head version: {self}")
            assigned_parent = self.head
        else:
            assigned_parent = parent
        del parent  # avoid accidental use

        if not assigned_parent.is_committed:
            if auto_commit:
                assigned_parent.commit()
            else:
                raise ValueError(f"parent version must be committed: {assigned_parent}")

        version = ProjectVersion.objects.create(project=self, name=name, description=description)
        version.parents.add(assigned_parent)

        # copy all project files from parent in SQL (see ProjectFile model below)
        # the parent project is committed, so we can safely use file references
        cursor = connection.cursor()
        cursor.execute(
            """
INSERT INTO bench_projectfile
(id, project_version_id, type, name, task_id, instruction_id, model_id, dataset_id)
SELECT gen_random_uuid(), %s, type, name, task_id, instruction_id, model_id, dataset_id
FROM bench_projectfile
WHERE project_version_id = %s
""",
            [version.id, assigned_parent.id],
        )

        # head has advanced to new version
        if assigned_parent == self.head:
            self.head = version

        return version

    objects: ProjectManager = ProjectManager()

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_project_organization_slug_ak",
                fields=["organization", "slug"],
            )
        ]


class ProjectBranch(UUIDModel):
    """
    A branch of a project, like in Git. A branch is a pointer to a project version.
    """

    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    project: models.ForeignKey = models.ForeignKey(
        "Project", on_delete=models.CASCADE, related_name="branches"
    )
    head: models.ForeignKey = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="branches+"
    )

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_project_branch_project_name_ak",
                fields=["project", "name"],
            )
        ]


class ProjectVersion(TaggableMixin, UUIDModel):
    """
    A project version records the state of a project at a specific point in time.
    """

    project = models.ForeignKey(Project, on_delete=models.CASCADE, related_name="versions")
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    committed_at = models.DateTimeField(null=True)

    parents = models.ManyToManyField(
        "ProjectVersion", related_name="children", symmetrical=False, blank=True
    )
    # files via ProjectFile
    program: models.ForeignKey = models.ForeignKey(
        "Task", on_delete=models.CASCADE, null=True, related_name="projects"
    )
    backends: models.ManyToManyField = models.ManyToManyField(
        "Model", related_name="referenced_in_projects+", blank=True
    )

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.project.slug}@{self.id.hex}"

    def reset(self):
        # TODO @Robustness: reset will fail if other versions are referencing some of the same files
        self.files.all().delete()

    def commit(self, name: Optional[str] = None):
        self.committed_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        self.name = name
        self.save()

    @property
    def is_committed(self):
        return self.committed_at is not None

    @property
    def organization(self):
        return self.project.organization


class File(UUIDModel):
    """
    A file defining symbols for a project version, potentially containing other files if it's a folder.
    """

    project_version: models.ForeignKey = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)

    is_folder = models.BooleanField(default=False)
    parent = models.ForeignKey("File", on_delete=models.CASCADE, null=True, related_name="files")
    # definitions via SymbolDefinition
    # files via File (if in a folder)

    def __str__(self):
        if self.parent:
            return f"{self.parent}/{self.name}"
        else:
            return f"{self.project_version}/{self.name}"

    @property
    def root(self) -> bool:
        return self.parent is None

    @property
    def path(self) -> str:
        return f"{self.parent.path}/{self.name}" if self.parent else self.name

    class Meta:
        constraints = [
            # ensure that the path is unique per project version (includes parent folder)
            models.UniqueConstraint(
                name="bench_project_file_project_name_ak",
                fields=["project_version_id", "name"],
                condition=models.Q(parent_id__isnull=True),
            ),
            models.UniqueConstraint(
                name="bench_project_file_project_parent_name_ak",
                fields=["project_version_id", "parent_id", "name"],
                condition=models.Q(parent_id__isnull=False),
            ),
        ]


class SymbolType(models.TextChoices):
    """
    The type of symbol to define in a project.
    """

    TASK = "task", "Task"
    INSTRUCTION = "instruct", "Instruction"
    MODEL = "model", "Model"
    DATASET = "data", "Dataset"
    DATASET_VIEW = "view", "DatasetView"


class Symbol(UUIDModel):
    """
    A generic symbol of a specific immutable type.
    Symbols are used to reference tasks, instructions, models, and datasets.
    References are resolved to definitions within a project version.
    """

    project = models.ForeignKey(Project, on_delete=models.CASCADE, related_name="symbols")
    type = TextChoicesField(choices_enum=SymbolType)


class SymbolDefinition(UUIDModel):
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
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="symbols")
    index = models.IntegerField(null=True)  # index into file
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    task = models.ForeignKey("Task", on_delete=models.RESTRICT, null=True, related_name="symbol")
    instruction = models.ForeignKey(
        "Instruction", on_delete=models.RESTRICT, null=True, related_name="symbol"
    )
    model = models.ForeignKey("Model", on_delete=models.RESTRICT, null=True, related_name="symbol")
    dataset = models.ForeignKey(
        "Dataset", on_delete=models.RESTRICT, null=True, related_name="symbol"
    )
    dataset_view = models.ForeignKey(
        "DatasetView", on_delete=models.RESTRICT, null=True, related_name="symbol"
    )

    @property
    def content(self) -> Union[Task, Instruction, Model, Dataset]:
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

    class Meta:
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
