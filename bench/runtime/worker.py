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
class ModuleWorkerState:
    source: wire.ModuleData
    name: str
    module: Module | None
    files: list[wire.FileData]
    errors: list[wire.ErrorData]


def rmap_module(state: ModuleWorkerState) -> wire.ModuleData:
    return wire.ModuleData(
        id=state.source.id,
        name=state.name,
        files=state.files,
    )


def parse_statement_type_node(statement: wire.StatementData) -> None:
    # TODO @Cleanup: parse_statement_type_node should live in language/wire
    btl_parser = parser_from_string(statement.type_node)
    if statement.symbol_type in (SymbolType.TASK, SymbolType.CODE):
        statement.type_node = parse_type_node_func(btl_parser, name=None)
    elif statement.symbol_type == SymbolType.DATASET:
        statement.type_node = parse_type_node_struct_inline(btl_parser, name=None)
    elif statement.symbol_type == SymbolType.TYPE:
        # hacky way to determine whether it's an inline redef or struct def
        if statement.type_node.startswith("("):
            btl_parser.eat_bracket("(")
            statement.type_node = parse_type_node_struct_inline(btl_parser, name=None)
            btl_parser.eat_bracket(")")
        else:
            statement.type_node = parse_type_node_struct(btl_parser, name=None)
    else:
        raise ValueError(f"unexpected symbol type {statement.symbol_type}")


def update_runtime(state: ModuleWorkerState) -> None:
    errors: list[language.Error] = []
    # parse (not resolve) type nodes in place since source can contain arbitrary btl strings
    source = state.source
    for statement in chain.from_iterable(file.statements for file in source.files):
        if not isinstance(statement.type_node, str):
            continue
        try:
            parse_statement_type_node(statement)
        except ParseError as e:
            errors.append(e.to_error())
    if errors:  # don't proceed resolving (need proper type nodes)
        state.errors = [wire.rmap_error(e) for e in errors]
        return

    # update language module (from wire format)
    module = wire.wmap_module(source)
    state.module = module

    # resolve
    collector = ErrorCollector()
    # TODO @Incomplete: load requirement's modules (and cache in state)
    resolve(module, lookup_module=error_module_lookup, on_error=collector)
    state.errors = [wire.rmap_error(e) for e in collector.errors]
    state.files = [wire.rmap_file(file) for file in module.files]


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
            source=payload.module, name=payload.module.name, module=None, files=[], errors=[]
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
        # self.change_pub_sock.bind(runtime_worker_addr)
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
                payload=RepModuleRuntimePayload(rmap_module(state), state.errors),
            )
            send_message(self.rep_sock, rep_module_runtime)
        elif msg.type == ZMessageType.MODULE_CHANGED:
            module_id = msg.payload_as(ReqModuleRuntimePayload).module_id
            state = await self.get_worker_state(module_id)
            update_runtime(state)  # always update for now with full everything
            rep_module_runtime = ZMessage(
                type=ZMessageType.REP_MODULE_RUNTIME,
                payload=RepModuleRuntimePayload(rmap_module(state), state.errors),
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
