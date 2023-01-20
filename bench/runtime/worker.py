import typing
from dataclasses import dataclass, field
from itertools import chain
from typing import Optional
from uuid import UUID

import structlog
import zmq
import zmq.asyncio

from bench import language
from bench.language import wire
from bench.language.error import ParseError
from bench.language.parse import ErrorCollector, resolve
from bench.language.type import StatementPath, SymbolType
from bench.language.wire import ModuleReference, parse_symbol_type_node
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

InterpModule = typing.NamedTuple(
    "InterpModule",
    module_idx=Optional[language.ModuleIndex],
    errors=list[language.Error],
    dependencies=list[language.ModuleIndex],
)


@dataclass
class ModuleWorkerState:
    source: wire.ModuleData
    # interpreted state
    interp = InterpModule(module_idx=None, errors=[], dependencies=[])
    # derived from interpreted
    wire_module: wire.ModuleData | None = None
    wire_errors: list[wire.ErrorData] | None = None
    wire_dependencies: dict[UUID, wire.ModuleData] = field(default_factory=dict)

    def derive_wire(self):
        """Re-derives wire state from interpreted state"""
        if self.interp.module_idx:
            self.wire_module = wire.rmap_module(self.interp.module_idx.module)
        else:  # re-use source
            self.wire_module = self.source
        self.wire_errors = [wire.rmap_error(e) for e in self.interp.errors]
        self.wire_dependencies = {
            m.module.id: wire.rmap_module(m.module) for m in self.interp.dependencies
        }


def get_requirements(source: wire.ModuleData) -> set[ModuleReference]:
    """Returns the set of module ids required by the given module source (not transitive)"""
    requirements_ids: set[ModuleReference] = set()
    for statement in chain.from_iterable(file.statements for file in source.files):
        if statement.symbol_type == SymbolType.REQUIREMENT:
            if not isinstance(statement.reference_module.id, UUID):
                raise ValueError(f"requirement must specify reference module id: {statement}")
            requirements_ids.add(statement.reference_module)
    return requirements_ids


def lookup_in_dependencies(dependencies: list[language.ModuleIndex]):
    # assumes no conflicting names (checked in resolve)
    dependencies_by_name = {m.module.name: m for m in dependencies}

    def lookup(requirement: language.Requirement, path: StatementPath) -> language.Statement | None:
        idx: language.ModuleIndex = dependencies_by_name.get(requirement.name)
        if not idx:
            return None
        return idx.statements_by_path.get(path)

    return lookup


def interp_runtime(
    source: wire.ModuleData, dependencies: list[language.ModuleIndex]
) -> InterpModule:
    """Interprets the given module source with the given dependencies"""
    # TODO @Performance: interp and exec jobs should probably happen in a separate thread
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
        return InterpModule(module_idx=None, errors=errors, dependencies=dependencies)

    # resolve
    interp_module = wire.wmap_module(source)
    collector = ErrorCollector()
    module_idx = resolve(
        interp_module, lookup_in_module=lookup_in_dependencies(dependencies), on_error=collector
    )
    errors.extend([e.to_error() for e in collector.errors])

    return InterpModule(module_idx=module_idx, errors=errors, dependencies=dependencies)


class RuntimeWorker:
    def __init__(self, worker_id: str | UUID):
        self.worker_id = worker_id
        self.rep_sock = zmq_ctx.socket(zmq.REP)
        self.int_req_sock = zmq_ctx.socket(zmq.REQ)
        self.change_sub_sock = zmq_ctx.socket(zmq.SUB)
        self.change_pub_sock = zmq_ctx.socket(zmq.PUB)
        self.modules_interp_cache: dict[UUID, InterpModule] = {}
        self.working_states: dict[UUID, ModuleWorkerState] = {}

    async def get_wire_module(self, module_id: UUID) -> wire.ModuleData:
        """Gets a module's wire data (uncached)."""
        req_read_module = ZMessage(ZMessageType.REQ_READ_MODULE, ReqReadModulePayload(module_id))
        send_message(self.int_req_sock, req_read_module)
        _, payload = await recv_message_with(self.int_req_sock, RepReadModulePayload)
        return payload.module

    async def get_interp_module(self, module_id: UUID, cache: bool) -> InterpModule:
        """Gets a complete interpreted module incl. dependencies (optional caching)"""
        if cache and module_id in self.modules_interp_cache:
            return self.modules_interp_cache[module_id]
        wire_module = await self.get_wire_module(module_id)
        dependencies = await self.get_dependencies(wire_module)
        interp = interp_runtime(wire_module, dependencies)
        if cache and interp.module_idx is not None:  # only cache if we got a valid index
            self.modules_interp_cache[module_id] = interp
        return interp

    async def get_dependencies(self, source: wire.ModuleData) -> list[language.ModuleIndex]:
        """Resolves the source's requirements into dependencies (transitively)."""
        requirements = get_requirements(source)
        dependencies = []
        for module_reference in requirements:
            # dependencies are always cached?
            interp = await self.get_interp_module(module_reference.id, cache=True)
            if interp.errors:
                raise ValueError(f"dependency {module_reference} has errors: {interp.errors}")
            dependencies.append(interp.module_idx)
        return dependencies

    async def update_runtime(self, state: ModuleWorkerState) -> None:
        """Updates a module's runtime state by re-interpreting it with its dependencies."""
        dependencies = await self.get_dependencies(state.source)
        state.interp = interp_runtime(state.source, dependencies)
        state.derive_wire()

    async def init_worker_state(self, module_id: UUID) -> None:
        """Initializes a module-specific worker state (loading and indexing)"""
        source_module = await self.get_wire_module(module_id)
        state = ModuleWorkerState(source=source_module)
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
            try:
                await self.process_message(msg)
            except Exception as e:
                logger.exception("runtime_worker.process_message", exc_info=e, msg=msg)

    async def process_message(self, msg: ZMessage):
        if msg.type == ZMessageType.REQ_MODULE_RUNTIME:
            module_id = msg.payload_as(ReqModuleRuntimePayload).module_id
            state = await self.get_worker_state(module_id)
            rep_module_runtime = ZMessage(
                type=ZMessageType.REP_MODULE_RUNTIME,
                payload=RepModuleRuntimePayload(
                    state.wire_module, list(state.wire_dependencies.values()), state.wire_errors
                ),
            )
            send_message(self.rep_sock, rep_module_runtime)
        elif msg.type == ZMessageType.MODULE_CHANGED:
            module_id = msg.payload_as(ReqModuleRuntimePayload).module_id
            state = await self.get_worker_state(module_id)
            await self.update_runtime(state)
            rep_module_runtime = ZMessage(
                type=ZMessageType.REP_MODULE_RUNTIME,
                payload=RepModuleRuntimePayload(
                    state.wire_module, list(state.wire_dependencies.values()), state.wire_errors
                ),
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
