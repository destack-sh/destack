import asyncio

from bench.language import (
    TERMINAL_RUN_STATUSES,
    Action,
    ActionType,
    Channel,
    Flow,
    LinkType,
    Message,
    Package,
    Run,
    RunStatus,
    Session,
    Trigger,
    code,
    text,
)
from bench.runtime import create_run_from_node
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


@simulated_runtime(system=True)
async def test_builtin_package(simulation: Simulation, runtime: RuntimeLambdaWorkload):  # noqa: RUF029
    """Test the Builtin package."""
    from bench.builtin import BuiltinPackage as BuiltinPackageRaw
    from bench.language import BENCH_BUILTIN_PACKAGE_PTR

    # builtin graph in memory and builtin graph loaded from runtime/bench should be equal
    BuiltinPackageLoaded = runtime.main_package._supergraph.get_or_error(BENCH_BUILTIN_PACKAGE_PTR)
    assert isinstance(BuiltinPackageLoaded, Package)
    BuiltinPackageLoadedGraph = BuiltinPackageLoaded._graph.copy()
    loaded_bench_bench = BuiltinPackageLoaded.parent
    assert loaded_bench_bench is not None
    if (main_store := loaded_bench_bench.main_store) is not None:
        BuiltinPackageLoadedGraph.remove(main_store)
    BuiltinPackageLoadedGraph.remove(loaded_bench_bench, recursive=False)
    assert_graph_equals(BuiltinPackageRaw._graph, BuiltinPackageLoadedGraph)


@simulated_runtime(system=True, runtimes=True)
async def test_start_run(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Run and wait for it to execute in another Runtime."""
    Page1 = runtime.page()
    Flow1 = Flow.new("Flow1")
    Page1.append(Flow1)
    await runtime.commit()

    run = create_run_from_node(Flow1)
    await run.wait_until_terminated()
    assert run.status == RunStatus.COMPLETED


@simulated_runtime(system=True, runtimes=True)
async def test_start_run_from_message(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Run from a Message in a Flow. Should be lifted into a Flow Run."""
    Channel1 = Channel.new("General")
    Flow1 = Flow.new("Flow1")
    Start1 = Action.new(ActionType.START, "Start1", triggers=[Trigger.on_message()])
    End1 = Action.new(ActionType.END, "End1")
    Flow1.extend(Start1, End1)
    runtime.page().extend(Channel1, Flow1)
    await runtime.commit()

    Message1 = Message.new(text=text("Hello, [@Flow1]!", {"Flow1": Flow1}))
    Channel1.messages.append(Message1)
    await runtime.commit()

    Run1 = await Run.get_run_of(Flow1, where=TERMINAL_RUN_STATUSES)
    assert Run1.status == RunStatus.COMPLETED


@simulated_runtime(system=True, runtimes=True)
async def test_reply_to_message(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Reply to a Message in a Flow. Should end and not recurse endlessly."""
    Channel1 = Channel.new("General")
    Flow1 = Flow.new("Flow1")
    Start1 = Action.new(ActionType.START, "Start1", triggers=[Trigger.on_message()])
    Code1 = Action.new(
        ActionType.CODE,
        "Code1",
        code=code("""\
message_in = runner.get_latest_run(Start1).inputs.message
reply = Message.new(title="Hi.", reply_to=message_in)
message_in.parent.append(reply)
"""),
    )
    End1 = Action.new(ActionType.END, "End1")
    Flow1.extend(Start1, Code1, End1)
    Start1.connect(LinkType.REQUIRE, Code1, is_manual=True)
    Code1.connect(LinkType.REQUIRE, End1, is_manual=True)
    runtime.page().extend(Channel1, Flow1)
    await runtime.commit()

    Message1 = Message.new(text=text("Hello, [@Flow1]!", {"Flow1": Flow1}))
    Channel1.messages.append(Message1)
    await runtime.commit()

    Run1 = await Run.get_run_of(Flow1, where=TERMINAL_RUN_STATUSES)
    assert Run1.status == RunStatus.COMPLETED


# nocheckin :Incomplete: wait, could we just use Interruptions to handle runtime "Triggers" for Messages?
#  (we could just Interrupt and then manually resolve that Interrupt whenever we send a Message
#   .. somehow? doesn't that move the logic to the client?)


@simulated_runtime(system=True, runtimes=True)
async def test_pause_resume_run(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Run a long async Flow and pause it, then resume it."""
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start", triggers=[Trigger.on_start()])
    Action1 = Action.new(ActionType.CODE, "Action1", code=code("await sleep(1)"))
    End = Action.new(ActionType.END, "End")
    Flow1.actions.extend(Start, Action1, End)
    Start.connect(LinkType.REQUIRE, Action1, is_manual=True)
    Action1.connect(LinkType.REQUIRE, End, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    async def pause_run(run: Run):
        run.pause()
        await runtime.session.commit()

    # run, pause, then resume
    run = create_run_from_node(Flow1)
    await runtime.commit()
    asyncio.get_event_loop().call_later(0.5, lambda: asyncio.create_task(pause_run(run)))
    await run.wait_until_status(RunStatus.PAUSED, *TERMINAL_RUN_STATUSES)
    assert run.status == RunStatus.PAUSED
    run.resume()
    await runtime.commit()  # should automatically resume within runtime
    await run.wait_until_terminated()
    assert run.status == RunStatus.COMPLETED
