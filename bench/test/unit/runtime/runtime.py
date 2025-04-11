import asyncio

from bench.language import (
    TERMINAL_PROCESS_STATUSES,
    Action,
    ActionType,
    Agent,
    Flow,
    LinkType,
    Membership,
    Message,
    Node,
    Package,
    ProcessStatus,
    Run,
    Session,
    Thread,
    code,
    text,
)
from bench.runtime import create_run
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime
from bench.test.utils import assert_graph_equals
from bench.utils.func import reload_module


def test_make_builtin_package(session: Session) -> None:
    """Make a Builtin package. Make it again and check they're equal."""
    from bench import builtin

    prev_graph = builtin.BuiltinPackage._graph.copy()

    reload_module(builtin)

    from bench import builtin

    assert prev_graph is not builtin.BuiltinPackage._graph
    assert_graph_equals(prev_graph, builtin.BuiltinPackage._graph)

    # print for reference :Builtins
    node_by_path: dict[str, Node] = {}
    for node in builtin.BuiltinPackage._graph.nodes:
        path = builtin.get_stable_builtin_path(node)
        node_by_path[path] = node

    paths = sorted(node_by_path.keys())
    for path in paths:
        node = node_by_path[path]
        print(f"{node.id} - {path} - {node.metatype.name}")  # noqa: T201


@simulated_runtime(system=True)
async def test_builtin_package(simulation: Simulation, runtime: RuntimeLambdaWorkload):  # noqa: RUF029
    """Test the Builtin package."""
    from bench.builtin import BuiltinPackage as BuiltinPackageRaw
    from bench.language import BENCH_BENCH_PACKAGE_PTR

    # builtin graph in memory and builtin graph loaded from runtime/bench should be equal
    BuiltinPackageLoaded = runtime.main_package._supergraph.get_or_error(BENCH_BENCH_PACKAGE_PTR)
    assert isinstance(BuiltinPackageLoaded, Package)
    BuiltinPackageLoadedGraph = BuiltinPackageLoaded._graph.copy()
    loaded_bench_bench = BuiltinPackageLoaded.parent
    assert loaded_bench_bench is not None
    if (main_store := loaded_bench_bench.main_store) is not None:
        BuiltinPackageLoadedGraph.remove(main_store)
    BuiltinPackageLoadedGraph.remove(loaded_bench_bench, recursive=False)
    for node in BuiltinPackageLoaded._graph.nodes:
        if isinstance(node, Membership):
            BuiltinPackageLoadedGraph.remove(node, recursive=False)
    assert_graph_equals(BuiltinPackageRaw._graph, BuiltinPackageLoadedGraph)


@simulated_runtime(system=True, runtimes=True)
async def test_start_run(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Run and wait for it to execute in another Runtime."""
    Page1 = runtime.page()
    Flow1 = Flow.new("Flow1")
    Page1.append(Flow1)
    await runtime.commit()

    run, _ = create_run(Flow1, parent=runtime.main_package)
    await run.wait_until_terminated()
    assert run.status == ProcessStatus.COMPLETED


# nocheckin: start Agent from .. Message? something?


@simulated_runtime(system=True, runtimes=True)
async def test_start_run_from_message(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Run by messaging an Identity in a Flow."""
    Flow1 = Flow.new("Flow1")
    Start1 = Action.new(ActionType.START, "Start1")
    End1 = Action.new(ActionType.END, "End1")
    Flow1.extend(Start1, End1)
    Start1.connect(LinkType.MANUAL, End1)
    Agent1 = Agent.new("Agent", main_flow=Flow1)
    Page1 = runtime.page()
    Page1.extend(Flow1, Agent1)
    await runtime.commit()

    # create Thread in separate tx to test loading
    Thread1 = Thread.new("Test Thread", memberships=[Membership.new(Agent1)])
    runtime.main_package.append(Thread1)
    await runtime.commit()

    # submit message
    Message1 = Message.new(text=text("Hello!"))
    Thread1.messages.append(Message1)
    await runtime.commit()

    Run1 = await Run.get_run_of(Flow1, where=TERMINAL_PROCESS_STATUSES)
    assert Run1.status == ProcessStatus.COMPLETED
    assert Run1.agent == Agent1


@simulated_runtime(system=True, runtimes=True)
async def test_pause_resume_run(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a long async Flow and pause it, then resume it."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Action1 = Action.new(ActionType.CODE, "Action1", code=code("await sleep(1)"))
    End = Action.new(ActionType.END, "End")
    Flow1.actions.extend(Start, Action1, End)
    Start.connect(LinkType.MANUAL, Action1)
    Action1.connect(LinkType.MANUAL, End)
    runtime.page().append(Flow1)
    await runtime.commit()

    async def pause_run(run: Run):
        run.pause()
        await runtime.session.commit()

    # run, pause, then resume
    run, _ = create_run(Flow1, parent=runtime.main_package)
    await runtime.commit()
    asyncio.get_event_loop().call_later(0.5, lambda: asyncio.create_task(pause_run(run)))
    await run.wait_until_status(ProcessStatus.PAUSED, *TERMINAL_PROCESS_STATUSES)
    assert run.status == ProcessStatus.PAUSED
    run.resume()
    await runtime.commit()  # should automatically resume within runtime
    await run.wait_until_terminated()
    assert run.status == ProcessStatus.COMPLETED
