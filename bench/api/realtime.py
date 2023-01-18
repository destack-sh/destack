from typing import AsyncGenerator, Optional
from uuid import UUID

import zmq
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench.api.symbol import Statement, TypeNode
from bench.runtime.worker import ReqModuleStatePayload
from bench.settings import ZMQ_RUNTIME_WORKER_ADDR
from bench.zmq import ZMessage, ZMessageType, recv_message, send_message, zmq_ctx


@gql.type
class Task:
    id: UUID
    name: str


@gql.type
class InterpStatement(Statement):
    type: Optional[TypeNode]


@gql.type
class ModuleState:
    id: UUID
    tasks: list[Task]
    all_symbols: list[InterpStatement]
    errors: list["ModuleError"]


@gql.type
class ModuleError:
    type: str
    message: str
    statement: Statement


@gql.type
class ModuleStateSubscription:
    @gql.subscription
    async def module_state_changed(
        self, project_version_id: GlobalID
    ) -> AsyncGenerator[ModuleState, None]:
        project_version_id = UUID(project_version_id.node_id)
        worker_req_sock = zmq_ctx.socket(zmq.REQ)
        worker_req_sock.connect(ZMQ_RUNTIME_WORKER_ADDR)
        worker_sub_sock = zmq_ctx.socket(zmq.SUB)
        worker_sub_sock.connect(ZMQ_RUNTIME_WORKER_ADDR)

        req_module_state_msg = ZMessage(
            ZMessageType.REQ_MODULE_STATE, ReqModuleStatePayload(module_id=project_version_id)
        )
        send_message(worker_req_sock, req_module_state_msg)
        rep_module_state = recv_message(worker_req_sock)

        # TODO @Incomplete: get module state updates from the worker via zmq server/client then pub/sub
        try:
            for module_state in []:
                yield module_state
        finally:
            worker_req_sock.close()
            worker_sub_sock.close()
