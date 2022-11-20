from __future__ import annotations

from datetime import datetime
from typing import Optional

import pytz
from django.core.validators import validate_slug
from django.db import connection, models, transaction
from social_core.utils import slugify

from bench.models import Organization
from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class ProjectType(models.TextChoices):
    EXECUTABLE = "executable", "Executable"
    LIBRARY = "library", "Library"


class ProjectManager(models.Manager):
    @transaction.atomic
    def create_project(
        self,
        organization: Organization,
        name: str,
        slug: Optional[str],
        type: ProjectType = ProjectType.EXECUTABLE,
    ) -> "Project":
        if not slug:
            slug = slugify(name)
        project = super().create(organization=organization, name=name, slug=slug, type=type)
        project.head = ProjectVersion.objects.create(project=project)
        project.save()
        return project

    def get_by_slug(self, organization: str, project: str) -> "Project":
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

    type = models.CharField(
        max_length=64, choices=ProjectType.choices, default=ProjectType.EXECUTABLE
    )
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

    @transaction.atomic
    def create_version(
        self,
        name: str = None,
        description: str = None,
        parent: ProjectVersion = None,
        auto_commit: bool = True,
    ) -> "ProjectVersion":
        if parent is None:
            parent = self.head
            if self.head is None:
                raise ValueError(f"project does not have a head version: {self}")
        if not parent.is_committed:
            if auto_commit:
                parent.commit()
            else:
                raise ValueError(f"parent version must be committed: {parent}")

        version = ProjectVersion.objects.create(project=self, name=name, description=description)
        version.parents.add(parent)
        if parent:
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
                [version.id, parent.id],
            )

        # head has advanced to new version
        if parent == self.head:
            self.head = version

        return version

    objects = ProjectManager()

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


class ProjectFileType(models.TextChoices):
    """
    The type of "file" in a project (tasks, instructions, models, datasets).
    """

    TASK = "task", "Task"
    INSTRUCTION = "instruct", "Instruction"
    MODEL = "model", "Model"
    DATASET = "data", "Dataset"


class ProjectFile(UUIDModel):
    """
    A "file" edge defining a single named object (task, instruction, model, dataset) in a project.
    """

    project_version: models.ForeignKey = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    type: models.CharField = models.CharField(max_length=64, choices=ProjectFileType.choices)
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)

    task = models.ForeignKey("Task", on_delete=models.RESTRICT, null=True)
    instruction = models.ForeignKey("Instruction", on_delete=models.RESTRICT, null=True)
    # TODO @Architecture: having models as part of projects doesn't seem quite right
    model = models.ForeignKey("Model", on_delete=models.RESTRICT, null=True)
    dataset = models.ForeignKey("Dataset", on_delete=models.RESTRICT, null=True)

    def __str__(self):
        return f"{self.project_version}/{self.name}.{self.type}"

    @property
    def name_dot_type(self):
        return f"{self.name}.{self.type}"

    class Meta:
        constraints = [
            # ensure that the name is unique per type within the project version
            models.UniqueConstraint(
                name="bench_project_file_project_type_name_ak",
                fields=["project_version_id", "type", "name"],
            ),
            # ensure that the objects are unique per type within the project version
            models.UniqueConstraint(
                name="bench_project_file_task_id_ak",
                fields=["project_version_id", "task_id"],
            ),
            models.UniqueConstraint(
                name="bench_project_file_instruction_id_ak",
                fields=["project_version_id", "instruction_id"],
            ),
            models.UniqueConstraint(
                name="bench_project_file_model_id_ak",
                fields=["project_version_id", "model_id"],
            ),
            models.UniqueConstraint(
                name="bench_project_file_dataset_id_ak",
                fields=["project_version_id", "dataset_id"],
            ),
        ]
