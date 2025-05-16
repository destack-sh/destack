from bench.language import Membership, Message, Thread, text
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


def test_print_examples():
    """Just import the examples to make sure they work."""
    from bench.runtime.agent.example import EXAMPLES

    assert EXAMPLES

    for example in EXAMPLES:
        print("=" * 32)  # noqa: T201
        print(example.title)  # noqa: T201
        print("=" * 32)  # noqa: T201
        print(example.code)  # noqa: T201
        print("=" * 32)  # noqa: T201


@simulated_runtime(system=True, runtimes=True)
async def test_bench_agent_get_reply_to_message(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Create a Run by messaging an Identity in a Flow."""
    from bench.builtin import BenchAgent

    # create Thread in separate tx to test loading
    Thread1 = Thread.new("Test Thread", memberships=[Membership.new(BenchAgent)])
    runtime.main_package.add_child(Thread1)
    await runtime.commit()

    # submit message
    Message1 = Message.new(text=text("Hello!"))
    Thread1.add_child(Message1)
    await runtime.commit()
