from __future__ import annotations

from datetime import datetime
from typing import Optional

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

    def get_or_create(
        self, organization: Organization, name: str, slug: Optional[str]
    ) -> "Project":
        if not slug:
            slug = slugify(name)
        project, _ = super().get_or_create(
            organization=organization, slug=slug, defaults={"name": name}
        )
        return project


class Project(TaggableMixin, UUIDModel):
    """
    A project to instruct an AI to do something.

    Projects are the root of versioning, similar to repositories in Git.
    All versions are available in 'versions' and may not be linear (also like in Git).
    Project "files" (the contents of the project) are copy-on-write.

    A project has a main program (the top-level task & flow implementations).
    Later, projects may also be "non-executable" libraries.
    """

    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    slug: models.SlugField = models.SlugField(
        max_length=128, unique=True, validators=[validate_slug]
    )
    description: models.CharField = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    head = models.ForeignKey("ProjectVersion", on_delete=models.CASCADE, related_name="project+")

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="projects"
    )

    @transaction.atomic
    def create_version(
        self,
        name: str,
        description: str = None,
        parent: ProjectVersion = None,
        auto_commit: bool = True,
    ) -> "ProjectVersion":
        if parent is None:
            parent = self.head
        if not parent.is_committed:
            if auto_commit:
                parent.commit()
            else:
                raise ValueError(f"parent version must be committed: {parent}")

        version = ProjectVersion.objects.create(
            project=self, name=name, description=description, parents=[parent]
        )
        if parent:
            # copy all project files from parent in SQL (see ProjectFile model below)
            cursor = connection.cursor()
            cursor.execute(
                """
INSERT INTO bench_projectfile
 (%s, type, name, task_id, flow_id, model_id, dataset_id)
SELECT %s, type, name, task_id, flow_id, model_id, dataset_id
 FROM bench_projectfile
 WHERE project_version_id = %s
""",
                [version.id, parent.id],
            )

        # head has advanced to new version
        if parent == self.head:
            self.head = version

        return version

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_project_organization_slug_ak",
                fields=["organization", "slug"],
            )
        ]


class ProjectFileType(models.TextChoices):
    """
    The type of "file" in a project, used to distinguish between tasks, flows, models, etc.
    """

    TASK = "task", "Task"
    FLOW = "flow", "Flow"
    MODEL = "model", "Model"
    DATASET = "dataset", "Dataset"


class ProjectFile(UUIDModel):
    """
    A "file" containing a single named object (task, flow, model, dataset) in a project.

    Conceptually, each task type is its own directory.
    """

    project_version: models.ForeignKey = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    type: models.CharField = models.CharField(max_length=64, choices=ProjectFileType.choices)
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)

    task = models.ForeignKey("Task", on_delete=models.CASCADE, null=True)
    flow = models.ForeignKey("Flow", on_delete=models.CASCADE, null=True)
    model = models.ForeignKey("Model", on_delete=models.CASCADE, null=True)
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE, null=True)

    class Meta:
        # ensure that the name is unique per type within the project
        constraints = [
            models.UniqueConstraint(
                name="bench_project_file_project_type_name_ak",
                fields=["project_version_id", "type", "name"],
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
    flows = models.ManyToManyField("Flow", through=ProjectFile)
    datasets = models.ManyToManyField("Dataset", through=ProjectFile)
    # we'll likely have multiple programs per project at some point
    program: models.ForeignKey = models.ForeignKey(
        "Flow", on_delete=models.CASCADE, related_name="projects"
    )
    # set this last as not to override 'models' imported from django.db
    models = models.ManyToManyField("Model", through=ProjectFile)

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.name}@{self.id}"

    def commit(self):
        self.committed_at = datetime.utcnow()
        self.save()

    @property
    def is_committed(self):
        return self.committed_at is not None

    @property
    def organization(self):
        return self.project.organization

    class Meta:
        indexes = []
        constraints = []
