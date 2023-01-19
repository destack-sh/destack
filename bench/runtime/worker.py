from dataclasses import dataclass
from itertools import chain
from uuid import UUID

import structlog
import zmq
import zmq.asyncio

from bench import language
from bench.language import wire
from bench.language.error import ParseError
from bench.language.parse import ErrorCollector, lookup_in_error, resolve
from bench.language.wire import parse_symbol_type_node
from bench.zmq import (
    ZMessage,
    ZMessageType,
    recv_message_poll,
    recv_message_with,
    send_message,
    zmq_ctx,
)
from bench.zmq.messages import (
    RepModuleRuntimePayload,
    RepReadModulePayload,
    ReqModuleRuntimePayload,
    ReqReadModulePayload,
)

logger = structlog.get_logger(__name__)


@dataclass
class ModuleWorkerState:
    source: wire.ModuleData
    name: str
    # interpreted state
    interp_dependencies: dict[UUID, language.Module]
    interp_module: language.Module | None
    errors: list[language.Error]
    # derived from interpreted
    wire_module: wire.ModuleData | None = None
    wire_errors: list[wire.ErrorData] | None = None

    def derive_wire(self):
        """Re-derives wire state from interpreted state"""
        if self.interp_module:
            self.wire_module = wire.rmap_module(self.interp_module)
        else:  # re-use source
            self.wire_module = self.source
        self.wire_errors = [wire.rmap_error(e) for e in self.errors]


def update_runtime(state: ModuleWorkerState) -> None:
    state.errors = []
    # TODO @Incomplete: lookup dependencies
    #  and use for lookup_in_module and reference resolution in state.source via lookup_in_module

    # parse (not resolve) type nodes in place since source can contain arbitrary btl strings
    for statement in chain.from_iterable(file.statements for file in state.source.files):
        if not isinstance(statement.type_node, str):
            continue
        try:
            statement.type_node = parse_symbol_type_node(statement.symbol_type, statement.type_node)
        except ParseError as e:
            error = e.to_error()
            error.statement = statement  # technically not correct but we only need id
            state.errors.append(error)

    # bail if we have type errors (types must be valid for proper parse)
    if state.errors:
        state.interp_module = None
        state.derive_wire()  # update wire state
        return

    # update language module (from wire format)
    state.interp_module = wire.wmap_module(state.source)

    # resolve
    collector = ErrorCollector()
    _ = resolve(state.interp_module, lookup_in_module=lookup_in_error, on_error=collector)
    state.errors.extend([e.to_error() for e in collector.errors])

    # update wire state
    state.derive_wire()


class RuntimeWorker:
    def __init__(self, worker_id: str | UUID):
        self.worker_id = worker_id
        self.rep_sock = zmq_ctx.socket(zmq.REP)
        self.int_req_sock = zmq_ctx.socket(zmq.REQ)
        self.change_sub_sock = zmq_ctx.socket(zmq.SUB)
        self.change_pub_sock = zmq_ctx.socket(zmq.PUB)
        self.working_states: dict[UUID, ModuleWorkerState] = {}

    async def init_worker_state(self, module_id):
        # fetch module from internal server
        req_read_module = ZMessage(ZMessageType.REQ_READ_MODULE, ReqReadModulePayload(module_id))
        send_message(self.int_req_sock, req_read_module)
        _, payload = await recv_message_with(self.int_req_sock, RepReadModulePayload)

        # initialise runtime state
        state = ModuleWorkerState(
            source=payload.module,
            name=payload.module.name,
            interp_dependencies={},
            interp_module=None,
            errors=[],
        )
        update_runtime(state)  # should probably happen in a thread?
        self.working_states[module_id] = state

    async def get_worker_state(self, module_id: UUID) -> ModuleWorkerState:
        if module_id not in self.working_states:
            await self.init_worker_state(module_id)
        return self.working_states[module_id]

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
        self.change_sub_sock.setsockopt(zmq.SUBSCRIBE, b"")
        # self.change_pub_sock.bind(runtime_worker_addr) TODO @Incomplete: pub runtime changes
        poller = zmq.asyncio.Poller()
        poller.register(self.rep_sock, zmq.POLLIN)
        poller.register(self.change_sub_sock, zmq.POLLIN)

        while True:
            msg = await recv_message_poll(poller)
            await self.process_message(msg)

    async def process_message(self, msg: ZMessage):
        if msg.type == ZMessageType.REQ_MODULE_RUNTIME:
            module_id = msg.payload_as(ReqModuleRuntimePayload).module_id
            state = await self.get_worker_state(module_id)
            rep_module_runtime = ZMessage(
                type=ZMessageType.REP_MODULE_RUNTIME,
                payload=RepModuleRuntimePayload(state.wire_module, state.wire_errors),
            )
            send_message(self.rep_sock, rep_module_runtime)
        elif msg.type == ZMessageType.MODULE_CHANGED:
            module_id = msg.payload_as(ReqModuleRuntimePayload).module_id
            state = await self.get_worker_state(module_id)
            update_runtime(state)  # always update for now with full everything
            rep_module_runtime = ZMessage(
                type=ZMessageType.REP_MODULE_RUNTIME,
                payload=RepModuleRuntimePayload(state.wire_module, state.wire_errors),
            )
            send_message(self.change_pub_sock, rep_module_runtime)
        else:
            raise RuntimeError(f"unexpected message type: {msg.type}")

        # TODO @Incomplete: trigger jobs and send out consequent job and runtime changes
        # TODO @Incomplete: write back compilation results to zmq server

    async def stop(self):
        logger.info("runtime_worker.stop", worker_id=self.worker_id)
        self.rep_sock.close()
        self.int_req_sock.close()
        self.change_sub_sock.close()
        self.change_pub_sock.close()
