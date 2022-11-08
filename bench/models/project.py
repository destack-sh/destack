from django.core.validators import validate_slug
from django.db import models

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel
from bench.models.versioning import VersionedCommit, VersionedRepository


class Project(VersionedRepository, TaggableMixin, UUIDModel):
    """
    A project to instruct an AI to do something.

    A project has a main program (the top-level task & flow implementations).
    Later, projects may also be "non-executable" libraries.

    Projects are the highest-level unit of versioning, similar to repositories in Git.
    All versions are available in 'versions'.
    """

    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    slug: models.SlugField = models.SlugField(
        max_length=128, unique=True, validators=[validate_slug]
    )
    description: models.CharField = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="projects"
    )
    # we'll likely have multiple programs per project at some point
    program: models.ForeignKey = models.ForeignKey(
        "Flow", on_delete=models.CASCADE, related_name="projects"
    )
    # flows via Flow.project
    # models via Model.project
    # datasets via Dataset.project


class ProjectVersion(VersionedCommit, TaggableMixin, UUIDModel):
    """
    A project version records the state of a project at a specific point in time.
    """

    project = models.ForeignKey(Project, on_delete=models.CASCADE, related_name="versions")
    version = models.CharField(max_length=256, null=True)
    parents = models.ManyToManyField("ProjectVersion", symmetrical=False)
    # dataset_versions via DatasetVersion.project_version
    # don't have model_versions yet because currently provided models are not versioned

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.name_version}"

    @property
    def organization(self):
        return self.project.organization

    @property
    def name_version(self) -> str:
        return f"{self.project.name}@{self.version}"

    class Meta:
        indexes = [
            models.Index(name="bench_project_version_idx", fields=["version"]),
        ]
        constraints = [
            models.UniqueConstraint(
                name="bench_project_version_project_version_ak",
                fields=["project", "version"],
            )
        ]
