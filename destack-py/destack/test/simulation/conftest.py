import pytest

from destack.test.simulation.core.oracle import SimulatedEventLoopPolicy


# NOTE: simulation tests must be run with one event loop per function to isolate
@pytest.fixture
def event_loop_policy():
    return SimulatedEventLoopPolicy()
