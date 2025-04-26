from dataclasses import dataclass
from typing import override

import structlog
from opentelemetry import trace

from bench.language import (
    EditType,
    NodeType,
    Page,
    repr_enums,
)
from bench.language.core.const import RESOURCE_NODE_TYPES
from bench.test.simulation.core import (
    SampledFloat,
    SampledInt,
    Simulation,
    to_value,
)
from bench.test.simulation.workload.client import ClientWorkload, ClientWorkloadSpec
from bench.test.utils import assert_graph_equals
from bench.utils.oracle import Oracle

from .spec import WorkloadType
from .workload import Workload, workload_

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass
class WritePageTreeSpec(ClientWorkloadSpec):
    type: WorkloadType = WorkloadType.WRITE_PAGE_TREE
    edit_types: tuple[EditType, ...] = (EditType.CREATE, EditType.ARCHIVE, EditType.DELETE)
    transactions: int | SampledInt = 1
    transactions_interval: float | SampledFloat = 0.0
    edits_per_transaction: int | SampledInt = 10
    live: bool = True


@workload_(WorkloadType.WRITE_PAGE_TREE, WritePageTreeSpec)
class WritePageTreeWorkload(ClientWorkload[WritePageTreeSpec]):
    """Write a random tree of pages."""

    def __init__(self, id: str, spec: WritePageTreeSpec, oracle: Oracle, simulation: Simulation):
        super().__init__(id, spec, oracle, simulation)
        self.page_num = 0
        self.all_edits: list[tuple[EditType, Page]] = []

    def __str__(self):
        return f"edit_types={repr_enums(self.spec.edit_types)}"

    @override
    async def run_once_in_session(self):
        max_transactions = to_value(self.random, self.spec.transactions)
        n_transactions = 0
        while n_transactions < max_transactions:
            # make random edits
            max_edits = to_value(self.random, self.spec.edits_per_transaction)
            for _ in range(max_edits):
                edit_type = self.random.choice(self.spec.edit_types)
                pages = self.main_package._graph.nodes_of_type(Page)
                if edit_type == EditType.CREATE:
                    parent = self.random.choice((self.main_package, *pages))
                    page = Page.new(title=f"Page {self.page_num}")
                    self.page_num += 1
                    parent.pages.append(page)
                elif edit_type == EditType.ARCHIVE:
                    if not pages:
                        continue  # no pages yet
                    page = self.random.choice(pages)
                    page.archive()
                elif edit_type == EditType.DELETE:
                    if not pages:
                        continue  # no pages yet
                    page = self.random.choice(pages)
                    page.delete()
                else:
                    raise NotImplementedError(f"unexpected edit type {edit_type}")
                self.all_edits.append((edit_type, page))
            await self.session.commit()
            n_transactions += 1

            # check we're in sync with host
            host = self.simulation.get_host(self.spec.bench)
            assert_graph_equals(
                self.main_package._graph,
                host.service.main_package._graph,
                # NOTE :Test: Resources should also be in sync in Runtime?
                #  (for some reason they're not right now)
                ignore_node_types=(NodeType.BENCH, *RESOURCE_NODE_TYPES),
            )

            # and wait for next tx
            wait = to_value(self.random, self.spec.transactions_interval)
            await self.oracle.sleep(wait)


@dataclass
class ReadPackageSpec(ClientWorkloadSpec):
    type: WorkloadType = WorkloadType.READ_PACKAGE
    live: bool = True


@workload_(WorkloadType.READ_PACKAGE, ReadPackageSpec)
class ReadPackageWorkload(ClientWorkload[ReadPackageSpec]):
    """Reads an entire package."""

    def __str__(self):
        return f"pkg={self.main_package!r}"

    @override
    async def check_group(self, group: list[Workload]):
        # check that all packages are the same
        for other in group:
            if other is self or not isinstance(other, ClientWorkload):
                continue
            assert_graph_equals(self.main_package._graph, other.main_package._graph)
