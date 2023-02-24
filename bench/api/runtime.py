from datetime import datetime
from itertools import chain
from typing import AsyncGenerator, Optional
from uuid import UUID

import structlog
import zmq
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import language, models
from bench.api.execution import Execution
from bench.api.statement import SimpleTypeNode, SimplyTyped, StatementType, SymbolType, TypeTag
from bench.language import wire
from bench.language.type import StatementModifier
from bench.models import mapper
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


def rmap_errors(wire_errors: list[wire.ErrorData], module: InterpModule) -> list[InterpError]:
    """Maps a wire error into a GQL error"""
    symbols_by_id = {}
    for symbol in chain.from_iterable(file.symbols for file in module.files):
        symbols_by_id[UUID(symbol.id.node_id)] = symbol

    errors = []
    for error in wire_errors:
        symbol = symbols_by_id[error.statement_id] if error.statement_id else None
        error = InterpError(type=InterpErrorType(error.type), message=error.message, symbol=symbol)
        errors.append(error)
    return errors


@gql.input
class BuildInput:
    project_version_id: GlobalID
    buildable_id: Optional[GlobalID] = None


@gql.type
class BuildState:
    project_version_id: GlobalID
    build_ids: list[GlobalID]
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


@gql.type
class ModuleRuntimeMutation:
    @gql.mutation
    async def build(self, input: BuildInput) -> BuildState | OperationInfo:
        # TODO @Cleanup @Performance: keep worker sockets across requests
        worker_req_sock = zmq_ctx.socket(zmq.REQ)
        worker_req_sock.connect(ZMQ_WORKER_REP_ADDR)
        project_version_id = UUID(input.project_version_id.node_id)
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
            build_ids=rep.build_ids,
            success=rep.error is None,
        )

    @gql.mutation
    async def run(self, input: RunInput) -> RunState | OperationInfo:
        worker_req_sock = zmq_ctx.socket(zmq.REQ)
        worker_req_sock.connect(ZMQ_WORKER_REP_ADDR)
        project_version_id = UUID(input.project_version_id.node_id)
        send_message(
            worker_req_sock,
            ZMessageType.REQ_MODULE_RUN,
            ReqModuleRunPayload(
                module_id=project_version_id,
                runnable_id=UUID(input.runnable_id.node_id) if input.runnable_id else None,
                build_id=UUID(input.build_id.node_id) if input.build_id else None,
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
        log = logger.bind(project_version_id=project_version_id)
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
        dependencies = [rmap_module(dep) for dep in payload.dependencies]
        errors = rmap_errors(payload.errors, module)
        yield ModuleRuntime(
            updated_at=payload.updated_at, module=module, dependencies=dependencies, errors=errors
        )

        # get runtime changes
        try:
            log.info("runtime.listen")
            while True:
                _, update = await recv_message_with(worker_sub_sock, ModuleRuntimeChangedPayload)
                log.debug("runtime.update", updated_at=update.updated_at)
                # :PartialModuleUpdates
                module = rmap_module(update.module)
                dependencies = [rmap_module(dep) for dep in update.dependencies]
                errors = rmap_errors(update.errors, module)
                yield ModuleRuntime(
                    updated_at=payload.updated_at,
                    module=module,
                    dependencies=dependencies,
                    errors=errors,
                )
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

        log = logger.bind(
            project_version_id=project_version_id,
            build_id=build_id,
            task_id=task_id,
            code_id=code_id,
            root_id=root_id,
            root_id_null=root_id_null,
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
                    # Here we just set the parent/root objects that we know are queried
                    # because strawberry isn't smart enough to optimize this (and avoid the lookup)
                    # Further, at this point the execution may not even be in the DB yet because
                    # we stream execution frames to DB and clients simultaneously, so the lookup can fail.
                    frame.parent = models.Execution(id=frame.parent_id)
                    frame.root = models.Execution(id=frame.root_id)
                    log.debug("executions.update", frame=frame)
                    yield frame
        finally:
            log.info("executions.close")
            worker_sub_sock.close()
