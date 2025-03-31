import abc
from dataclasses import dataclass
from typing import Any, Awaitable, Callable, override

from bench.language import NodeMode, Run, RunnableNode, RunOptions, RunStatus
from bench.runtime import MemoryCache, Runner, Runtime, create_run
from bench.test.simulation.core import Simulation

from .client import ClientWorkload, ClientWorkloadSpec
from .spec import WorkloadType
from .workload import workload_


@dataclass
class RuntimeWorkloadSpec(ClientWorkloadSpec):
    pass


class RuntimeWorkload[SpecT: RuntimeWorkloadSpec](ClientWorkload[SpecT], abc.ABC):
    """Workloads that run on a single Client with a Runtime."""

    @override
    async def prepare_in_session(self):
        self.runtime = Runtime(
            session=self.session,
            network=self.simulation.network.network,
            cache=MemoryCache(self.bench),
            oracle=self.oracle,
        )
        await self.runtime.start()

    async def run_in_runtime(
        self,
        run: Run | RunnableNode,
        *,
        inputs: Any | None = None,
        options: RunOptions | None = None,
        mode: NodeMode | None = None,
        return_error: bool = False,
    ) -> Runner:
        """Run something in the Runtime."""
        if not isinstance(run, Run):
            run, _ = create_run(
                run,
                inputs=inputs,
                options=options,
                mode=mode,
                parent=self.main_package,
                session=self.session,
                status=RunStatus.QUEUED,
            )
            await self.session.commit()
        runner = await self.runtime.run(run, _return_error=return_error)
        await self.session.commit()
        assert runner is not None, f"no runner for {run!r}"
        return runner


@dataclass
class RuntimeLambdaWorkloadSpec(RuntimeWorkloadSpec):
    """Spec for a workload that runs a lambda."""

    func: Callable[["Simulation", "RuntimeLambdaWorkload"], Awaitable[None]] | None = None


@workload_(WorkloadType.RUNTIME_LAMBDA, RuntimeLambdaWorkloadSpec)
class RuntimeLambdaWorkload(RuntimeWorkload[RuntimeLambdaWorkloadSpec]):
    """Workload that runs a lambda."""

    @override
    async def run_once_in_session(self):
        assert self.spec.func is not None, f"{self!r} has no lambda to run"
        await self.spec.func(self.simulation, self)
