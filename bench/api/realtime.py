from typing import AsyncGenerator, Optional
from uuid import UUID

import zmq
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench.api.symbol import Statement, TypeNode
from bench.runtime.worker import ReqModuleRuntimePayload
from bench.settings import ZMQ_RUNTIME_WORKER_ADDR
from bench.zmq import ZMessage, ZMessageType, recv_message_with, send_message, zmq_ctx
from bench.zmq.messages import RepModuleRuntimePayload


@gql.type
class Job:
    id: UUID
    name: str


@gql.type
class InterpStatement(Statement):
    type: Optional[TypeNode]


@gql.type
class ModuleRuntime:
    id: UUID
    jobs: list[Job]
    symbols: list[InterpStatement]
    errors: list["ModuleError"]


@gql.type
class ModuleError:
    type: str
    message: str
    statement: Statement


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

        req_module_runtime_msg = ZMessage(
            ZMessageType.REQ_MODULE_RUNTIME, ReqModuleRuntimePayload(module_id=project_version_id)
        )
        send_message(worker_req_sock, req_module_runtime_msg)
        reply, module_runtime = await recv_message_with(worker_req_sock, RepModuleRuntimePayload)
        yield ModuleRuntime(module_id=module_runtime.module_id, tasks=[], symbols=[], errors=[])

        # TODO @Incomplete: get module state updates from the worker via zmq server/client then pub/sub
        try:
            for module_runtime in []:
                yield module_runtime
        finally:
            worker_req_sock.close()
            worker_sub_sock.close()
