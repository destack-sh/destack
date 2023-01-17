from dataclasses import dataclass
from datetime import datetime
from typing import Optional
from uuid import UUID

import structlog
import zmq
from strawberry_django_plus import gql

from bench.api.symbol import Statement, TypeNode
from bench.language import Module
from bench.utils.zmq import zmq_ctx

logger = structlog.get_logger(__name__)


@gql.type
class Task:
    id: UUID
    type: str
    name: str
    started_at: datetime


@gql.type
class InterpStatement(Statement):
    type: Optional[TypeNode]


@gql.type
class ModuleState:
    id: UUID
    revision_hash: str
    name: str
    tasks: list[Task]
    all_symbols: list[InterpStatement]
    errors: list["ModuleError"]


@gql.type
class ModuleError:
    type: str
    message: str
    statement: Statement


@dataclass
class ModuleWorkerState:
    module_id: UUID
    module_state: ModuleState
    module: Module


class RuntimeWorker:
    def __init__(self, worker_id: str | UUID):
        self.worker_id = worker_id
        self.int_req_sock = zmq_ctx.socket(zmq.REQ)
        self.change_sub_sock = zmq_ctx.socket(zmq.SUB)

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
        self.int_req_sock.connect(internal_server_addr)
        self.change_sub_sock.connect(internal_server_addr)
        self.change_sub_sock.connect(api_server_addr)
        # TODO @Incomplete: get initial module state from zmq server, subscribe to changes
        module = None

        # TODO @Incomplete: trigger and tasks and send out updated module state

        # TODO @Incomplete: write back compilation results to zmq server
        pass

    async def stop(self):
        logger.info("runtime_worker.stop")
        self.int_req_sock.close()
        self.change_sub_sock.close()
