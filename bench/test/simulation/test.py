import dataclasses
import gc

import pytest
import structlog
from opentelemetry import trace

from bench.sql import BUILTIN_GLOBAL_SCHEMA, BUILTIN_REGIONAL_SCHEMA
from bench.system import StoreMap
from bench.test.conftest import TestProfile
from bench.test.fixtures import (
    create_test_db,
    delete_test_db,
    make_global_store,
    make_regional_store,
)
from bench.test.simulation.core import (
    BenchSpec,
    ClientSpec,
    HostSpec,
    Simulation,
    SimulationSpec,
    get_simulation_id,
)
from bench.test.simulation.workload import ReadPackageSpec, WatchLogsSpec, WriteBlockTreeSpec
from bench.utils.func import group_by

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

#
# Read and mark simulations from disk
#

# NOTE :Test: read simulation specs from disk? (some JSON schema + toml thing?)
#  probably also mark and categorize them? (see for example FoundationDB test specs)
BUILTIN_SIMULATIONS: list[SimulationSpec] = [
    #
    # Sanity
    #
    SimulationSpec(
        name="SingleClientEmpty",
        description="Create a single client with no workloads",
        profile=TestProfile.QUICK,
        clients=(ClientSpec(name="alice-1", user="alice"),),
    ),
    SimulationSpec(
        name="MultiClientEmpty",
        description="Create multiple clients with no workloads",
        profile=TestProfile.QUICK,
        clients=(
            ClientSpec(name="alice-1", user="alice"),
            ClientSpec(name="alice-2", user="alice"),
            ClientSpec(name="bob-1", user="bob"),
        ),
    ),
    SimulationSpec(
        name="SingleHostEmpty",
        description="Create a single host with no workloads",
        profile=TestProfile.QUICK,
        clients=(ClientSpec(name="alice-1", user="alice"),),
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
    ),
    SimulationSpec(
        name="MultiHostEmpty",
        description="Create multiple hosts with no workloads",
        profile=TestProfile.QUICK,
        clients=(
            ClientSpec(name="alice-1", user="alice"),
            ClientSpec(name="bob-1", user="bob"),
        ),
        hosts=(
            HostSpec(bench=BenchSpec(name="alice", owner="alice")),
            HostSpec(bench=BenchSpec(name="bob", owner="bob")),
        ),
    ),
    #
    # Simple
    #
    SimulationSpec(
        name="SingleWriterBlockTree",
        description="Write a block tree with one client, read with another client",
        clients=(ClientSpec(name="alice-1", user="alice"),),
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
        workloads=(
            WriteBlockTreeSpec(bench="alice", client="alice-1", transactions=10),
            ReadPackageSpec(bench="alice", client="alice-1"),
        ),
    ),
    SimulationSpec(
        name="SingleWriterMultiReaderBlockTree",
        description="Write a block tree with one client, read with multiple clients",
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
        clients=(
            ClientSpec(name="alice-1", user="alice"),
            ClientSpec(name="alice-2", user="alice"),
            ClientSpec(name="alice-3", user="alice"),
        ),
        workloads=(
            WriteBlockTreeSpec(
                bench="alice", client="alice-1", transactions=10, group="alice-0-main"
            ),
            ReadPackageSpec(bench="alice", client="alice-1", group="alice-0-main"),
            ReadPackageSpec(bench="alice", client="alice-2", group="alice-0-main"),
            ReadPackageSpec(bench="alice", client="alice-3", group="alice-0-main"),
        ),
    ),
    SimulationSpec(
        name="SingleWriterLogTail",
        description="Write a block tree with one client, watch logs multiple clients",
        clients=(
            ClientSpec(name="alice-1", user="alice"),
            ClientSpec(name="alice-2", user="alice"),
        ),
        hosts=(HostSpec(bench=BenchSpec(name="alice", owner="alice")),),
        workloads=(
            WriteBlockTreeSpec(bench="alice", client="alice-1", transactions=10),
            WatchLogsSpec(bench="alice", client="alice-1", tail_user="alice", group="alice-0-main"),
            WatchLogsSpec(bench="alice", client="alice-2", tail_user="alice", group="alice-0-main"),
        ),
    ),
    # TODO :Test!: test multi-writer, various write patterns, latency, ...
]
SIMULATIONS_BY_PROFILE = group_by(BUILTIN_SIMULATIONS, lambda s: s.profile)


async def run_builtin_simulation(spec: SimulationSpec):
    simulation_id = get_simulation_id(spec)
    global_store = make_global_store(f"test-{simulation_id}-global")
    regional_store = make_regional_store(f"test-{simulation_id}-regional")
    store_map = StoreMap({"*": regional_store})
    await create_test_db(global_store, BUILTIN_GLOBAL_SCHEMA)
    await create_test_db(regional_store, BUILTIN_REGIONAL_SCHEMA)
    simulation = Simulation(
        id=simulation_id,
        spec=spec,
        global_store=global_store,
        regional_store=regional_store,
        store_map=store_map,
    )
    try:
        await simulation.run()
        await delete_test_db(global_store)
    except Exception as e:
        logger.exception("simulation.error", simulation=simulation, error=e)
        raise
    finally:
        # force gc for simulation isolation
        del spec
        del global_store
        del regional_store
        del store_map
        del simulation
        gc.collect()


# NOTE: we lay out the simulation tests like below so so pytest collects them nicely
#  (organized by category and parameterized by simulation)


@pytest.mark.quick
@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.QUICK, ()), ids=lambda s: s.name
)
async def test_simulation_quick(spec: SimulationSpec):
    await run_builtin_simulation(spec)


@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.DEFAULT, ()), ids=lambda s: s.name
)
async def test_simulation_default(spec: SimulationSpec):
    await run_builtin_simulation(spec)


@pytest.mark.careful
@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.CAREFUL, ()), ids=lambda s: s.name
)
async def test_simulation_careful(spec: SimulationSpec):
    await run_builtin_simulation(spec)


@pytest.mark.paranoid
@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.PARANOID, ()), ids=lambda s: s.name
)
async def test_simulation_paranoid(spec: SimulationSpec, num_seeds: int = 1):
    for i in range(0, num_seeds):
        subspec = dataclasses.replace(spec, seed=spec.seed + i)
        await run_builtin_simulation(subspec)
