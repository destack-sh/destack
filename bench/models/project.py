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


class ProjectManager(models.Manager):
    @transaction.atomic
    def create(self, organization: Organization, name: str, slug: Optional[str]) -> "Project":
        if not slug:
            slug = slugify(name)
        project = super().create(organization=organization, name=name, slug=slug)
        project.head = ProjectVersion.objects.create(project=project)
        project.save()
        return project


class Project(TaggableMixin, UUIDModel):
    """
    A project to instruct an AI to do something.

    Projects are the root of versioning, similar to repositories in Git.
    All versions are available in 'versions' and may not be linear (also like in Git).
    Project "files" (the contents of the project) are copy-on-write.

    A project has a main program (the top-level task & instruction implementations).
    Later, projects could also be "non-executable" libraries.
    """

    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    slug: models.SlugField = models.SlugField(
        max_length=128, unique=True, validators=[validate_slug]
    )
    description: models.CharField = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    # TODO @Cleanup: head and branch heads should probably move (to tags?)
    head = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, null=True, related_name="project+"
    )

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
 (project_version_id, type, name, task_id, instruction_id, model_id, dataset_id)
SELECT %s, type, name, task_id, instruction_id, model_id, dataset_id
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


class ProjectFileType(models.TextChoices):
    """
    The type of "file" in a project, used to distinguish between tasks, instructions, models, etc.
    """

    TASK = "task", "Task"
    INSTRUCTION = "instruction", "Instruction"
    MODEL = "model", "Model"
    DATASET = "dataset", "Dataset"


class ProjectFile(UUIDModel):
    """
    A "file" containing a single named object (task, instruction, model, dataset) in a project.

    Conceptually, each task type has its own extension.
    """

    project_version: models.ForeignKey = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    type: models.CharField = models.CharField(max_length=64, choices=ProjectFileType.choices)
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)

    task = models.ForeignKey("Task", on_delete=models.CASCADE, null=True)
    instruction = models.ForeignKey("Instruction", on_delete=models.CASCADE, null=True)
    model = models.ForeignKey("Model", on_delete=models.CASCADE, null=True)
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE, null=True)

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


class ProjectVersion(TaggableMixin, UUIDModel):
    """
    A project version records the state of a project at a specific point in time.
    """

    project = models.ForeignKey(Project, on_delete=models.CASCADE, related_name="versions")
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    parents = models.ManyToManyField("ProjectVersion", symmetrical=False)
    committed_at = models.DateTimeField(null=True)

    # files via ProjectFile
    tasks = models.ManyToManyField("Task", through=ProjectFile)
    instructions = models.ManyToManyField("Instruction", through=ProjectFile)
    datasets = models.ManyToManyField("Dataset", through=ProjectFile)
    # we'll likely have multiple programs per project at some point
    program: models.ForeignKey = models.ForeignKey(
        "Instruction", on_delete=models.CASCADE, null=True, related_name="projects"
    )
    # set this last as not to override 'models' imported from django.db
    models = models.ManyToManyField("Model", through=ProjectFile)

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.name}@{self.id}"

    def reset(self):
        # TODO @Robustness: reset will fail if other versions are referencing some of the same files
        self.files.all().delete()

    def commit(self):
        self.committed_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        self.save()

    @property
    def is_committed(self):
        return self.committed_at is not None

    @property
    def organization(self):
        return self.project.organization
