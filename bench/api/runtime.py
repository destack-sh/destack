from itertools import chain
from typing import AsyncGenerator, Optional
from uuid import UUID

import structlog
import zmq
from strawberry.scalars import JSON
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import language
from bench.api.statement import StatementType, SymbolType, TypeTag
from bench.language import wire
from bench.language.type import StatementModifier
from bench.language.wire import wmap_type_node
from bench.runtime.worker import ReqModuleRuntimePayload
from bench.settings import ZMQ_RUNTIME_WORKER_PUB_ADDR, ZMQ_RUNTIME_WORKER_REP_ADDR
from bench.zmq import ZMessageType, recv_message_with, send_message, zmq_ctx
from bench.zmq.messages import (
    ModuleRuntimeChangedPayload,
    RepModuleCompilePayload,
    RepModuleRuntimePayload,
    ReqModuleCompilePayload,
)

logger = structlog.get_logger(__name__)


@gql.type
class TypeNode:
    name: Optional[str]
    type: TypeTag
    required: bool = True
    description: Optional[str]
    reference: Optional[str] = None
    children: Optional[list["TypeNode"]] = None


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
class InterpSymbol:
    id: GlobalID
    file: InterpFile
    name: Optional[str]
    type: StatementType
    modifier: Optional[StatementModifier]
    symbol_type: Optional[SymbolType]
    type_node: Optional[TypeNode]


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
            # only include type node if it's been parsed
            interp_symbol = InterpSymbol(
                id=GlobalID("Statement", str(statement.id)),
                file=interp_file,
                name=statement.name,
                modifier=statement.modifier,
                type=statement.type,
                symbol_type=statement.symbol_type,
                type_node=wmap_type_node(statement.type_nodes) if statement.type_nodes else None,
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
class CompileInput:
    project_version_id: GlobalID
    compilation_id: GlobalID


@gql.type
class CompileState:
    project_version_id: GlobalID
    compilation_id: GlobalID
    success: bool


@gql.input
class RunInput:
    project_version_id: GlobalID
    runconfig_id: GlobalID
    arguments: JSON


@gql.type
class RunState:
    project_version_id: GlobalID
    runconfig_id: GlobalID
    success: bool
    output: JSON


@gql.type
class ModuleRuntimeMutation:
    @gql.mutation
    async def compile(self, input: CompileInput) -> CompileState | OperationInfo:
        project_version_id = UUID(input.project_version_id.node_id)
        worker_req_sock = zmq_ctx.socket(zmq.REQ)
        worker_req_sock.connect(ZMQ_RUNTIME_WORKER_REP_ADDR)
        send_message(
            worker_req_sock,
            ZMessageType.REQ_MODULE_COMPILE,
            ReqModuleCompilePayload(
                module_id=project_version_id, compilation_id=input.compilation_id.node_id
            ),
        )
        _, rep = await recv_message_with(worker_req_sock, RepModuleCompilePayload)
        return CompileState(
            project_version_id=input.project_version_id,
            compilation_id=input.compilation_id,
            success=rep.success,
        )

    @gql.mutation
    async def run(self, input: RunInput) -> RunState | OperationInfo:
        raise NotImplementedError


@gql.type
class ModuleRuntimeSubscription:
    @gql.subscription
    async def module_runtime_changed(
        self, project_version_id: GlobalID
    ) -> AsyncGenerator[ModuleRuntime, None]:
        project_version_id = UUID(project_version_id.node_id)
        logger.info("subscribe", project_version_id=project_version_id)
        worker_req_sock = zmq_ctx.socket(zmq.REQ)
        worker_req_sock.connect(ZMQ_RUNTIME_WORKER_REP_ADDR)
        worker_sub_sock = zmq_ctx.socket(zmq.SUB)
        worker_sub_sock.connect(ZMQ_RUNTIME_WORKER_PUB_ADDR)
        # TODO @Performance: filter subscription messages properly (in all sites)
        worker_sub_sock.setsockopt(zmq.SUBSCRIBE, b"")

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
        yield ModuleRuntime(module=module, dependencies=dependencies, errors=errors)

        # get runtime changes
        try:
            logger.info("listen", project_version_id=project_version_id)
            while True:
                _, update = await recv_message_with(worker_sub_sock, ModuleRuntimeChangedPayload)
                logger.debug("update", project_version_id=project_version_id)
                # TODO @Performance: this should definitely be partial updates
                #  :PartialModuleUpdates
                module = rmap_module(update.module)
                dependencies = [rmap_module(dep) for dep in update.dependencies]
                errors = rmap_errors(update.errors, module)
                yield ModuleRuntime(module=module, dependencies=dependencies, errors=errors)
        finally:
            logger.info("close", project_version_id=project_version_id)
            worker_req_sock.close()
            worker_sub_sock.close()

    @gql.subscription
    async def model_execution_changed(
        self, project_version_id: GlobalID
    ) -> AsyncGenerator[None, None]:
        raise NotImplementedError
