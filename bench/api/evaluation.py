from typing import Annotated, AsyncGenerator, Iterable, Optional, cast
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied
from django.db.models import Q
from strawberry import auto, lazy
from strawberry.scalars import JSON
from strawberry.types import Info
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
    build: Optional[Annotated["Statement", lazy(".statement")]]
    statement: Optional[Annotated["Statement", lazy(".statement")]]
    record: Optional[Annotated["DatasetRecord", lazy(".statement")]]
    type_node: Optional[Annotated["SimpleTypeNode", lazy(".statement")]]
    self_metrics: Optional[JSON]
    aggregated_metrics: JSON


async def _expand_filters(
    project_version_id: UUID,
    include_ancestor_versions: bool,
    build_id_in: list[UUID] | None,
    system_id_in: list[UUID] | None,
    ancestor_depth: int = 8,
):
    if include_ancestor_versions:
        # TODO @Performance: implement symbol version id expansion in SQL
        project_versions = await sync_to_async(models.ProjectVersion.objects.get_ancestors)(
            version_id=project_version_id, depth=ancestor_depth
        )
        project_version_ids = [pv.id for pv in project_versions]
        expanded_symbol_ids = await sync_to_async(models.RefMapping.objects.expand_target_ids)(
            [*(build_id_in or []), *(system_id_in or [])], ancestor_depth
        )
    else:
        project_version_ids = [project_version_id]
        expanded_symbol_ids = [*(build_id_in or []), *(system_id_in or [])]
    return expanded_symbol_ids, project_version_ids


@gql.type
class EvaluationQuery:
    @gql.connection(directives=[CanViewProject()])
    async def evaluations(
        self,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID],
        include_ancestor_versions: bool = False,
        kind_in: Optional[list[EvaluationKind]] = None,
        scope_in: Optional[list[EvaluationScope]] = None,
        build_id_in: Optional[list[GlobalID]] = None,
        system_id_in: Optional[list[GlobalID]] = None,
    ) -> Iterable[EvaluationResult]:
        qs = models.EvaluationResult.objects.all()
        project_id = UUID(project_id.node_id)
        project_version_id = UUID(project_version_id.node_id)
        build_id_in = to_uuids(build_id_in)
        system_id_in = to_uuids(system_id_in)
        expanded_symbol_ids, project_version_ids = await _expand_filters(
            project_version_id, include_ancestor_versions, build_id_in, system_id_in
        )
        # :EvaluationsFilter
        qs = qs.filter(project_id=project_id, project_version_id__in=project_version_ids)
        if kind_in:
            qs = qs.filter(kind__in=kind_in)
        if scope_in:
            qs = qs.filter(scope__in=scope_in)
        if build_id_in:
            qs = qs.filter(build_id__in=expanded_symbol_ids)
        if system_id_in:
            qs = qs.filter(
                Q(statement_id__in=expanded_symbol_ids)
                | Q(record_id__in=expanded_symbol_ids)
                | Q(type_node_id__in=expanded_symbol_ids)
            )
        return qs


@gql.type
class EvaluationSubscription:
    @asafe_subscription
    async def evaluations_changed(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID],
        include_ancestor_versions: bool = False,
        kind_in: Optional[list[EvaluationKind]] = None,
        scope_in: Optional[list[EvaluationScope]] = None,
        build_id_in: Optional[list[GlobalID]] = None,
        system_id_in: Optional[list[GlobalID]] = None,
    ) -> AsyncGenerator[EvaluationResult, None]:
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
        expanded_symbol_ids, project_version_ids = await _expand_filters(
            project_version_id, include_ancestor_versions, build_id_in, system_id_in
        )
        log.debug("evaluations.listen")
        while True:
            msg: NMessage[EvaluationSavedPayload] = await evaluations_sub.next_msg()
            for evaluation in msg.p.evaluations:
                # :EvaluationsFilter
                other_kind = kind_in and evaluation.kind not in kind_in
                other_scope = scope_in and evaluation.scope not in scope_in
                other_build_id = build_id_in and evaluation.build_id not in expanded_symbol_ids
                other_system_id = (
                    system_id_in
                    and evaluation.statement_id not in expanded_symbol_ids
                    and evaluation.type_node_id not in expanded_symbol_ids
                    and evaluation.record_id not in expanded_symbol_ids
                )
                if other_kind or other_scope or other_build_id or other_system_id:
                    continue

                evaluation = mapper.rmap_evaluation_result(evaluation)
                log.debug("evaluations.update", evaluation=evaluation)
                yield evaluation
