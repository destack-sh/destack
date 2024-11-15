# ruff: noqa: E402

import pytest

from bench.test.conftest import _setup_test_env
from bench.test.simulation.oracle import SimulatedEventLoopPolicy

# NOTE: must run setup before importing from bench
_setup_test_env()


# NOTE: simulation tests must be run with one event loop per function to isolate
#  (and use our custom event loop for fast-forwarding support)
@pytest.fixture
def event_loop_policy():
    return SimulatedEventLoopPolicy()
