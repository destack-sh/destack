import abc
from dataclasses import dataclass
from typing import Awaitable, Callable, final, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import SOURCE_NODE_TYPES, Bench, Package, Page, TextLineIn, text_line
from bench.test.simulation.core import Simulation, make_remote_session

from .spec import WorkloadSpec, WorkloadType
from .workload import Workload, workload_

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass
class ClientWorkloadSpec(WorkloadSpec):
    client: str = ""
    bench: str = ""


class ClientWorkload[SpecT: ClientWorkloadSpec](Workload[SpecT], abc.ABC):
    """Workloads that run on a single Client."""

    def page(self, title: TextLineIn = "Page1") -> Page:
        """Gets or creates a page in the current package."""
        title = text_line(title)
        page = self.main_package.pages.find(lambda p: p.title == title)
        if page is None:
            page = Page.new(title=title)
            self.main_package.pages.append(page)
        return page

    async def commit(self):
        """Commits the current session."""
        return await self.session.commit()

    @override
    @final
    async def prepare(self):
        self.bench_id: UUID = self.simulation.get_bench_id(self.spec.bench)
        self.client = self.simulation.get_client(self.spec.client)
        self.host = self.simulation.get_host(self.spec.bench)
        self.session = await make_remote_session(
            simulation=self.simulation,
            source_id=self.id,
            bench_id=self.bench_id,
            client=self.client,
            host=self.host,
            oracle=self.oracle,
        )
        self.supergraph = self.session._supergraph

        await self.session.open(_set_in_context=False)
        async with self.session.active():
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
