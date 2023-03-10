from datetime import datetime
from itertools import chain
from typing import AsyncGenerator, Optional, cast
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied, ValidationError
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import language, models
from bench.api.auth import can_view_project, can_write_project
from bench.api.execution import Execution, ExecutionTriggerType, expand_project_version_ids
from bench.api.statement import SimpleTypeNode, SimplyTyped, StatementType, SymbolType, TypeTag
from bench.api.util import asafe_mutation, asafe_subscription, to_uuid, to_uuids
from bench.language import wire
from bench.language.type import StatementModifier
from bench.models import Project, ProjectVersion, User, mapper
from bench.msg import NMessageType, messages
from bench.msg.core import request, subscribe
from bench.msg.messages import (
    ExecutionChangedPayload,
    ModuleRuntimeChangedPayload,
    RepModuleBuildPayload,
    RepModuleRunPayload,
    RepModuleRuntimePayload,
    ReqModuleBuildPayload,
    ReqModuleRunPayload,
    ReqModuleRuntimePayload,
)

logger = structlog.get_logger(__name__)


@gql.type
class InterpModule:
    id: GlobalID
    name: str
    files: list["InterpFile"]


@gql.type
class InterpFile:
    id: GlobalID
    module: InterpModule
    path: str
    symbols: list["InterpSymbol"]


JobType = gql.enum(wire.JobType)
JobStatus = gql.enum(wire.JobStatus)


@gql.type
class InterpJob:
    id: GlobalID
    type: JobType
    status: JobStatus
    started_at: Optional[datetime]
    terminated_at: Optional[datetime]
    symbol: Optional["InterpSymbol"]


@gql.type
class InterpSimpleType(SimpleTypeNode):
    """
    Proxy type to SimpleType to avoid overwriting source SimpleType references
    (no extra fields yet but needed since (SimpleType, id) global id would be the same
     for the simple types output by the runtime and by the source types put in).
    """

    pass


# not to be confused with language.InterpSymbol
# which is not what we get out of the runtime worker yet
@gql.type
class InterpSymbol(SimplyTyped):
    id: GlobalID
    file: InterpFile
    order_key: str
    parent_id: Optional[GlobalID]
    name: Optional[str]
    type: StatementType
    generated: bool
    modifier: Optional[StatementModifier]
    symbol_type: Optional[SymbolType]
    root_type_tag: Optional[TypeTag]
    type_nodes: Optional[list[InterpSimpleType]]


InterpErrorType = gql.enum(language.ErrorType)


@gql.type
class InterpError:
    type: InterpErrorType
    message: str
    symbol: Optional[InterpSymbol]


# TODO @Cleanup: distinguish project change, module static analysis, module jobs and module runtime
#  Right now it's all intermingled.
@gql.type
class ModuleRuntime:
    updated_at: datetime
    module: InterpModule
    jobs: list[InterpJob]
    dependencies: list[InterpModule]
    errors: list["InterpError"]
    stale_symbols: list[InterpSymbol]


def rmap_module(wire_module: wire.ModuleData) -> InterpModule:
    """Maps a wire module into a GQL interpreted module"""
    interp_module = InterpModule(
        id=GlobalID("ProjectVersion", str(wire_module.id)),
        name=wire_module.name,
        files=[],
    )
    for file in wire_module.files:
        interp_file = InterpFile(
            id=GlobalID("File", str(file.id)),
            module=interp_module,
            path=file.path,
            symbols=[],
        )
        interp_module.files.append(interp_file)
        for statement in file.statements:
            if statement.type in (StatementType.COMMENT, StatementType.BLANK):
                continue  # ignore non-symbol statements
            root_type_tag, type_nodes = mapper.wmap_type_nodes(
                None, statement.type_nodes, impute_type_reference=True
            )
            interp_symbol = InterpSymbol(
                id=GlobalID("Statement", str(statement.id)),
                file=interp_file,
                order_key=statement.order_key,
                parent_id=statement.parent_id,
                name=statement.name,
                type=statement.type,
                generated=statement.generated,
                modifier=statement.modifier,
                symbol_type=statement.symbol_type,
                root_type_tag=root_type_tag,
                type_nodes=type_nodes,
            )
            interp_file.symbols.append(interp_symbol)
    return interp_module


def _get_symbol_from_module(module: InterpModule, symbol_id: UUID) -> Optional[InterpSymbol]:
    symbol_id_str = str(symbol_id)
    for symbol in chain.from_iterable(file.symbols for file in module.files):
        if symbol.id.node_id == symbol_id_str:
            return symbol
    return None


def rmap_errors(wire_errors: list[wire.ErrorData], module: InterpModule) -> list[InterpError]:
    """Maps a wire error into a GQL error"""
    errors = []
    for error in wire_errors:
        symbol = _get_symbol_from_module(module, error.statement_id)
        error = InterpError(type=InterpErrorType(error.type), message=error.message, symbol=symbol)
        errors.append(error)
    return errors


