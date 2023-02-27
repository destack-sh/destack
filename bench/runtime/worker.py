import asyncio
import typing
from dataclasses import dataclass, field
from datetime import datetime
from functools import partial
from itertools import chain
from typing import Optional
from uuid import UUID

import pytz
import structlog
import zmq
import zmq.asyncio

from bench import language
from bench.language import wire
from bench.language.parse import ErrorCollector, interp, resolve
from bench.language.type import Build, StatementPath, SymbolType
from bench.language.wire import ModuleReference
from bench.msg import (
    ZMessage,
    ZMessageType,
    recv_message_poll,
    recv_message_with,
    send_message,
    zmq_ctx,
)
from bench.msg.messages import (
    ExecutionChangedPayload,
    ModuleBuildErrorType,
    ModuleChangedPayload,
    ModuleRunErrorType,
    ModuleRuntimeChangedPayload,
    RepModuleBuildPayload,
    RepModuleRunPayload,
    RepModuleRuntimePayload,
    RepReadModulePayload,
    RepWriteModulePayload,
    ReqModuleBuildPayload,
    ReqModuleRunPayload,
    ReqModuleRuntimePayload,
    ReqReadModulePayload,
    ReqWriteModulePayload,
    as_key,
)
from bench.runtime.build import BuildResult, make_build
from bench.runtime.execute import Proxy, instantiate, run
from bench.runtime.tracing import ExecutionTracer, MultiTracer, ValidationTracer
from bench.runtime.type import CodeInstance, ExecutionFrame, ExecutionFrameData, TaskInstance
from bench.utils.uuidt import UUIDT

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
    def module_id(self) -> UUID:
        return self.source.id

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
    # resolve
    logger.info("interp_runtime", module=source)
    module = wire.wmap_module(source)
    # TODO @Accuracy: revert explicit statement references to StatementPath to lookup refs properly
    collector = ErrorCollector()
    module_idx = resolve(
        module, lookup_in_module=lookup_in_dependencies(dependencies), on_error=collector
    )
    interp(module_idx, on_error=collector)
    errors = [e.to_error() for e in collector.errors]

    return InterpModule(module_idx=module_idx, errors=errors, dependencies=dependencies)


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

    # TODO @Cleanup: runtime worker should be more functional
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
        worker_rep_addr: str,
        worker_pub_addr: str,
        intserver_rep_addr: str,
        intserver_pub_addr: str,
    ):
        logger.info(
            "start",
            worker_id=self.worker_id,
            worker_addr=worker_rep_addr,
            worker_pub_addr=worker_pub_addr,
            intserver_rep_addr=intserver_rep_addr,
            intserver_pub_addr=intserver_pub_addr,
        )
        self.rep_sock.bind(worker_rep_addr)
        self.intserver_req_sock.connect(intserver_rep_addr)
        self.sub_sock.connect(intserver_pub_addr)
        self.sub_sock.setsockopt(zmq.SUBSCRIBE, as_key(ZMessageType.MODULE_CHANGED))
        self.pub_sock.bind(worker_pub_addr)
        poller = zmq.asyncio.Poller()
        poller.register(self.rep_sock, zmq.POLLIN)
        poller.register(self.sub_sock, zmq.POLLIN)

        while True:
            msg = await recv_message_poll(poller)
            try:
                await self.process_message(msg)
            except Exception as e:
                logger.exception("worker.process_message", exc_info=e, msg=msg)

    async def process_message(self, msg: ZMessage):
        logger.debug("process_message", request=msg)
        if msg.type == ZMessageType.REQ_MODULE_RUNTIME:
            module_id = msg.payload_as(ReqModuleRuntimePayload).module_id
            state = await self.get_worker_state(module_id)
            send_message(
                self.rep_sock,
                ZMessageType.REP_MODULE_RUNTIME,
                RepModuleRuntimePayload(
                    updated_at=datetime.utcnow().replace(tzinfo=pytz.utc),
                    module=state.wire_module,
                    dependencies=list(state.wire_dependencies.values()),
                    errors=state.wire_errors,
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
                    module_id=state.source.id,
                    updated_at=datetime.utcnow().replace(tzinfo=pytz.utc),
                    module=state.wire_module,
                    dependencies=list(state.wire_dependencies.values()),
                    errors=state.wire_errors,
                ),
            )
        elif msg.type == ZMessageType.REQ_MODULE_BUILD:
            payload: ReqModuleBuildPayload = msg.payload_as(ReqModuleBuildPayload)
            state = await self.get_worker_state(payload.module_id)
            send_rep = partial(send_message, self.rep_sock, ZMessageType.REP_MODULE_BUILD)

            # get the builds to run
            if not state.interpreted:
                logger.debug("fail_build", module_id=payload.module_id, msg=msg)
                send_rep(RepModuleBuildPayload(error=ModuleBuildErrorType.NOT_READY))
                return
            buildable = state.interp.module_idx.symbol_by_id(payload.buildable_id)
            if isinstance(buildable, language.Task):
                # collect any builds that contain this task
                builds = []
                for build in state.interp.module_idx.symbols_of_type(Build):
                    if not build.is_definition:
                        continue
                    if any(t.definition.id == buildable.id for t in build.tasks):
                        builds.append(build)
            elif isinstance(buildable, language.Build):
                builds = [buildable]
            else:
                logger.debug("fail_build", module_id=payload.module_id, msg=msg)
                send_rep(RepModuleBuildPayload(error=ModuleBuildErrorType.INVALID_BUILDABLE))
                return

            # actually build (concurrently)
            # :BlockingWorkerMessages
            build_processes = [make_build(build) for build in builds]
            build_results: list[BuildResult] = await asyncio.gather(
                *build_processes, return_exceptions=False
            )
            build_ids = [r.id for r in builds]
            send_rep(RepModuleBuildPayload(error=None, build_ids=build_ids))

            # write back results
            for build_result in build_results:
                generated_file = build_result.to_file(module=state.interp.module_idx.module)
                write = ReqWriteModulePayload(
                    module_id=state.source.id,
                    files=[wire.rmap_file(generated_file)],
                    generated_mappings=[(build_result.build.id, build_result.source_mappings)],
                )
                send_message(self.intserver_req_sock, ZMessageType.REQ_WRITE_MODULE, write)
                _, write_result = await recv_message_with(
                    self.intserver_req_sock, RepWriteModulePayload
                )
                if not write_result.success:
                    # TODO @Robustness: panic if we can't write back builds?
                    logger.error("write_module_failed", write=write, write_result=write_result)
        elif msg.type == ZMessageType.REQ_MODULE_RUN:
            payload: ReqModuleRunPayload = msg.payload_as(ReqModuleRunPayload)
            state = await self.get_worker_state(payload.module_id)
            send_rep = partial(send_message, self.rep_sock, ZMessageType.REP_MODULE_RUN)

            # get the runconfig to run
            if not state.interpreted:
                send_rep(RepModuleRunPayload(error=ModuleRunErrorType.NOT_READY))
                return
            idx = state.interp.module_idx
            try:
                build = idx.get_symbol(payload.build, Build) if payload.build else None
                runnable_type = SymbolType[payload.runnable_type] if payload.runnable_type else None
                runnable = idx.symbol(payload.runnable, symbol_t=runnable_type)
            except (TypeError, KeyError) as e:
                send_rep(RepModuleRunPayload(error=ModuleRunErrorType.INVALID_RUNCONFIG))
                return

            # instantiate & run
            root_id = UUIDT()  # root execution id is pre-set for tracking
            try:  # instantiate
                tracker = forward_execution_capture(root_id, self.pub_sock)
                tracer = MultiTracer(
                    [ExecutionTracer(state.module_id, tracker), ValidationTracer()]
                )
                runnable_instance = instantiate(
                    runnable, idx=idx, build=build, proxy=Proxy(tracer=tracer)
                )
            except Exception as e:
                logger.exception("", exc_info=e)
                send_rep(RepModuleRunPayload(root_id, error=ModuleRunErrorType.INTERNAL_ERROR))
                return
            try:  # run
                if isinstance(runnable_instance, TaskInstance):
                    code_instance = runnable_instance.code
                elif not isinstance(runnable_instance, CodeInstance):
                    send_rep(
                        RepModuleRunPayload(root_id, error=ModuleRunErrorType.INVALID_RUNCONFIG)
                    )
                    return
                else:
                    code_instance = runnable_instance
                logger.info("run", code_instance=code_instance)
                # :BlockingWorkerMessages
                ret = await run(code_instance, payload.arguments)
            except Exception as e:
                logger.exception("run_failed", exc_info=e)
                send_rep(RepModuleRunPayload(root_id, error=ModuleRunErrorType.RUNTIME_ERROR))
                return
            send_rep(RepModuleRunPayload(root_id, error=None, output=ret))
        else:
            raise RuntimeError(f"unexpected message type: {msg.type}")

        # TODO @Incomplete: auto-trigger build/generate jobs on source change

    async def stop(self):
        logger.info("stop", worker_id=self.worker_id)
        self.rep_sock.close()
        self.intserver_req_sock.close()
        self.sub_sock.close()
        self.pub_sock.close()


def forward_execution_capture(root_id: UUID, pub_sock: zmq.Socket):
    def _do_track(frame: ExecutionFrame):
        # TODO @Performance: batch execution frame updates
        if frame.root is None:
            frame.id = root_id  # set root to fixed id (in-place)
        frame_data = ExecutionFrameData.from_frame(frame)
        logger.debug("execution.track", frame=frame_data.id)
        send_message(
            pub_sock,
            ZMessageType.EXECUTION_CHANGED,
            ExecutionChangedPayload(frame.module_id, frames=[frame_data]),
        )

    return _do_track
