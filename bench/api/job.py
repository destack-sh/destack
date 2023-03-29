from typing import TYPE_CHECKING, Annotated, Optional

from strawberry import auto, lazy
from strawberry_django_plus import gql

from bench import models

JobType = gql.enum(models.JobType)
JobStatus = gql.enum(models.JobStatus)


if TYPE_CHECKING:
    from bench.api.build import BuildCandidate
    from bench.api.deployment import Deployment
    from bench.api.root import Project, ProjectVersion


@gql.django.type(models.Job)
class Job(gql.Node):
    project: Annotated["Project", lazy(".project")]
    project_version: Annotated["ProjectVersion", lazy(".project")]
    deployment: Annotated["Deployment", lazy(".deployment")]
    parent: Optional["Job"]
    created_at: auto
    updated_at: auto
    started_at: auto
    terminated_at: auto
    type: JobType
    status: JobStatus
    # job-specific data
    build_candidate: Optional[Annotated["BuildCandidate", lazy(".build")]]


@gql.type
class JobQuery:
    pass


@gql.type
class JobSubscription:
    pass
