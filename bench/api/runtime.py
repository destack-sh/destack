from __future__ import annotations

from datetime import datetime
from itertools import chain
from typing import AsyncGenerator, Optional, cast
from uuid import UUID

import structlog
import zmq
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import language, models
from bench.api.auth import can_view_project, can_write_project
from bench.api.execution import Execution
from bench.api.statement import SimpleTypeNode, SimplyTyped, StatementType, SymbolType, TypeTag
from bench.language import wire
from bench.language.type import StatementModifier
from bench.models import ProjectVersion, User, mapper
from bench.msg import ZMessageType, recv_message_with, send_message, zmq_ctx
from bench.msg.messages import (
    ExecutionChangedPayload,
    ModuleRuntimeChangedPayload,
    RepModuleBuildPayload,
    RepModuleRunPayload,
    RepModuleRuntimePayload,
    ReqModuleBuildPayload,
    ReqModuleRunPayload,
    as_key,
)
from bench.runtime.worker import ReqModuleRuntimePayload
from bench.settings import ZMQ_WORKER_PUB_ADDR, ZMQ_WORKER_REP_ADDR

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
    symbol: Optional[InterpSymbol]


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


@gql.input
class RunInput:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID] = None
    build_id: Optional[GlobalID] = None
    arguments: JSON


@gql.type
class RunState:
    project_version_id: GlobalID
    runnable_id: Optional[GlobalID]
    build_id: Optional[GlobalID]
    output: Optional[JSON]
    success: bool


def check_can_write(user: User, project_version_id: UUID):
    project_version = (
        ProjectVersion.objects.all()
        .prefetch_related("project", "project__user", "project__organization")
        .get(id=project_version_id)
    )
    if not can_write_project(user, project_version.project):
        raise PermissionDenied("You don't have permission to write to this project.")


def check_can_view(user: User, project_version_id: UUID):
    project_version = (
        ProjectVersion.objects.all()
        .prefetch_related("project", "project__user", "project__organization")
        .get(id=project_version_id)
    )
    if not can_view_project(user, project_version.project):
        raise PermissionDenied("You don't have permission to view this project.")


@gql.type
class ModuleRuntimeMutation:
    @gql.mutation
    async def build(self, info: Info, input: BuildInput) -> BuildState | OperationInfo:
        # TODO @Cleanup @Performance: keep worker sockets across requests?
        worker_req_sock = zmq_ctx.socket(zmq.REQ)
        worker_req_sock.connect(ZMQ_WORKER_REP_ADDR)
        project_version_id = UUID(input.project_version_id.node_id)
        user = cast(User, info.context.request.scope["user"]._wrapped)
        await sync_to_async(check_can_write)(user, project_version_id)

        # :BlockingWorkerMessages
        send_message(
            worker_req_sock,
            ZMessageType.REQ_MODULE_BUILD,
            ReqModuleBuildPayload(
                module_id=project_version_id, buildable_id=input.buildable_id.node_id
            ),
        )
        _, rep = await recv_message_with(worker_req_sock, RepModuleBuildPayload)
        return BuildState(
            project_version_id=input.project_version_id,
            success=rep.error is None,
        )

    @gql.mutation
    async def run(self, info: Info, input: RunInput) -> RunState | OperationInfo:
        # TODO @Auth: check if user has write access to project
        worker_req_sock = zmq_ctx.socket(zmq.REQ)
        worker_req_sock.connect(ZMQ_WORKER_REP_ADDR)
        project_version_id = UUID(input.project_version_id.node_id)
        user = cast(User, info.context.request.scope["user"]._wrapped)
        await sync_to_async(check_can_write)(user, project_version_id)

        # :BlockingWorkerMessages
        send_message(
            worker_req_sock,
            ZMessageType.REQ_MODULE_RUN,
            ReqModuleRunPayload(
                module_id=project_version_id,
                runnable=UUID(input.runnable_id.node_id) if input.runnable_id else None,
                runnable_type=None,
                build=UUID(input.build_id.node_id) if input.build_id else None,
                arguments=input.arguments,
                blocking=True,
            ),
        )
        _, rep = await recv_message_with(worker_req_sock, RepModuleRunPayload)
        return RunState(
            project_version_id=input.project_version_id,
            runnable_id=input.runnable_id,
            build_id=input.build_id,
            success=rep.error is None,
            output=rep.output,
        )


