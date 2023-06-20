from typing import Optional, cast
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
from bench.api.auth import check_can_write_project
from bench.api.execution import Execution, ExecutionTriggerType
from bench.api.utils import asafe_mutation, to_uuid
from bench.bench.const import SessionTracingLevel
from bench.models import packer
from bench.msg import NMessageType, messages
from bench.msg.core import NMessage, request
from bench.msg.messages import (
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
    build_id: Optional[GlobalID] = None
    execution_id: Optional[GlobalID] = None
    arguments: Optional[JSON] = None
    trace: int = SessionTracingLevel.ALL
    block: bool = True
    timeout_seconds: Optional[int] = None


ModuleRunErrorType = gql.enum(messages.RunErrorType)


@gql.type
class RunState:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID]
    default_build_id: Optional[GlobalID]
    success: bool
    execution_id: Optional[GlobalID]
    execution: Optional[Execution]


@gql.input
class CancelRunInput:
    project_version_id: GlobalID
    execution_id: GlobalID


@gql.type
class CancelRunPayload:
    success: bool
    execution: Optional[Execution]


@gql.type
class RuntimeMutation:
    @asafe_mutation
    async def langserver_wake(
        self, info: Info, input: LangserverWakeInput
    ) -> LangserverWakePayload | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        await sync_to_async(check_can_write_project)(info, project_version)
        await request(
            NMessageType.REQUEST_LANGSERVER,
            ReqLangserverPayload(module_id=project_version_id),
            reply_t=RepLangserverPayload,
        )
        return LangserverWakePayload(success=True)

    @asafe_mutation
    async def run(self, info: Info, input: RunInput) -> RunState | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        # TODO @Auth: should run be a guest-level permission for projects?
        await sync_to_async(check_can_write_project)(info, project_version)

        run = ReqRunPayload(
            module_id=project_version_id,
            runnable=to_uuid(input.runnable_id),
            runnable_type=None,
            default_build_id=to_uuid(input.build_id),
            arguments=input.arguments,
            block=input.block,
            tracing_level=input.trace,
            trigger_type=ExecutionTriggerType.UI,
            trigger_id=user.id,
            execution_id=to_uuid(input.execution_id),
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
        execution = packer.unpack_data(rep.p.execution) if rep.p.execution else None
        return RunState(
            project_version_id=input.project_version_id,
            runnable_id=input.runnable_id,
            default_build_id=input.build_id,
            success=success,
            execution=execution,
            execution_id=rep.p.execution_id,
        )

    @asafe_mutation
    async def cancel_run(
        self, info: Info, input: CancelRunInput
    ) -> CancelRunPayload | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        await sync_to_async(check_can_write_project)(info, project_version)
        cancel = ReqCancelRunPayload(
            module_id=project_version_id,
            execution_id=to_uuid(input.execution_id),
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
        return CancelRunPayload(success=success, execution=None)
