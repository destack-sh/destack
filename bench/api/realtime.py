from typing import AsyncGenerator
from uuid import UUID

import zmq
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench.runtime.worker import ModuleState
from bench.utils.zmq import ZMessage, ZMessageType, send_message, zmq_ctx


@gql.type
class ModuleStateSubscription:
    @gql.subscription
    async def module_state_changed(
        self, project_version_id: GlobalID
    ) -> AsyncGenerator[ModuleState, None]:
        project_version_id = UUID(project_version_id.node_id)
        worker_req_sock = zmq_ctx.socket(zmq.REQ)
        worker_sub_sock = zmq_ctx.socket(zmq.SUB)

        send_message(worker_req_sock, ZMessage(ZMessageType.REQ_MODULE_STATE))
        # TODO @Incomplete: get module state updates from the worker via zmq server/client then pub/sub
        try:
            for module_state in []:
                yield module_state
        finally:
            worker_req_sock.close()
            worker_sub_sock.close()
