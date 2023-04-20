from typing import Annotated, AsyncGenerator, Iterable, Optional

import structlog
from strawberry import auto, lazy
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.auth import CanViewProject
from bench.api.project import File, Project, ProjectVersion
from bench.api.statement import Statement
from bench.api.util import asafe_subscription

logger = structlog.get_logger(__name__)

BuildCandidateStatus = gql.enum(models.BuildCandidateStatus)


@gql.django.type(models.BuildSettings)
class BuildSettings(gql.Node):
    statement: Optional[Annotated["Statement", lazy(".statement")]]
    reactive: bool


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
    @gql.connection(directives=[CanViewProject()])
    def build_candidates(
        self,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID],
        build_id: Optional[GlobalID] = None,
        status_in: Optional[list[BuildCandidateStatus]] = None,
    ) -> Iterable[BuildCandidate]:
        qs = models.BuildCandidate.objects.filter(project_id=project_id.node_id)
        if project_version_id:
            qs = qs.filter(project_version_id=project_version_id.node_id)
        if build_id:
            qs = qs.filter(build_id=build_id.node_id)
        if status_in:
            qs = qs.filter(status__in=status_in)
        return qs


@gql.type
class BuildSubscription:
    @asafe_subscription
    async def build_candidate_changed(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: GlobalID,
        build_id: Optional[GlobalID] = None,
    ) -> AsyncGenerator[BuildCandidate, None]:
        raise NotImplementedError
