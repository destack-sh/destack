# ruff: noqa: E402

import pytest

from bench.test.conftest import _setup_test_env
from bench.test.simulation.core import SimulatedEventLoopPolicy

# NOTE: must run setup before importing from bench
_setup_test_env()


# NOTE: simulation tests must be run with one event loop per function to isolate
@pytest.fixture
def event_loop_policy():
    return SimulatedEventLoopPolicy()
