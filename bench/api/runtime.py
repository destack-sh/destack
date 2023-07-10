from typing import Optional
from uuid import UUID

import posthog
import structlog
from asgiref.sync import sync_to_async
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import bench as language
from bench import models
from bench.api.auth import check_can_read_project, check_can_write_project
from bench.api.session import Run, RunTriggerType
from bench.api.utils import asafe_mutation, get_user_from_info, to_uuid
from bench.bench.core import SessionTracingLevel
from bench.models import packer
from bench.msg import messages
from bench.msg.core import NMessage, request
from bench.msg.messages import (
    NMessageType,
    RepCancelRunPayload,
    RepLangserverPayload,
    RepRunPayload,
    ReqCancelRunPayload,
    ReqLangserverPayload,
    ReqRunPayload,
)

logger = structlog.get_logger(__name__)

InterpErrorType = gql.enum(language.IssueType)


@gql.input
class LangserverWakeInput:
    project_version_id: GlobalID


@gql.type
class LangserverWakePayload:
    success: bool


@gql.input
class RunInput:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID] = None
    run_id: Optional[GlobalID] = None
    arguments: Optional[JSON] = None
    trace: int = SessionTracingLevel.ALL
    block: bool = True
    keyed: bool = False
    timeout_seconds: Optional[int] = None


ModuleRunErrorType = gql.enum(messages.RunErrorType)


@gql.type
class RunState:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID]
    success: bool
    run_id: Optional[GlobalID]
    run: Optional[Run]


@gql.input
class CancelRunInput:
    project_version_id: GlobalID
    run_id: GlobalID


@gql.type
class CancelRunPayload:
    success: bool
    run: Optional[Run]


@gql.type
class RuntimeMutation:
    @asafe_mutation
    async def langserver_wake(
        self, info: Info, input: LangserverWakeInput
    ) -> LangserverWakePayload | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        await sync_to_async(check_can_read_project)(info, project_version)
        await request(
            NMessageType.REQUEST_LANGSERVER,
            ReqLangserverPayload(module_id=project_version_id),
            reply_t=RepLangserverPayload,
        )
        return LangserverWakePayload(success=True)

    @asafe_mutation
    async def run(self, info: Info, input: RunInput) -> RunState | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        user = get_user_from_info(info)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        # TODO @Auth: should run be a guest-level permission for projects?
        await sync_to_async(check_can_write_project)(info, project_version)

        run = ReqRunPayload(
            module_id=project_version_id,
            runnable=to_uuid(input.runnable_id),
            runnable_type=None,
            arguments=input.arguments,
            block=input.block,
            tracing_level=input.trace,
            trigger_type=RunTriggerType.UI,
            trigger_id=user.id,
            run_id=to_uuid(input.run_id),
            keyed=input.keyed,
        )
        try:
            rep: NMessage[RepRunPayload] = await request(
                NMessageType.REQUEST_RUN,
                run,
                reply_t=RepRunPayload,
                timeout=input.timeout_seconds,
            )
            success = rep.p.error is None
            error = rep.p.error
        except TimeoutError:
            success = False
            error = ModuleRunErrorType.TIMEOUT
        posthog.capture(
            str(user.id),
            "run",
            {"project_version_id": str(project_version_id), "success": success, "error": error},
        )
        run = packer.unpack_data(rep.p.run) if rep.p.run else None
        return RunState(
            project_version_id=input.project_version_id,
            runnable_id=input.runnable_id,
            success=success,
            run=run,
            run_id=rep.p.run_id,
        )

    @asafe_mutation
    async def cancel_run(
        self, info: Info, input: CancelRunInput
    ) -> CancelRunPayload | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        user = get_user_from_info(info)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        await sync_to_async(check_can_write_project)(info, project_version)
        cancel = ReqCancelRunPayload(
            module_id=project_version_id,
            run_id=to_uuid(input.run_id),
        )
        try:
            rep: NMessage[RepCancelRunPayload] = await request(
                NMessageType.REQUEST_CANCEL_RUN,
                cancel,
                reply_t=RepCancelRunPayload,
            )
            success = rep.p.success
        except TimeoutError:
            success = False
        posthog.capture(
            str(user.id),
            "cancel_run",
            {"project_version_id": str(project_version_id), "success": success},
        )
        return CancelRunPayload(success=success, run=None)