@gql.type
class ModuleRuntimeSubscription:
    @gql.subscription
    async def module_runtime_changed(
        self, info: Info, project_version_id: GlobalID
    ) -> AsyncGenerator[ModuleRuntime, None]:
        project_version_id = UUID(project_version_id.node_id)
        user = cast(User, info.context.request.scope["user"]._wrapped)
        await sync_to_async(check_can_view)(user, project_version_id)

        log = logger.bind(
            project_version_id=project_version_id,
            worker_rep_addr=ZMQ_WORKER_REP_ADDR,
            worker_pub_addr=ZMQ_WORKER_PUB_ADDR,
        )
        log.info("runtime.subscribe")
        worker_req_sock = zmq_ctx.socket(zmq.REQ)
        worker_req_sock.connect(ZMQ_WORKER_REP_ADDR)
        worker_sub_sock = zmq_ctx.socket(zmq.SUB)
        worker_sub_sock.connect(ZMQ_WORKER_PUB_ADDR)
        worker_sub_sock.setsockopt(
            zmq.SUBSCRIBE, as_key(ZMessageType.MODULE_RUNTIME_CHANGED, str(project_version_id))
        )

        # get initial runtime
        send_message(
            worker_req_sock,
            ZMessageType.REQ_MODULE_RUNTIME,
            ReqModuleRuntimePayload(module_id=project_version_id),
        )
        _, payload = await recv_message_with(worker_req_sock, RepModuleRuntimePayload)
        module = rmap_module(payload.module)
        runtime = ModuleRuntime(
            updated_at=payload.updated_at,
            module=(module),
            dependencies=[rmap_module(dep) for dep in payload.dependencies],
            errors=(rmap_errors(payload.errors, module)),
            jobs=[rmap_job(job, module) for job in payload.jobs],
        )
        yield runtime

        # get runtime changes
        try:
            log.info("runtime.listen")
            while True:
                _, update = await recv_message_with(worker_sub_sock, ModuleRuntimeChangedPayload)
                log.debug("runtime.update", updated_at=update.updated_at)
                # module updates aren't really partial end-to-end yet (only complete fields for worker<->here)
                # :PartialModuleUpdates
                if update.module is not None:
                    runtime.module = rmap_module(update.module)
                if update.dependencies is not None:
                    runtime.dependencies = [rmap_module(dep) for dep in update.dependencies]
                if update.errors is not None:
                    runtime.errors = rmap_errors(update.errors, runtime.module)
                if update.jobs is not None:
                    runtime.jobs = [rmap_job(job, runtime.module) for job in update.jobs]
                runtime.updated_at = update.updated_at
                yield runtime
        finally:
            log.info("runtime.close")
            worker_req_sock.close()
            worker_sub_sock.close()

    @gql.subscription
    async def module_execution_changed(
        self,
        info: Info,
        project_version_id: GlobalID,
        build_id: Optional[GlobalID] = None,
        task_id: Optional[GlobalID] = None,
        code_id: Optional[GlobalID] = None,
        root_id: Optional[GlobalID] = None,
        root_id_null: bool = False,
    ) -> AsyncGenerator[Execution, None]:
        project_version_id = UUID(project_version_id.node_id)
        user = cast(User, info.context.request.scope["user"]._wrapped)
        await sync_to_async(check_can_view)(user, project_version_id)

        log = logger.bind(
            project_version_id=project_version_id,
            build_id=build_id,
            task_id=task_id,
            code_id=code_id,
            root_id=root_id,
            root_id_null=root_id_null,
            worker_rep_addr=ZMQ_WORKER_REP_ADDR,
        )
        log.info("executions.subscribe")

        worker_sub_sock = zmq_ctx.socket(zmq.SUB)
        worker_sub_sock.connect(ZMQ_WORKER_PUB_ADDR)
        worker_sub_sock.setsockopt(
            zmq.SUBSCRIBE, as_key(ZMessageType.EXECUTION_CHANGED, str(project_version_id))
        )

        try:
            log.info("executions.listen")
            while True:
                _, update = await recv_message_with(worker_sub_sock, ExecutionChangedPayload)
                update: ExecutionChangedPayload
                for frame_data in update.frames:
                    if (
                        build_id is not None
                        and build_id.node_id != frame_data.build_id
                        or task_id is not None
                        and task_id.node_id != frame_data.task_id
                        or code_id is not None
                        and code_id.node_id != frame_data.code_id
                        or root_id is not None
                        and root_id.node_id != frame_data.root_id
                        or root_id_null is True
                        and frame_data.root_id is not None
                    ):
                        # TODO @Performance: filter execution frames via zmq
                        continue
                    frame = mapper.rmap_execution_frame(frame_data)
                    # TODO @Cleanup @Performance: optimize all relation lookups for id only
                    # Here we just set the relation objects that we know are queried
                    # because strawberry isn't smart enough to optimize this (and avoid the lookup)
                    # Further, at this point the execution may not even be in the DB yet because
                    # we stream execution frames to DB and clients simultaneously, so the lookup can fail.
                    frame.parent = models.Execution(id=frame.parent_id)
                    frame.root = models.Execution(id=frame.root_id)
                    frame.code = models.Statement(id=frame.code_id)
                    frame.task = models.Statement(id=frame.task_id)
                    frame.build = models.Statement(id=frame.build_id)
                    log.debug("executions.update", frame=frame)
                    yield frame
        finally:
            log.info("executions.close")
            worker_sub_sock.close()
