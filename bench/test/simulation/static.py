import dataclasses

import pytest
import structlog
from opentelemetry import trace

from bench.test.conftest import TestProfile
from bench.test.simulation.core import (
    BenchSpec,
    ClientSpec,
    ComputerSpec,
    HostSpec,
    RuntimeSpec,
    SimulationSpec,
    UserSpec,
    run_simulation,
)
from bench.test.simulation.workload import ReadPackageSpec, WatchLogsSpec, WritePageTreeSpec
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
        users=(UserSpec(name="alice"),),
        clients=(ClientSpec(name="alice-1", parent=("user", "alice")),),
    ),
    SimulationSpec(
        name="MultiClientEmpty",
        description="Create multiple clients with no workloads",
        profile=TestProfile.QUICK,
        users=(UserSpec(name="alice"), UserSpec(name="bob")),
        clients=(
            ClientSpec(name="alice-1", parent=("user", "alice")),
            ClientSpec(name="alice-2", parent=("user", "alice")),
            ClientSpec(name="bob-1", parent=("user", "bob")),
        ),
    ),
    SimulationSpec(
        name="SingleHostEmpty",
        description="Create a single host with no workloads",
        profile=TestProfile.QUICK,
        users=(UserSpec(name="alice"),),
        clients=(ClientSpec(name="alice-1", parent=("user", "alice")),),
        benches=(BenchSpec(name="alice", owner="alice"),),
        hosts=(HostSpec(bench="alice"),),
    ),
    SimulationSpec(
        name="MultiHostEmpty",
        description="Create multiple hosts with no workloads",
        profile=TestProfile.QUICK,
        users=(UserSpec(name="alice"), UserSpec(name="bob")),
        clients=(
            ClientSpec(name="alice-1", parent=("user", "alice")),
            ClientSpec(name="bob-1", parent=("user", "bob")),
        ),
        benches=(BenchSpec(name="alice", owner="alice"), BenchSpec(name="bob", owner="bob")),
        hosts=(HostSpec(bench="alice"), HostSpec(bench="bob")),
    ),
    SimulationSpec(
        name="SingleRuntimeEmpty",
        description="Create a single runtime with no workloads",
        profile=TestProfile.QUICK,
        users=(UserSpec(name="alice"),),
        computers=(ComputerSpec(name="alice-computer", bench="alice"),),
        clients=(ClientSpec(name="alice-1", parent=("user", "alice")),),
        benches=(BenchSpec(name="alice", owner="alice"),),
        hosts=(HostSpec(bench="alice"),),
        runtimes=(RuntimeSpec(name="alice-runtime", computer="alice-computer"),),
    ),
    #
    # Simple
    #
    SimulationSpec(
        name="SingleWriterPageTree",
        description="Write a page tree with one client, read with another client",
        users=(UserSpec(name="alice"),),
        clients=(ClientSpec(name="alice-1", parent=("user", "alice")),),
        benches=(BenchSpec(name="alice", owner="alice"),),
        hosts=(HostSpec(bench="alice"),),
        workloads=(
            WritePageTreeSpec(bench="alice", client="alice-1", transactions=10),
            ReadPackageSpec(bench="alice", client="alice-1"),
        ),
    ),
    SimulationSpec(
        name="SingleWriterMultiReaderPageTree",
        description="Write a page tree with one client, read with multiple clients",
        users=(UserSpec(name="alice"),),
        clients=(
            ClientSpec(name="alice-1", parent=("user", "alice")),
            ClientSpec(name="alice-2", parent=("user", "alice")),
            ClientSpec(name="alice-3", parent=("user", "alice")),
        ),
        benches=(BenchSpec(name="alice", owner="alice"),),
        hosts=(HostSpec(bench="alice"),),
        workloads=(
            WritePageTreeSpec(
                bench="alice", client="alice-1", transactions=10, group="alice-0-main"
            ),
            ReadPackageSpec(bench="alice", client="alice-1", group="alice-0-main"),
            ReadPackageSpec(bench="alice", client="alice-2", group="alice-0-main"),
            ReadPackageSpec(bench="alice", client="alice-3", group="alice-0-main"),
        ),
    ),
    SimulationSpec(
        name="SingleWriterLogTail",
        description="Write a page tree with one client, watch logs multiple clients",
        users=(UserSpec(name="alice"),),
        clients=(
            ClientSpec(name="alice-1", parent=("user", "alice")),
            ClientSpec(name="alice-2", parent=("user", "alice")),
        ),
        benches=(BenchSpec(name="alice", owner="alice"),),
        hosts=(HostSpec(bench="alice"),),
        workloads=(
            WritePageTreeSpec(bench="alice", client="alice-1", transactions=10),
            WatchLogsSpec(bench="alice", client="alice-1", tail_user="alice", group="alice-0-main"),
            WatchLogsSpec(bench="alice", client="alice-2", tail_user="alice", group="alice-0-main"),
        ),
    ),
    # TODO :Test!: test multi-writer, various write patterns, latency, ...
]
SIMULATIONS_BY_PROFILE = group_by(BUILTIN_SIMULATIONS, lambda s: s.profile)


# NOTE: we lay out the simulation tests like below so so pytest collects them nicely
#  (organized by category and parameterized by simulation)


@pytest.mark.quick
@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.QUICK, ()), ids=lambda s: s.name
)
async def test_simulation_quick(spec: SimulationSpec):
    await run_simulation(spec)


@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.DEFAULT, ()), ids=lambda s: s.name
)
async def test_simulation_default(spec: SimulationSpec):
    await run_simulation(spec)


@pytest.mark.careful
@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.CAREFUL, ()), ids=lambda s: s.name
)
async def test_simulation_careful(spec: SimulationSpec):
    await run_simulation(spec)


@pytest.mark.paranoid
@pytest.mark.parametrize(
    "spec", SIMULATIONS_BY_PROFILE.get(TestProfile.PARANOID, ()), ids=lambda s: s.name
)
async def test_simulation_paranoid(spec: SimulationSpec, num_seeds: int = 1):
    for i in range(0, num_seeds):
        subspec = dataclasses.replace(spec, seed=spec.seed + i)
        await run_simulation(subspec)
