from itertools import chain
from typing import AsyncGenerator, Optional
from uuid import UUID

import zmq
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import language
from bench.api.symbol import StatementType, SymbolType, TypeNode
from bench.language import wire
from bench.runtime.worker import ReqModuleRuntimePayload
from bench.settings import ZMQ_RUNTIME_WORKER_ADDR
from bench.zmq import ZMessage, ZMessageType, recv_message_with, send_message, zmq_ctx
from bench.zmq.messages import RepModuleRuntimePayload


@gql.type
class InterpModule:
    id: UUID
    name: str
    files: list["InterpFile"]


@gql.type
class InterpFile:
    id: UUID
    module: InterpModule
    path: str
    statements: list["InterpStatement"]


@gql.type
class InterpStatement:
    id: UUID
    file: InterpFile
    name: Optional[str]
    type: StatementType
    symbol_type: Optional[SymbolType]
    type_node: Optional[TypeNode]


@gql.type
class ModuleRuntime:
    module: InterpModule
    errors: list["Error"]


ErrorType = gql.enum(language.ErrorType)


@gql.type
class Error:
    type: ErrorType
    message: str
    statement: Optional[InterpStatement]


def rmap_module(wire_module: wire.ModuleData) -> InterpModule:
    """Maps a wire module into a GQL interpreted module"""
    interp_module = InterpModule(id=wire_module.id, name=wire_module.name, files=[])
    for file in wire_module.files:
        interp_file = InterpFile(id=file.id, module=interp_module, path=file.path, statements=[])
        interp_module.files.append(interp_file)
        for statement in file.statements:
            interp_statement = InterpStatement(
                id=statement.id,
                file=interp_file,
                name=statement.name,
                type=statement.type,
                symbol_type=statement.symbol_type,
                type_node=statement.type_node,
            )
            interp_file.statements.append(interp_statement)
    return interp_module


def rmap_errors(wire_errors: list[wire.ErrorData], module: InterpModule) -> list[Error]:
    """Maps a wire error into a GQL error"""
    statements_by_id = {}
    for statement in chain.from_iterable(file.statements for file in module.files):
        statements_by_id[statement.id] = statement

    errors = []
    for error in wire_errors:
        statement = statements_by_id[error.statement_id] if error.statement_id else None
        error = Error(type=ErrorType(error.type), message=error.message, statement=statement)
        errors.append(error)
    return errors


@gql.type
class ModuleRuntimeSubscription:
    @gql.subscription
    async def module_runtime_changed(
        self, project_version_id: GlobalID
    ) -> AsyncGenerator[ModuleRuntime, None]:
        project_version_id = UUID(project_version_id.node_id)
        worker_req_sock = zmq_ctx.socket(zmq.REQ)
        worker_req_sock.connect(ZMQ_RUNTIME_WORKER_ADDR)
        worker_sub_sock = zmq_ctx.socket(zmq.SUB)
        worker_sub_sock.connect(ZMQ_RUNTIME_WORKER_ADDR)

        # get initial runtime
        req_module_runtime_msg = ZMessage(
            ZMessageType.REQ_MODULE_RUNTIME, ReqModuleRuntimePayload(module_id=project_version_id)
        )
        send_message(worker_req_sock, req_module_runtime_msg)
        _, payload = await recv_message_with(worker_req_sock, RepModuleRuntimePayload)
        module = rmap_module(payload.module)
        yield ModuleRuntime(module=module, errors=rmap_errors(payload.errors, module))

        # get runtime changes
        try:
            while True:
                _, update = await recv_message_with(worker_sub_sock, RepModuleRuntimePayload)
                module = rmap_module(update.module)
                yield ModuleRuntime(module=module, errors=rmap_errors(update.errors, module))

        finally:
            worker_req_sock.close()
            worker_sub_sock.close()


@gql.type
class ModuleExecutionSubscription:
    @gql.subscription
    async def model_execution_changed(
        self, project_version_id: GlobalID
    ) -> AsyncGenerator[None, None]:
        raise NotImplementedError


@gql.type
class ProjectSubscription:
    @gql.subscription
    async def project_changed(self, project_id: GlobalID) -> AsyncGenerator[None, None]:
        raise NotImplementedError
