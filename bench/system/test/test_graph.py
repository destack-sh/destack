from typing import NamedTuple

import structlog
from hypothesis.strategies import DataObject

from bench.proto.wire import (
    SupervisorStub,
)
from bench.system.test.conftest import UserHandle

# nocheckin :Robustness! :Test!: test GraphIO much more thoroughly (see hypothesis, FoundationDB, ...)

logger = structlog.get_logger(__name__)


class SimulationConfig(NamedTuple):
    seed: int
    num_clients: int
    num_rounds: int
    num_edits_per_round: int


async def run(data: DataObject):
    pass


async def test_graph_converge_single_node_conflicts(supervisor: SupervisorStub):
    """Simultaneously update a User's name from multiple clients, should converge."""

    pass


async def test_graph_handle_successive_updates(supervisor: SupervisorStub):
    """Update a node multiple times in the same transaction in succession, should converge."""

    pass


async def test_graph_update_node_with_invalid_property(
    some_user: UserHandle, supervisor: SupervisorStub
):
    """Update a User property to an invalid value, should be rejected."""

    pass
