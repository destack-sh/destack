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
from bench.language.parse import ErrorCollector, interp, resolve
from bench.language.type import Compilation, StatementPath, SymbolType
from bench.language.wire import ModuleReference, parse_symbol_type_node
from bench.runtime.compile import compile
from bench.zmq import (
    ZMessage,
    ZMessageType,
    recv_message_poll,
    recv_message_with,
    send_message,
    zmq_ctx,
)
from bench.zmq.messages import (
    ModuleChangedPayload,
    ModuleRuntimeChangedPayload,
    RepModuleCompilePayload,
    RepModuleRuntimePayload,
    RepReadModulePayload,
    ReqModuleCompilePayload,
    ReqModuleRunPayload,
    ReqModuleRuntimePayload,
    ReqReadModulePayload,
    ReqWriteModulePayload,
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
    # cached wire representations (derived from interpreted)
    wire_module: wire.ModuleData | None = None
    wire_errors: list[wire.ErrorData] | None = None
    wire_dependencies: dict[UUID, wire.ModuleData] = field(default_factory=dict)

    @property
    def interpreted(self) -> bool:
        return self.interp.module_idx is not None

    def derive_wire(self):
        """Re-derives wire state from interpreted state"""
        if self.interp.module_idx:
            self.wire_module = wire.rmap_module(self.interp.module_idx.module)
        else:  # re-use source (if failed to parse or not yet parsed)
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

    def lookup(
        requirement: language.RequirementContent, path: StatementPath
    ) -> language.Scope | None:
        idx: language.ModuleIndex = dependencies_by_name.get(requirement.module_name)
        if not idx:
            return None
        return idx.get_scope(path)

    return lookup


def interp_runtime(
    source: wire.ModuleData, dependencies: list[language.ModuleIndex]
) -> InterpModule:
    """Interprets the given module source with the given dependencies"""
    # TODO @Performance: interp and exec jobs should probably happen in a separate thread
    errors = impute_parsed_type_nodes(source)
    if errors:  # types must be valid for proper parse
        return InterpModule(module_idx=None, errors=errors, dependencies=dependencies)

    # resolve
    interp_module = wire.wmap_module(source)
    collector = ErrorCollector()
    module = lookup_in_dependencies(dependencies)
    module_idx = resolve(interp_module, lookup_in_module=module, on_error=collector)
    interp(module_idx, on_error=collector)
    errors.extend([e.to_error() for e in collector.errors])

    return InterpModule(module_idx=module_idx, errors=errors, dependencies=dependencies)


def impute_parsed_type_nodes(source: wire.ModuleData):
    errors = []
    # TODO @Cleanup: parsing from wire module should not need to happen at all
    #  (once we've switched to full type nodes in DB)
    # parse (not resolve) type nodes in place as source contains btl strings
    for statement in chain.from_iterable(file.statements for file in source.files):
        if statement.type_node is None:
            continue
        elif not isinstance(statement.type_node, str):
            # type node may already be parsed (either from wire or from a previous interp if partial changes)
            continue
        try:
            statement.type_node = parse_symbol_type_node(statement.symbol_type, statement.type_node)
        except ParseError as e:
            error = e.to_error()
            error.statement = statement  # technically not correct but we only need id
            errors.append(error)
    return errors


class RuntimeWorker:
    """Manages and executes the runtime of a set of modules (incl. static analysis)"""

    def __init__(self, worker_id: str | UUID):
        self.worker_id = worker_id
        self.rep_sock = zmq_ctx.socket(zmq.REP)
        self.intserver_req_sock = zmq_ctx.socket(zmq.REQ)
        self.sub_sock = zmq_ctx.socket(zmq.SUB)
        self.pub_sock = zmq_ctx.socket(zmq.PUB)
        self.modules_interp_cache: dict[UUID, InterpModule] = {}
        self.working_states: dict[UUID, ModuleWorkerState] = {}

    async def fetch_wire_module(self, module_id: UUID) -> wire.ModuleData:
        """Gets a module's wire data (uncached)."""
        logger.info("fetch_wire_module", module_id=module_id)
        send_message(
            self.intserver_req_sock,
            ZMessageType.REQ_READ_MODULE,
            ReqReadModulePayload(module_id),
        )
        _, payload = await recv_message_with(self.intserver_req_sock, RepReadModulePayload)
        return payload.module

    async def get_interp_module(self, module_id: UUID, cache: bool) -> InterpModule:
        """Gets a complete interpreted module incl. dependencies (optional caching)"""
        logger.info("interp_module", module_id=module_id, cache=cache)
        if cache and module_id in self.modules_interp_cache:
            return self.modules_interp_cache[module_id]
        wire_module = await self.fetch_wire_module(module_id)
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
        logger.info("update_runtime", source=state.source)
        dependencies = await self.get_dependencies(state.source)
        state.interp = interp_runtime(state.source, dependencies)
        state.derive_wire()

    async def init_worker_state(
        self, module_id: UUID, source_module: wire.ModuleData | None = None
    ) -> ModuleWorkerState:
        """Initializes a module-specific worker state (loading and indexing)"""
        if not source_module:
            source_module = await self.fetch_wire_module(module_id)
        state = ModuleWorkerState(source=source_module)
        await self.update_runtime(state)
        self.working_states[module_id] = state
        return state

    async def get_worker_state(self, module_id: UUID) -> ModuleWorkerState:
        if module_id not in self.working_states:
            await self.init_worker_state(module_id)
        return self.working_states[module_id]

    async def run(
        self,
        runtime_worker_rep_addr: str,
        runtime_worker_pub_addr: str,
        internal_server_rep_addr: str,
        internal_server_pub_addr: str,
    ):
        logger.info(
            "start",
            worker_id=self.worker_id,
            runtime_worker_addr=runtime_worker_rep_addr,
            runtime_worker_pub_addr=runtime_worker_pub_addr,
            internal_server_rep_addr=internal_server_rep_addr,
            internal_server_pub_addr=internal_server_pub_addr,
        )
        self.rep_sock.bind(runtime_worker_rep_addr)
        self.intserver_req_sock.connect(internal_server_rep_addr)
        self.sub_sock.connect(internal_server_pub_addr)
        self.sub_sock.setsockopt(zmq.SUBSCRIBE, b"")
        self.pub_sock.bind(runtime_worker_pub_addr)
        poller = zmq.asyncio.Poller()
        poller.register(self.rep_sock, zmq.POLLIN)
        poller.register(self.sub_sock, zmq.POLLIN)

        while True:
            msg = await recv_message_poll(poller)
            try:
                await self.process_message(msg)
            except Exception as e:
                logger.exception("runtime_worker.process_message", exc_info=e, msg=msg)

    async def process_message(self, msg: ZMessage):
        logger.debug("process_message", request=msg)
        if msg.type == ZMessageType.REQ_MODULE_RUNTIME:
            module_id = msg.payload_as(ReqModuleRuntimePayload).module_id
            state = await self.get_worker_state(module_id)
            send_message(
                self.rep_sock,
                ZMessageType.REP_MODULE_RUNTIME,
                RepModuleRuntimePayload(
                    state.wire_module, list(state.wire_dependencies.values()), state.wire_errors
                ),
            )
        elif msg.type == ZMessageType.MODULE_CHANGED:
            change = msg.payload_as(ModuleChangedPayload)
            if change.module_id not in self.working_states:
                state = await self.init_worker_state(change.module_id, change.module)
            else:
                state = await self.get_worker_state(change.module_id)
                # TODO @Robustness: overwriting entire module source is not great :PartialModuleUpdates
                state.source = change.module
                await self.update_runtime(state)
            send_message(
                self.pub_sock,
                ZMessageType.MODULE_RUNTIME_CHANGED,
                ModuleRuntimeChangedPayload(
                    state.source.id,
                    state.wire_module,
                    list(state.wire_dependencies.values()),
                    state.wire_errors,
                ),
            )
        elif msg.type == ZMessageType.REQ_MODULE_COMPILE:
            payload = msg.payload_as(ReqModuleCompilePayload)
            state = await self.get_worker_state(payload.module_id)
            if not state.interpreted:
                logger.debug("fail_compile", module_id=payload.module_id)
                send_message(
                    self.rep_sock,
                    ZMessageType.REP_MODULE_COMPILE,
                    RepModuleCompilePayload(success=False),
                )
                return

            compilation = state.interp.module_idx.symbol_by_id(payload.compilation_id, Compilation)
            compile_result = await compile(compilation)
            send_message(
                self.rep_sock,
                ZMessageType.REP_MODULE_COMPILE,
                RepModuleCompilePayload(success=True),
            )
            compiled_file = compile_result.to_file(module=state.interp.module_idx.module)
            write = ReqWriteModulePayload(
                module_id=state.source.id,
                files=[wire.rmap_file(compiled_file)],
            )
            send_message(self.intserver_req_sock, ZMessageType.REQ_WRITE_MODULE, write)
        elif msg.type == ZMessageType.REQ_MODULE_RUN:
            payload = msg.payload_as(ReqModuleRunPayload)
            raise NotImplementedError("TODO @Incomplete: implement REQ_MODULE_RUN")
        else:
            raise RuntimeError(f"unexpected message type: {msg.type}")

        # TODO @Incomplete: trigger jobs and send out consequent job and runtime changes
        # TODO @Incomplete: stream back runtime results & frames (to api server)

    async def stop(self):
        logger.info("stop", worker_id=self.worker_id)
        self.rep_sock.close()
        self.intserver_req_sock.close()
        self.sub_sock.close()
        self.pub_sock.close()
