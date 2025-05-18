import abc
from dataclasses import dataclass
from typing import Awaitable, Callable, final, override

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language import (
    BENCH_PTR,
    BENCH_SLUG,
    LOADED_PACKAGE_NODE_TYPES,
    Bench,
    NodeReference,
    NodeType,
    Page,
    TextLineIn,
    text_line,
)
from bench.test.simulation.core import Simulation, make_remote_session

from .spec import WorkloadSpec, WorkloadType
from .workload import Workload, workload_

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass
class ClientWorkloadSpec(WorkloadSpec):
    client: str = ""
    bench: str = ""
    system: bool = False


class ClientWorkload[SpecT: ClientWorkloadSpec](Workload[SpecT], abc.ABC):
    """Workloads that run on a single Client."""

    def page(self, title: TextLineIn = "Page1") -> Page:
        """Gets or creates a page in the current package."""
        title = text_line(title)
        pages = self.main_package.get_children(Page)
        page = next((p for p in pages if p.title == title), None)
        if page is None:
            page = Page.new(title=title)
            self.main_package.add_child(page)
        return page

    async def commit(self):
        """Commits the current session."""
        return await self.session.commit()

    @override
    @final
    async def prepare(self):
        self.bench_id: UUID = self.simulation.get_bench_id(self.spec.bench)
        self.client = self.simulation.get_client(self.spec.client)
        self.self_host = self.simulation.get_host(self.spec.bench)
        if self.spec.system:
            self.bench_host = self.simulation.get_host(BENCH_SLUG)
        else:
            self.bench_host = None
        self.session = await make_remote_session(
            simulation=self.simulation,
            source_id=self.id,
            bench_id=self.bench_id,
            client=self.client,
            host=self.self_host,
            oracle=self.oracle,
            system=self.spec.system,
        )
        self.supergraph = self.session._supergraph

        await self.session.open(_set_in_context=False)
        async with self.session.active():
            bench_ptr = NodeReference(
                node_type=NodeType.BENCH, id=self.bench_id, bench_id=self.bench_id
            )
            self.bench = (
                await Bench.include_descendants(NodeType.PACKAGE, *LOADED_PACKAGE_NODE_TYPES)
                .select_all()
                .get(bench_ptr, live=True)
            )
            main_package = self.bench.package
            assert main_package is not None, f"{self!r} has no main package"
            self.main_package = main_package
            self.session.parent = self.bench  # patch in the session parent
            if self.bench_host is not None:
                self.bench_bench = await Bench.include_descendants(*LOADED_PACKAGE_NODE_TYPES).get(
                    BENCH_PTR, live=True
                )
            else:
                self.bench_bench = None
            await self.prepare_in_session()
            await self.session.commit()

    async def prepare_in_session(self):
        """Prepare the workload in the given session."""
        pass  # do nothing by default

    @override
    @final
    async def run_once(self):
        async with self.session.active():
            await self.run_once_in_session()
            await self.session.commit()

    async def run_once_in_session(self):
        """Runs one repetition of the workload in the given session."""
        pass


@dataclass
class ClientLambdaWorkloadSpec(ClientWorkloadSpec):
    """Spec for a workload that runs a lambda."""

    func: Callable[["Simulation", "ClientLambdaWorkload"], Awaitable[None]] | None = None


@workload_(WorkloadType.CLIENT_LAMBDA, ClientLambdaWorkloadSpec)
class ClientLambdaWorkload(ClientWorkload[ClientLambdaWorkloadSpec]):
    """Workload that runs a lambda."""

    @override
    async def run_once_in_session(self):
        assert self.spec.func is not None, f"{self!r} has no lambda to run"
        await self.spec.func(self.simulation, self)
