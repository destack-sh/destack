from dataclasses import dataclass, field
from itertools import chain
from uuid import UUID

import structlog
import zmq
import zmq.asyncio

from bench import language
from bench.language import wire
from bench.language.error import ParseError
from bench.language.parse import ErrorCollector, index_module, resolve
from bench.language.type import StatementPath, SymbolType
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
    # interpreted state
    interp_dependencies: dict[UUID, language.Module]
    interp_module: language.Module | None
    errors: list[language.Error]
    # derived from interpreted
    wire_dependencies: dict[UUID, wire.ModuleData] = field(default_factory=dict)
    wire_module: wire.ModuleData | None = None
    wire_errors: list[wire.ErrorData] | None = None

    def derive_wire(self):
        """Re-derives wire state from interpreted state"""
        if self.interp_module:
            self.wire_module = wire.rmap_module(self.interp_module)
        else:  # re-use source
            self.wire_module = self.source
        self.wire_errors = [wire.rmap_error(e) for e in self.errors]


def get_requirements(source: wire.ModuleData) -> set[UUID]:
    """Returns the set of module ids required by the given module source (not transitive)"""
    requirements_ids: set[UUID] = set()
    for statement in chain.from_iterable(file.statements for file in source.files):
        if statement.symbol_type == SymbolType.REQUIREMENT:
            if not isinstance(statement.reference_module, UUID):
                raise ValueError(f"requirement must specify reference module id: {statement}")
            requirements_ids.add(statement.reference_module)
    return requirements_ids


def lookup_in_dependencies(dependencies: list[language.ModuleIndex]):
    # assumes no conflicting names
    dependencies_by_name = {m.module.name: m for m in dependencies}

    def lookup(requirement: language.Requirement, path: StatementPath) -> language.Statement | None:
        idx: language.ModuleIndex = dependencies_by_name.get(requirement.name)
        if not idx:
            return None
        return idx.statements_by_path.get(path)

    return lookup


def interp_runtime(
    source: wire.ModuleData, dependencies: list[language.ModuleIndex]
) -> tuple[language.Module | None, list[language.Error]]:
    """Interprets the given module source with the given dependencies"""
    errors = []
    # parse (not resolve) type nodes in place as source contains btl strings
    for statement in chain.from_iterable(file.statements for file in source.files):
        if not isinstance(statement.type_node, str):
            continue
        try:
            statement.type_node = parse_symbol_type_node(statement.symbol_type, statement.type_node)
        except ParseError as e:
            error = e.to_error()
            error.statement = statement  # technically not correct but we only need id
            errors.append(error)
    if errors:  # types must be valid for proper parse
        return None, errors

    # resolve
    interp_module = wire.wmap_module(source)
    collector = ErrorCollector()
    _ = resolve(
        interp_module, lookup_in_module=lookup_in_dependencies(dependencies), on_error=collector
    )
    errors.extend([e.to_error() for e in collector.errors])

    return interp_module, errors


class RuntimeWorker:
    def __init__(self, worker_id: str | UUID):
        self.worker_id = worker_id
        self.rep_sock = zmq_ctx.socket(zmq.REP)
        self.int_req_sock = zmq_ctx.socket(zmq.REQ)
        self.change_sub_sock = zmq_ctx.socket(zmq.SUB)
        self.change_pub_sock = zmq_ctx.socket(zmq.PUB)
        self.modules_idx_cache: dict[UUID, language.ModuleIndex] = {}
        self.working_states: dict[UUID, ModuleWorkerState] = {}

    async def get_wire_module(self, module_id: UUID) -> wire.ModuleData:
        """Gets a module's wire data (uncached)."""
        req_read_module = ZMessage(ZMessageType.REQ_READ_MODULE, ReqReadModulePayload(module_id))
        send_message(self.int_req_sock, req_read_module)
        _, payload = await recv_message_with(self.int_req_sock, RepReadModulePayload)
        return payload.module

    async def get_dependency_module_idx(self, module_id: UUID) -> language.ModuleIndex:
        """Gets a dependency's language module index (potentially cached)."""
        if module_id in self.modules_idx_cache:
            return self.modules_idx_cache[module_id]
        # get, parse and index dependency module
        wire_module = await self.get_wire_module(module_id)
        # TODO @Incomplete: get dependencies of dependency
        interp_module, errors = interp_runtime(wire_module, [])
        if errors:
            raise ValueError(f"dependency module has errors: {errors}")
        idx = index_module(interp_module)
        self.modules_idx_cache[module_id] = idx
        return idx

    async def update_runtime(self, state: ModuleWorkerState):
        """Updates a module's runtime state by re-interpreting it with its dependencies."""
        requirements = get_requirements(state.source)
        dependencies = []
        for requirement_id in requirements:
            dependency = await self.get_dependency_module_idx(requirement_id)
            dependencies.append(dependency)
        interp_module, errors = interp_runtime(state.source, dependencies)
        state.interp_dependencies = {d.module.id: d.module for d in dependencies}
        state.interp_module = interp_module
        state.errors = errors
        state.derive_wire()

    async def init_worker_state(self, module_id):
        # fetch module from internal server
        source_module = await self.get_wire_module(module_id)
        # initialise runtime state
        state = ModuleWorkerState(
            source=source_module,
            interp_dependencies={},
            interp_module=None,
            errors=[],
        )
        await self.update_runtime(state)
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
            await self.update_runtime(state)
            rep_module_runtime = ZMessage(
                type=ZMessageType.REP_MODULE_RUNTIME,
                payload=RepModuleRuntimePayload(state.wire_module, state.wire_errors),
            )
            send_message(self.change_pub_sock, rep_module_runtime)
        else:
            raise RuntimeError(f"unexpected message type: {msg.type}")

        # TODO @Incomplete: trigger jobs and send out consequent job and runtime changes
        # TODO @Incomplete: write back compilation results (to internal server)
        # TODO @Incomplete: stream back runtime results & frames (to api server)

    async def stop(self):
        logger.info("runtime_worker.stop", worker_id=self.worker_id)
        self.rep_sock.close()
        self.int_req_sock.close()
        self.change_sub_sock.close()
        self.change_pub_sock.close()