def rmap_job(wire_job: wire.JobData, module: InterpModule) -> InterpJob:
    """Maps a wire job into a GQL job"""
    symbol = _get_symbol_from_module(module, wire_job.statement_id)
    return InterpJob(
        id=GlobalID("Job", str(wire_job.id)),
        type=wire_job.type,
        status=wire_job.status,
        started_at=wire_job.started_at,
        terminated_at=wire_job.terminated_at,
        symbol=symbol,
    )


@gql.input
class BuildInput:
    project_version_id: GlobalID
    buildable_id: Optional[GlobalID] = None


@gql.type
class BuildState:
    project_version_id: GlobalID
    success: bool


ExecutionTracingLevel = gql.enum(wire.ExecutionTracingLevel)


@gql.input
class RunInput:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID] = None
    build_id: Optional[GlobalID] = None
    arguments: JSON
    tracing: ExecutionTracingLevel = ExecutionTracingLevel.ALL_FRAMES_WITH_DATA


ModuleRunErrorType = gql.enum(messages.ModuleRunErrorType)


@gql.type
class RunState:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID]
    build_id: Optional[GlobalID]
    output: Optional[JSON]
    success: bool
    error: Optional[ModuleRunErrorType]
    error_details: Optional[JSON]


def check_can_write_project(user: User, project_version_id: UUID):
    project_version = (
        ProjectVersion.objects.all()
        .prefetch_related("project", "project__user", "project__organization")
        .get(id=project_version_id)
    )
    if not can_write_project(user, project_version.project):
        raise PermissionDenied("You don't have permission to write to this project.")


def check_can_view_project(user: User, project_version_id: UUID = None, project_id: UUID = None):
    if not project_version_id and not project_id:
        raise ValueError("must set project_version_id or project_id")

    if project_version_id is None:
        project = Project.objects.prefetch_related("user", "organization").get(id=project_id)
    else:
        project_version = (
            ProjectVersion.objects.all()
            .prefetch_related("project", "project__user", "project__organization")
            .get(id=project_version_id)
        )
        project = project_version.project
        if project_id is not None and project_id != project.id:
            raise ValueError("project_id must match project_version_id")
    if not can_view_project(user, project):
        raise PermissionDenied("You don't have permission to view this project.")


@gql.type
class ModuleRuntimeMutation:
    @asafe_mutation
    async def build(self, info: Info, input: BuildInput) -> BuildState | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        user = cast(User, info.context.request.scope["user"]._wrapped)
        await sync_to_async(check_can_write_project)(user, project_version_id)

        # :BlockingWorkerMessages
        req = ReqModuleBuildPayload(
            module_id=project_version_id, buildable_id=input.buildable_id.node_id
        )
        rep = await request(NMessageType.REQUEST_MODULE_BUILD, req, RepModuleBuildPayload)
        return BuildState(
            project_version_id=input.project_version_id,
            success=rep.error is None,
        )

    @asafe_mutation
    async def run(self, info: Info, input: RunInput) -> RunState | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        user = cast(User, info.context.request.scope["user"]._wrapped)
        # TODO @Auth: should run be a guest-level permission for projects?
        await sync_to_async(check_can_write_project)(user, project_version_id)
        # :SingleOwnedDeployment
        deployment_id = (
            await models.Deployment.objects.filter(
                owned=True, project_version_id=project_version_id
            )
            .values_list("id", flat=True)
            .afirst()
        )
        if deployment_id is None:
            raise ValueError("no available deployment found")

        run = ReqModuleRunPayload(
            module_id=project_version_id,
            runnable=UUID(input.runnable_id.node_id) if input.runnable_id else None,
            runnable_type=None,
            build=UUID(input.build_id.node_id) if input.build_id else None,
            arguments=input.arguments,
            blocking=True,
            tracing_level=input.tracing,
            deployment_id=deployment_id,
            trigger_type=ExecutionTriggerType.UI_INTERACTIVE,
            trigger_id=user.id,
        )
        rep = await request(NMessageType.REQUEST_MODULE_RUN, run, RepModuleRunPayload)
        return RunState(
            project_version_id=input.project_version_id,
            runnable_id=input.runnable_id,
            build_id=input.build_id,
            success=rep.error is None,
            output=rep.output,
            error=rep.error,
            error_details=rep.error_details,
        )


