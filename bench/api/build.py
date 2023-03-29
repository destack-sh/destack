from typing import Annotated

from strawberry import auto, lazy
from strawberry_django_plus import gql

from bench import models
from bench.api.project import File, Project, ProjectVersion
from bench.api.statement import Statement

BuildCandidateStatus = gql.enum(models.BuildCandidateStatus)


@gql.django.filter(models.BuildCandidate)
class BuildCandidateFilter:
    pass


@gql.django.type(models.BuildCandidate)
class BuildCandidate(gql.Node):
    created_at: auto
    updated_at: auto
    project: Annotated["Project", lazy(".project")]
    project_version: Annotated["ProjectVersion", lazy(".project")]
    file: Annotated["File", lazy(".project")]
    status: BuildCandidateStatus
    build: Annotated["Statement", lazy(".build")]


@gql.type
class BuildQuery:
    pass


@gql.type
class BuildSubscription:
    pass
