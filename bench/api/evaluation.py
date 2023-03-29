from typing import Annotated, AsyncGenerator, Iterable, Optional, cast
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied
from strawberry import auto, lazy
from strawberry.scalars import JSON
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.auth import CanViewProject, check_can_view_project_by_id
from bench.api.project import File, Project, ProjectVersion
from bench.api.statement import DatasetRecord, SimpleTypeNode, Statement
from bench.api.util import asafe_subscription, to_uuids
from bench.models import mapper
from bench.msg import NMessageType
from bench.msg.core import NMessage, subscribe
from bench.msg.messages import EvaluationSavedPayload

logger = structlog.get_logger(__name__)

EvaluationKind = gql.enum(models.EvaluationKind)
EvaluationScope = gql.enum(models.EvaluationScope)


@gql.django.type(models.EvaluationResult)
class EvaluationResult(gql.Node):
    created_at: auto
    updated_at: auto
    project: Annotated["Project", lazy(".project")]
    project_version: Annotated["ProjectVersion", lazy(".project")]
    file: Annotated["File", lazy(".project")]
    kind: EvaluationKind
    scope: EvaluationScope
    statement: Annotated["Statement", lazy(".statement")]
    record: Annotated["DatasetRecord", lazy(".statement")]
    type_node: Annotated["SimpleTypeNode", lazy(".statement")]
    self_metrics: Optional[JSON]
    aggregated_metrics: JSON


async def _expand_project_version_ids(
    project_version_id: UUID, system_ids: list[UUID] | None, ancestor_depth: Optional[int]
):
    # TODO @Performance: implement symbol version id expansion in SQL
    project_versions = await sync_to_async(models.ProjectVersion.objects.get_ancestors)(
        version_id=project_version_id, depth=ancestor_depth
    )
    project_version_ids = [pv.id for pv in project_versions]
    expanded_symbol_ids = await sync_to_async(models.RefMapping.objects.expand_target_ids)(
        system_ids or [], ancestor_depth
    )
    return project_version_ids, expanded_symbol_ids


@gql.type
class EvaluationQuery:
    @gql.connection(directives=[CanViewProject()])
    async def evaluations(
        self,
        info,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID],
        include_ancestor_versions: bool = False,
        kind: Optional[EvaluationKind] = None,
        scope: Optional[EvaluationScope] = None,
        system_id_in: Optional[list[GlobalID]] = None,
    ) -> Iterable[EvaluationResult]:
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
            log.debug("evaluations.subscribe_denied")
            return

        log.info("evaluations.subscribe")
        evaluations_sub = await subscribe(
            f"{NMessageType.EVALUATION_SAVED}.{project_version_id}",
            payload_t=EvaluationSavedPayload,
        )

        system_id_in = to_uuids(system_id_in)
        if include_ancestor_versions:
            project_version_ids, expanded_symbol_ids = await _expand_project_version_ids(
                project_version_id, system_id_in, ancestor_depth=8
            )
        else:
            project_version_ids = [project_version_id]
            expanded_symbol_ids = system_id_in
        log.debug("evaluations.listen")
        while True:
            msg: NMessage[EvaluationSavedPayload] = await evaluations_sub.next_msg()
            for evaluation in msg.p.evaluations:
                other_kind = kind is not None and evaluation.kind != kind
                other_scope = scope is not None and evaluation.scope != scope
                other_system_id = (
                    expanded_symbol_ids is not None
                    and evaluation.system_id not in expanded_symbol_ids
                )
                if other_kind or other_scope or other_system_id:
                    continue

                evaluation = mapper.rmap_evaluation_result(evaluation)
                log.debug("evaluations.update", evaluation=evaluation)
                yield evaluation


@gql.type
class EvaluationSubscription:
    @asafe_subscription
    async def evaluations_changed(self) -> AsyncGenerator[EvaluationResult, None]:
        raise NotImplementedError
