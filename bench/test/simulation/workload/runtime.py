from dataclasses import dataclass
from typing import Any, Awaitable, Callable, final, override
from uuid import UUID

from bench.language import (
    SOURCE_NODE_TYPES,
    Bench,
    Block,
    BlockType,
    NodeMode,
    NodeSuperGraph,
    Package,
    Run,
    RunnableNode,
    Session,
)
from bench.language.runtime.run import RunOptions
from bench.runtime import MemoryCache, Runner, Runtime
from bench.runtime.core.runner import create_run_from_node
from bench.test.simulation.core import ClientHandle, HostHandle, make_remote_session

from .spec import WorkloadSpec, WorkloadType
from .workload import Workload, workload_


@dataclass
class RuntimeWorkloadSpec(WorkloadSpec):
    client: str = ""
    bench: str = ""


class RuntimeWorkload[SpecT: RuntimeWorkloadSpec](Workload[SpecT]):
    """Workloads that run on a single Client with a Runtime."""

    @override
    @final
    async def prepare(self):
        self.bench_id: UUID = self.simulation.resolve_bench_id(self.spec.bench)
        self.client: ClientHandle = self.simulation.get_client(self.spec.client)
        self.host: HostHandle = self.simulation.get_host(self.spec.bench)
        self.session: Session = await make_remote_session(
            simulation=self.simulation,
            bench_id=self.bench_id,
            client=self.client,
            host=self.host,
            oracle=self.oracle,
        )
        self.supergraph: NodeSuperGraph = self.session._supergraph

        await self.session.open(_set_in_context=False)
        async with self.session.active(readonly=False):
            self.bench = await Bench.get(id=self.bench_id, live=True)
            self.session.parent = self.bench  # patch in the session parent
            self.main_package = await (
                Package.include_descendants(*SOURCE_NODE_TYPES)
                .include_ancestors(Bench)
                .select_all()
                .deselect(Bench.encryption_key)
                .get(self.bench.main_package_ptr, live=True)
            )
            await self.prepare_in_session()
            await self.session.commit()

    async def prepare_in_session(self):
        """Prepare the workload in the given session."""
        pass  # do nothing by default

    @override
    @final
    async def run_once(self):
        async with self.session.active(readonly=False):
            await self.run_once_in_session()
            await self.session.commit()

    async def run_once_in_session(self):
        """Runs one repetition of the workload in the given session."""
        pass


@dataclass
class RuntimeLambdaWorkloadSpec(RuntimeWorkloadSpec):
    """Spec for a workload that runs a lambda."""

    func: Callable[["RuntimeLambdaWorkload"], Awaitable[None]] | None = None


@workload_(WorkloadType.RUNTIME_LAMBDA, RuntimeLambdaWorkloadSpec)
class RuntimeLambdaWorkload(RuntimeWorkload[RuntimeLambdaWorkloadSpec]):
    """Workload that runs a lambda."""

    @override
    async def prepare_in_session(self):
        self.runtime = Runtime(
            session=self.session, cache=MemoryCache(self.bench), oracle=self.oracle
        )

    @override
    async def run_once_in_session(self):
        assert self.spec.func is not None, f"{self!r} has no lambda to run"
        await self.spec.func(self)

    def page(self, name: str = "Page1") -> Block:
        """Gets or creates a page in the current package."""
        page = self.main_package.blocks.get(name)
        if page is None:
            page = Block.new(BlockType.PAGE, name=name)
            self.main_package.blocks.append(page)
        return page

    async def commit(self):
        """Commits the current session."""
        return await self.session.commit()

    async def run_in_runtime(
        self,
        run: Run | RunnableNode,
        *,
        variables: Any | None = None,
        inputs: Any | None = None,
        options: RunOptions | None = None,
        mode: NodeMode | None = None,
        return_error: bool = False,
    ) -> Runner:
        """Run something in the Runtime."""
        if not isinstance(run, Run):
            run = create_run_from_node(
                run,
                isolate=True,
                variables=variables,
                inputs=inputs,
                options=options,
                mode=mode,
                parent=self.bench,
                session=self.session,
            )
        runner = await self.runtime.run(run, return_error=return_error)
        assert runner is not None, f"no runner for {run!r}"
        return runner
