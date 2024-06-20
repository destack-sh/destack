import asyncio
import time
from random import Random
from typing import final

import pytest
import structlog
from opentelemetry import trace

from bench.language import Store
from bench.sql.engine import GLOBAL_SCHEMA
from bench.test.fixtures import create_test_db, make_global_store
from bench.test.simulation.activity import ActivityBase, get_activity_cls
from bench.test.simulation.oracle import SimulatedLoop
from bench.test.simulation.spec import BenchSpec, ClientSpec, NetworkSpec, SimulationSpec
from bench.utils.oracle import REAL_ORACLE

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def get_simulation_id(simulation: SimulationSpec) -> str:
    return f"{simulation.name}-{simulation.seed}"


@final
class Simulation:
    """An active simulation"""

    def __init__(self, id: str, spec: SimulationSpec, global_store: Store):
        self.id = id
        self.spec = spec
        self.global_store = global_store

        # init state
        self.random = Random(spec.seed)
        self.network = Network(spec.network, self)
        self.clients_by_name: dict[str, ClientSpec] = {
            client.name: client for client in spec.clients
        }
        self.loop = SimulatedLoop(base_time_ns=time.time_ns)
        self.activities: list[ActivityBase] = []
        for activity_spec in spec.activities:
            activity_cls = get_activity_cls(activity_spec.type)
            activity = activity_cls(activity_spec, self)
            self.activities.append(activity)

        # runtime state
        self.started_at_ns = None
        self.finished_at_ns = None

    def __str__(self):
        return f"{self.id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    def get_client(self, name: str):
        client = self.clients_by_name.get(name)
        assert (
            client is not None
        ), f"{self!r} has no client: {name} (available: {list(self.clients_by_name)})"
        return client

    async def run(self):
        """Run the simulation."""
        with tracer.start_as_current_span("simulation.prepare"):
            # create clients
            ...

            # create benches
            ...

            # prepare activities
            await asyncio.gather(*(activity.prepare() for activity in self.activities))
        logger.info("simulation.start", simulation=self)

        self.started_at_ns = REAL_ORACLE.time_ns()
        with tracer.start_as_current_span("simulation.run"):
            # run activities
            ...
        logger.info("simulation.run", simulation=self)


@final
class Network:
    """A network for connecting services and clients"""

    def __init__(self, spec: NetworkSpec, simulation: Simulation):
        self.spec = spec
        self.simulation = simulation


#
# Read and mark simulations from disk
#

# nocheckin: read simulation specs from disk
HARDCODED_SIMULATIONS: list[SimulationSpec] = [
    SimulationSpec(
        name="single_client_rw", benches=(BenchSpec(),), clients=(ClientSpec(name="clienta"),)
    )
]


@pytest.mark.parametrize("spec", HARDCODED_SIMULATIONS, ids=lambda s: s.name)
async def test_simulation(spec: SimulationSpec):
    simulation_id = get_simulation_id(spec)
    global_store = make_global_store(f"test_{simulation_id}")
    await create_test_db(global_store, GLOBAL_SCHEMA)
    simulation = Simulation(simulation_id, spec, global_store)
    try:
        await simulation.run()
    except Exception as e:
        logger.exception("simulation.error", simulation=simulation, error=e)
        raise
