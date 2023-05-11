from typing import TYPE_CHECKING, Annotated, AsyncGenerator, Iterable, Optional, cast
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from rest_framework.exceptions import PermissionDenied
from strawberry import auto, lazy
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.auth import CanViewProject, check_can_view_project_by_id
from bench.api.util import asafe_subscription
from bench.models import mapper
from bench.msg import NMessageType
from bench.msg.core import NMessage, subscribe
from bench.msg.messages import JobSavedPayload

logger = structlog.get_logger(__name__)

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
    @gql.django.connection(directives=[CanViewProject()])
    def jobs(
        self,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID],
        status_in: Optional[list[JobStatus]] = None,
        type_in: Optional[list[JobType]] = None,
    ) -> Iterable[Job]:
        qs = models.Job.objects.filter(project_id=project_id.node_id)
        if project_version_id:
            qs = qs.filter(project_version_id=project_version_id.node_id)
        if status_in:
            qs = qs.filter(status__in=status_in)
        if type_in:
            qs = qs.filter(type__in=type_in)
        return qs


@gql.type
class JobSubscription:
    @asafe_subscription
    async def jobs_changed(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: GlobalID,
        type_in: Optional[list[JobType]] = None,
    ) -> AsyncGenerator[Job, None]:
        project_id = UUID(project_id.node_id)
        project_version_id = UUID(project_version_id.node_id)
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        log = logger.bind(
            project_id=project_id,
            project_version_id=project_version_id,
            user=user,
        )
        try:
            await sync_to_async(check_can_view_project_by_id)(
                user, project_id=project_id, project_version_id=project_version_id
            )
        except PermissionDenied:
            log.debug("jobs.subscribe_denied")
            return

        log.info("jobs.subscribe")
        jobs_sub = await subscribe(
            f"{NMessageType.JOB_SAVED}.{project_version_id}", payload_t=JobSavedPayload
        )

        log.info("jobs.listen")
        while True:
            msg: NMessage[JobSavedPayload] = await jobs_sub.next_msg()
            for job in msg.p.jobs:
                if type_in and job.type not in type_in:
                    continue
                job = mapper.rmap_job(job)
                log.debug("jobs.update", job=job)
                yield job
