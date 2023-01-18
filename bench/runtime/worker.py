from dataclasses import dataclass
from itertools import chain
from uuid import UUID

import structlog
import zmq
import zmq.asyncio

from bench import language
from bench.language import Module, SymbolType, wire
from bench.language.error import ParseError
from bench.language.parse import (
    ErrorCollector,
    error_module_lookup,
    parse_type_node_func,
    parse_type_node_struct,
    parse_type_node_struct_inline,
    parser_from_string,
    resolve,
)
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
class Job:
    id: UUID
    name: str


@dataclass
class ModuleWorkerState:
    source: wire.ModuleData
    name: str
    module: Module | None
    symbols: list[wire.StatementData]
    errors: list[wire.ErrorData]


def update_worker_state(state: ModuleWorkerState) -> None:
    errors: list[language.Error] = []
    # parse (not resolve) type nodes in place since source can contain arbitrary btl strings
    source = state.source
    for statement in chain.from_iterable(file.statements for file in source.files):
        if not isinstance(statement.type_node, str):
            continue
        try:
            # TODO @Incomplete: parse inline type definition (alias redefinition)
            btl_parser = parser_from_string(statement.type_node)
            if statement.symbol_type in (SymbolType.TASK, SymbolType.CODE):
                statement.type_node = parse_type_node_func(btl_parser, name=None)
            elif statement.symbol_type == SymbolType.DATASET:
                statement.type_node = parse_type_node_struct_inline(btl_parser, name=None)
            elif statement.symbol_type == SymbolType.TYPE:
                statement.type_node = parse_type_node_struct(btl_parser, name=None)
        except ParseError as e:
            errors.append(e.to_error())

    # if any errors so far, don't proceed resolving (need proper type nodes)
    if errors:
        state.errors = [wire.rmap_error(e) for e in errors]
        return

    # update module
    module = wire.wmap_module(source)
    state.module = module

    # resolve
    collector = ErrorCollector()
    # TODO @Incomplete: load requirement's modules (and cache in state)
    resolve(module, lookup_module=error_module_lookup, on_error=collector)
    state.errors = [wire.rmap_error(e) for e in collector.errors]


class RuntimeWorker:
    def __init__(self, worker_id: str | UUID):
        self.worker_id = worker_id
        self.rep_sock = zmq_ctx.socket(zmq.REP)
        self.int_req_sock = zmq_ctx.socket(zmq.REQ)
        self.change_sub_sock = zmq_ctx.socket(zmq.SUB)
        self.working_states: dict[UUID, ModuleWorkerState] = {}

    async def get_worker_state(self, module_id: UUID) -> ModuleWorkerState:
        if module_id not in self.working_states:
            req_read_module = ZMessage(
                ZMessageType.REQ_READ_MODULE, ReqReadModulePayload(module_id)
            )
            send_message(self.int_req_sock, req_read_module)
            _, payload = await recv_message_with(self.int_req_sock, RepReadModulePayload)
            state = ModuleWorkerState(
                source=payload.module, name=payload.module.name, module=None, symbols=[], errors=[]
            )
            update_worker_state(state)  # should probably happen in a thread?
            self.working_states[module_id] = state

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

        poller = zmq.asyncio.Poller()
        poller.register(self.rep_sock, zmq.POLLIN)
        poller.register(self.change_sub_sock, zmq.POLLIN)

        while True:
            msg = await recv_message_poll(poller)

            if msg.type == ZMessageType.REQ_MODULE_RUNTIME:
                module_id = msg.payload_as(ReqModuleRuntimePayload).module_id
                state = await self.get_worker_state(module_id)
                reply_payload = RepModuleRuntimePayload(
                    module_id, files=[], symbols=state.symbols, errors=state.errors
                )
                rep_module_runtime = ZMessage(ZMessageType.REP_MODULE_RUNTIME, reply_payload)
                send_message(self.rep_sock, rep_module_runtime)

            # TODO @Incomplete: get initial module state from zmq server, subscribe to changes
            # TODO @Incomplete: trigger and tasks and send out updated module state
            # TODO @Incomplete: write back compilation results to zmq server

    async def stop(self):
        logger.info("runtime_worker.stop", worker_id=self.worker_id)
        self.rep_sock.close()
        self.int_req_sock.close()
        self.change_sub_sock.close()
