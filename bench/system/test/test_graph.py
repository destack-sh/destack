from typing import NamedTuple, cast
from uuid import uuid4

import grpclib
import structlog

from bench.conftest import raises_grpc_error
from bench.language import Property, User
from bench.language.const import EditType, NodeType
from bench.language.expression import NodeReference
from bench.language.transaction import new_edit_id, pack_node_delta
from bench.proto import wiring
from bench.proto.wire import (
    CommitTransactionRequest,
    EditData,
    GraphIoStub,
    GraphScope,
    SupervisorStub,
)
from bench.system.test.conftest import UserHandle
from bench.utils.dt import utcnow

# TODO :Robustness! :Test!: test GraphIO much more thoroughly (see hypothesis, FoundationDB, ...)

logger = structlog.get_logger(__name__)


class SimulationConfig(NamedTuple):
    seed: int
    num_clients: int
    num_rounds: int
    num_edits_per_round: int


async def test_graph_converge_single_node_conflicts(supervisor: SupervisorStub):
    """Simultaneously update a User's name from multiple clients, should converge."""

    config = SimulationConfig(seed=42, num_clients=2, num_rounds=3, num_edits_per_round=1)
    user, clients = await make_clients(supervisor, config)
    consumers = [EditConsumer(client) for client in clients]
    producers = [
        EditProducer(
            client,
            seed_nodes=[user],
            edit_types=[EditType.UPDATE],
            node_types=[],
            update_properties={NodeType.USER: [User.name]},
        )
        for client in clients
    ]
    await run_and_check_simulation(
        consumers=consumers,
        producers=producers,
        graph=cast(GraphIoStub, supervisor),
        scope=GraphScope(),
        config=config,
    )


async def test_graph_handle_successive_updates(supervisor: SupervisorStub):
    """Update a node multiple times in the same transaction in succession, should converge."""

    config = SimulationConfig(seed=42, num_clients=1, num_rounds=1, num_edits_per_round=10)
    user, clients = await make_clients(supervisor, config)
    consumers = [EditConsumer(client) for client in clients]
    producers = [
        EditProducer(
            client,
            seed_nodes=[user],
            edit_types=[EditType.UPDATE],
            node_types=[],
            update_properties={NodeType.USER: [User.name, User.text]},
        )
        for client in clients
    ]
    await run_and_check_simulation(
        consumers=consumers,
        producers=producers,
        graph=cast(GraphIoStub, supervisor),
        scope=GraphScope(),
        config=config,
    )


async def test_graph_update_node_with_invalid_property(
    some_user: UserHandle, supervisor: SupervisorStub
):
    """Update a User property to an invalid value, should be rejected."""

    user = some_user.user
    user.name = "thisiswaytoolong" * 64
    node_data = wiring.pack_object(user)
    edit = EditData(
        id=new_edit_id(),
        type=wiring.pack_enum(EditType, EditType.UPDATE),
        node_ptr=NodeReference.from_node_data(node_data),
        properties=[cast(Property, User.name).id],
        new_node_packed=pack_node_delta(node_data, only=(User.name,)),
        old_node_packed=pack_node_delta(node_data, only=(User.name,)),
        edited_at=utcnow(),
        origin=some_user.origin,
        subject_ptr=some_user.user._to_ref_data(),
    )
    with raises_grpc_error(grpclib.Status.INVALID_ARGUMENT):
        _ = await supervisor.commit_transaction(
            CommitTransactionRequest(scope=GraphScope(), id=str(uuid4()), edits=[edit]),
            metadata=some_user.headers,
        )