@gql.type
class ModuleRuntimeSubscription:
    @asafe_subscription
    async def module_runtime_changed(
        self, info: Info, project_version_id: GlobalID
    ) -> AsyncGenerator[ModuleRuntime, None]:
        project_version_id = UUID(project_version_id.node_id)
        user = cast(User, info.context.request.scope["user"]._wrapped)
        log = logger.bind(
            project_version_id=project_version_id,
            user=user,
        )
        try:
            await sync_to_async(check_can_view_project)(user, project_version_id=project_version_id)
        except PermissionDenied:
            log.debug("runtime.subscribe_denied", project_version_id=project_version_id)
            return

        log.info("runtime.subscribe")
        runtime_sub = await subscribe(
            f"{NMessageType.MODULE_RUNTIME_CHANGED}.{project_version_id}",
            ModuleRuntimeChangedPayload,
        )

        # get initial runtime
        rep = await request(
            NMessageType.REQUEST_MODULE_RUNTIME,
            ReqModuleRuntimePayload(module_id=project_version_id),
            RepModuleRuntimePayload,
        )
        payload = rep.payload
        module = rmap_module(rep.p.module)
        runtime = ModuleRuntime(
            updated_at=payload.updated_at,
            module=module,
            dependencies=[rmap_module(dep) for dep in payload.dependencies],
            errors=(rmap_errors(payload.errors, module)),
            jobs=[rmap_job(job, module) for job in payload.jobs],
            stale_symbols=[
                _get_symbol_from_module(module, symbol_id) for symbol_id in payload.stale_symbols
            ],
        )
        yield runtime

        # get runtime changes
        log.info("runtime.listen")
        while True:
            update = await runtime_sub.next_msg()
            payload = update.payload
            log.debug("runtime.update", updated_at=payload.updated_at)
            # module updates aren't really partial end-to-end yet (only complete fields for worker<->here)
            # :PartialModuleUpdates
            # also the mapping duplication is a bit ugly
            if payload.module is not None:
                runtime.module = rmap_module(payload.module)
            if payload.dependencies is not None:
                runtime.dependencies = [rmap_module(dep) for dep in payload.dependencies]
            if payload.errors is not None:
                runtime.errors = rmap_errors(payload.errors, runtime.module)
            if payload.jobs is not None:
                runtime.jobs = [rmap_job(job, runtime.module) for job in payload.jobs]
            if payload.stale_symbols is not None:
                runtime.stale_symbols = [
                    _get_symbol_from_module(runtime.module, symbol_id)
                    for symbol_id in payload.stale_symbols
                ]
            runtime.updated_at = payload.updated_at
            yield runtime

    @asafe_subscription
    async def module_execution_changed(
        self,
        info: Info,
        project_id: GlobalID,
        project_version_id: Optional[GlobalID],
        include_ancestor_versions: bool = False,
        build_ids: list[GlobalID] | None = None,
        task_ids: list[GlobalID] | None = None,
        code_ids: list[GlobalID] | None = None,
        root_id: Optional[GlobalID] = None,
        root_id_null: bool = False,
    ) -> AsyncGenerator[Execution, None]:
        project_id = UUID(project_id.node_id)
        project_version_id = UUID(project_version_id.node_id)
        user = cast(User, info.context.request.scope["user"]._wrapped)

        log = logger.bind(
            project_id=project_id,
            project_version_id=project_version_id,
            build_ids=build_ids,
            task_ids=task_ids,
            code_ids=code_ids,
            root_id=root_id,
            root_id_null=root_id_null,
            user=user,
        )

        try:
            await sync_to_async(check_can_view_project)(
                user, project_id=project_id, project_version_id=project_version_id
            )
        except PermissionDenied:
            log.debug("executions.subscribe_denied")
            return

        log.info("executions.subscribe")
        runtime_sub = await subscribe(
            f"{NMessageType.EXECUTION_CHANGED}.{project_version_id}", ExecutionChangedPayload
        )

        # :ExecutionsFilter
        project_version_id = to_uuid(project_version_id)
        build_ids = to_uuids(build_ids)
        task_ids = to_uuids(task_ids)
        code_ids = to_uuids(code_ids)
        # If filtering by a symbol and including multiple versions, expand into their mappings.
        # We do this once before listening for performance and simplicity, though this means that new versions
        # will not be automatically included in the execution subscription. We could periodically re-check,
        # but that's a bit more complicated and not really worth it for now.
        if include_ancestor_versions:
            if project_version_id is None:
                raise ValidationError(
                    "project_version_id must be specified if include_ancestor_versions"
                )
            project_version_ids, expanded_symbol_ids = await expand_project_version_ids(
                project_version_id, build_ids, task_ids, code_ids, ancestor_depth=8
            )
        else:
            expanded_symbol_ids = [*(build_ids or []), *(task_ids or []), *(code_ids or [])]

        log.info("executions.listen")
        while True:
            msg = await runtime_sub.next_msg()
            update = msg.payload
            for frame_data in update.frames:
                # :ExecutionsFilter
                other_build = build_ids and frame_data.build_id not in expanded_symbol_ids
                other_task = task_ids and frame_data.task_id not in expanded_symbol_ids
                other_code = code_ids and frame_data.code_id not in expanded_symbol_ids
                other_root = (
                    root_id is not None
                    and root_id.node_id != frame_data.root_id
                    or root_id_null is True
                    and frame_data.root_id is not None
                )
                if other_build or other_task or other_code or other_root:
                    # TODO @Performance: filter execution frames via zmq?
                    continue
                frame = mapper.rmap_execution_frame(frame_data)
                log.debug("executions.update", frame=frame)
                yield frame
