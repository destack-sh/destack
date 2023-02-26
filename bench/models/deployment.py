from __future__ import annotations

from typing import TYPE_CHECKING
from uuid import UUID

from django.db import models
from django_choices_field import TextChoicesField

from bench.models.statement import Statement
from bench.models.utils import UUIDModel

if TYPE_CHECKING:
    from bench.models.organization import Organization
    from bench.models.project import ProjectVersion
    from bench.models.user import User


class DeploymentStatus(models.TextChoices):
    INACTIVE = "INACTIVE"
    SLEEPING = "SLEEPING"
    ACTIVE = "ACTIVE"
    ARCHIVED = "ARCHIVED"
    DESTROYED = "DESTROYED"


class DeploymentType(models.TextChoices):
    ADHOC = "ADHOC"
    MANUAL = "MANUAL"


class DeploymentManager(models.Manager["Deployment"]):
    def create_deployment(
        self,
        project_version: ProjectVersion,
        owner: User | Organization,
        type: DeploymentType,
        status: DeploymentStatus = DeploymentStatus.INACTIVE,
    ) -> Deployment:
        return self.create(
            project=project_version.project,
            project_version=project_version,
            organization=owner if isinstance(owner, Organization) else None,
            user=owner if isinstance(owner, User) else None,
            type=type,
            status=status,
        )

    def copy(
        self,
        source_deployment: Deployment,
        target_version: "ProjectVersion",
        refs: dict[UUID, Statement],
    ) -> Deployment:
        target_deployment = Deployment.objects.get(id=source_deployment.id)
        target_deployment.id = None
        target_deployment.project_version = target_version
        target_deployment.save()
        # copy deployed statements
        deployed_statements = []
        for deployed_statement in source_deployment.deployed_statements.all():
            deployed_statement.id = None
            deployed_statement.deployment = target_deployment
            deployed_statement.statement = refs[deployed_statement.statement_id]
            deployed_statements.append(deployed_statement)
        models.DeployedStatement.objects.bulk_create(deployed_statements)
        return target_deployment


class Deployment(UUIDModel):
    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, null=True, related_name="deployments"
    )
    user: models.ForeignKey = models.ForeignKey(
        "User", on_delete=models.CASCADE, null=True, related_name="deployments"
    )
    project: models.ForeignKey = models.ForeignKey(
        "Project", on_delete=models.CASCADE, related_name="deployments"
    )
    project_version: models.ForeignKey = models.ForeignKey(
        "ProjectVersion",
        on_delete=models.CASCADE,
        related_name="deployments",
    )
    type = TextChoicesField(choices_enum=DeploymentType)
    status = TextChoicesField(choices_enum=DeploymentStatus)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deploy_all_statements = models.BooleanField(default=False)
    deployed_statements: models.ManyToManyField = models.ManyToManyField(
        "Statement", through="DeployedStatement"
    )

    def __str__(self):
        return f"{self.owner} {self.project_version} {self.status}"

    def __repr__(self):
        return f"<Deployment {self}>"

    @property
    def owner(self) -> Organization | User:
        return self.organization or self.user

    class Meta:
        ordering = ["-created_at"]
        constraints = [
            # must have an owner (both may be set while moving)
            models.CheckConstraint(
                check=models.Q(organization__isnull=False) | models.Q(user__isnull=False),
                name="bench_deployment_has_owner",
            ),
            # owner can only deploy a project version once
            models.UniqueConstraint(
                fields=["organization", "project_version"],
                condition=models.Q(organization__isnull=False),
                name="bench_deployment_version_organization_ak",
            ),
            models.UniqueConstraint(
                fields=["user", "project_version"],
                condition=models.Q(user__isnull=False),
                name="bench_deployment_version_user_ak",
            ),
        ]


class DeployedStatement(UUIDModel):
    deployment: models.ForeignKey = models.ForeignKey(
        "Deployment", on_delete=models.CASCADE, related_name="deployed_statements+"
    )
    statement: models.ForeignKey = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="deployments"
    )
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    class Meta:
        constraints = [
            # a statement can only be deployed once
            models.UniqueConstraint(
                fields=["deployment", "statement"],
                name="bench_deployed_statement_ak",
            ),
        ]
