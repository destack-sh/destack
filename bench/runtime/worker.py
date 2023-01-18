from dataclasses import dataclass
from uuid import UUID

import structlog
import zmq
import zmq.asyncio

from bench.language import Module, Statement
from bench.zmq import ZMessage, ZMessageType, recv_message, recv_message_poll, send_message, zmq_ctx
from bench.zmq.messages import ReqModuleStatePayload

logger = structlog.get_logger(__name__)


@dataclass
class Error:
    type: str
    message: str
    statement: Statement


@dataclass
class ModuleState:
    id: UUID
    revision_hash: str
    name: str
    all_symbols: list[Statement]
    errors: list["Error"]


@dataclass
class ModuleWorkerState:
    module_id: UUID
    module_state: ModuleState
    module: Module


class RuntimeWorker:
    def __init__(self, worker_id: str | UUID):
        self.worker_id = worker_id
        self.rep_sock = zmq_ctx.socket(zmq.REP)
        self.int_req_sock = zmq_ctx.socket(zmq.REQ)
        self.change_sub_sock = zmq_ctx.socket(zmq.SUB)
        self.working_modules: dict[UUID, ModuleWorkerState] = {}

    async def get_init_worker_state(self, module_id: UUID) -> ModuleWorkerState:
        if module_id not in self.working_modules:
            req_read_module = ZMessage(ZMessageType.REQ_READ_MODULE)
            send_message(self.int_req_sock, req_read_module)
            rep_read_module = recv_message(self.int_req_sock)

        return self.working_modules[module_id]

    async def start(
        self, runtime_worker_addr: str, internal_server_addr: str, api_server_addr: str
    ):
        logger.info(
            "runtime_worker.start",
            worker_id=self.worker_id,
            runtime_worker_addr=runtime_worker_addr,
            internal_server_addr=internal_server_addr,
            api_server_addr=api_server_addr,
        )
        self.rep_sock.bind(runtime_worker_addr)
        self.int_req_sock.connect(internal_server_addr)
        self.change_sub_sock.connect(internal_server_addr)
        self.change_sub_sock.connect(api_server_addr)

        poller = zmq.asyncio.Poller()
        poller.register(self.rep_sock, zmq.POLLIN)
        poller.register(self.change_sub_sock, zmq.POLLIN)

        while True:
            msg = await recv_message_poll(poller)

            if msg.type == ZMessageType.REQ_MODULE_STATE:
                module_id = msg.payload_as(ReqModuleStatePayload).module_id
                state = await self.get_init_worker_state(module_id)

            # TODO @Incomplete: get initial module state from zmq server, subscribe to changes
            # TODO @Incomplete: trigger and tasks and send out updated module state
            # TODO @Incomplete: write back compilation results to zmq server

    async def stop(self):
        logger.info("runtime_worker.stop", worker_id=self.worker_id)
        self.rep_sock.close()
        self.int_req_sock.close()
        self.change_sub_sock.close()
