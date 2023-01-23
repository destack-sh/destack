from itertools import chain
from typing import AsyncGenerator, Optional
from uuid import UUID

import zmq
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

import bench
from bench import language
from bench.api.statement import StatementType, SymbolType
from bench.language import wire
from bench.language.type import StatementModifier
from bench.runtime.worker import ReqModuleRuntimePayload
from bench.settings import ZMQ_RUNTIME_WORKER_REP_ADDR
from bench.zmq import ZMessageType, recv_message_with, send_message, zmq_ctx
from bench.zmq.messages import RepModuleRuntimePayload

TypeTag = gql.enum(bench.language.type.TypeTag)


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
    id: UUID
    source_id: GlobalID
    name: str
    files: list["InterpFile"]


@gql.type
class InterpFile:
    id: UUID
    source_id: GlobalID
    module: InterpModule
    path: str
    statements: list["InterpStatement"]


@gql.type
class InterpStatement:
    id: UUID
    source_id: GlobalID
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
    statement: Optional[InterpStatement]


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
        id=wire_module.id,
        source_id=GlobalID("ProjectVersion", str(wire_module.id)),
        name=wire_module.name,
        files=[],
    )
    for file in wire_module.files:
        interp_file = InterpFile(
            id=file.id,
            source_id=GlobalID("File", str(file.id)),
            module=interp_module,
            path=file.path,
            statements=[],
        )
        interp_module.files.append(interp_file)
        for statement in file.statements:
            # only include type node if it's been parsed
            type_node = (
                statement.type_node if isinstance(statement.type_node, wire.TypeNode) else None
            )
            interp_statement = InterpStatement(
                id=statement.id,
                source_id=GlobalID("Statement", str(statement.id)),
                file=interp_file,
                name=statement.name,
                modifier=statement.modifier,
                type=statement.type,
                symbol_type=statement.symbol_type,
                type_node=type_node,  # no need to map since lang and api types match
            )
            interp_file.statements.append(interp_statement)
    return interp_module


def rmap_errors(wire_errors: list[wire.ErrorData], module: InterpModule) -> list[InterpError]:
    """Maps a wire error into a GQL error"""
    statements_by_id = {}
    for statement in chain.from_iterable(file.statements for file in module.files):
        statements_by_id[statement.id] = statement

    errors = []
    for error in wire_errors:
        statement = statements_by_id[error.statement_id] if error.statement_id else None
        error = InterpError(
            type=InterpErrorType(error.type), message=error.message, statement=statement
        )
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
        worker_req_sock.connect(ZMQ_RUNTIME_WORKER_REP_ADDR)
        worker_sub_sock = zmq_ctx.socket(zmq.SUB)
        worker_sub_sock.connect(ZMQ_RUNTIME_WORKER_REP_ADDR)

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
            while True:
                _, update = await recv_message_with(worker_sub_sock, RepModuleRuntimePayload)
                # (this should definitely be partial updates)
                module = rmap_module(update.module)
                dependencies = [rmap_module(dep) for dep in update.dependencies]
                errors = rmap_errors(update.errors, module)
                yield ModuleRuntime(module=module, dependencies=dependencies, errors=errors)
        finally:
            worker_req_sock.close()
            worker_sub_sock.close()

    @gql.subscription
    async def model_execution_changed(
        self, project_version_id: GlobalID
    ) -> AsyncGenerator[None, None]:
        raise NotImplementedError
