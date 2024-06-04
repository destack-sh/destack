import asyncio
import random
import secrets
from typing import Any, NamedTuple, cast
from uuid import uuid4

import grpclib
import structlog

from bench.conftest import raises_grpc_error
from bench.language import Node, Property, Text, User
from bench.language.const import EditType, NodeType, PrimitiveType, StructType, UserStatus
from bench.language.expression import NodeReference
from bench.language.transaction import new_edit_id, pack_node_delta
from bench.proto import wire, wiring
from bench.proto.wire import (
    AnyNodeData,
    CommitTransactionRequest,
    EditData,
    GraphIoStub,
    GraphScope,
    SupervisorStub,
    WatchEditsRequest,
    WatchEditsResponse,
)
from bench.system.test.conftest import UserHandle, make_existing_user_handle, make_new_user_handle
from bench.utils.dt import utcnow

# TODO :Robustness! :Test!: test GraphIO much more thoroughly (see hypothesis, FoundationDB, ...)

logger = structlog.get_logger(__name__)


class SimulationConfig(NamedTuple):
    seed: int
    num_clients: int
    num_rounds: int
    num_edits_per_round: int


class Epoch(NamedTuple):
    epoch: int
    edits: list[EditData]


class EditConsumer:
    def __init__(self, client: UserHandle):
        self.client = client
        self.edits: list[EditData] = []
        self.epochs: list[Epoch] = []
        self.task: asyncio.Task | None = None

    def receive(self, rep: WatchEditsResponse):
        epoch = Epoch(rep.epoch, rep.edits)
        self.epochs.append(epoch)
        self.edits.extend(rep.edits)

    async def start(self, graph: GraphIoStub, *, scope: GraphScope, since_epoch: int | None):
        watch_req = WatchEditsRequest(
            since_epoch=since_epoch, scope=scope, node_types=[wire.NodeType.USER]
        )

        async def _do_consume():
            async for rep in graph.watch_edits(watch_req, metadata=self.client.headers):
                self.receive(rep)

        self.task = asyncio.create_task(_do_consume())

    def stop(self):
        if self.task is not None:
            self.task.cancel()


class EditProducer:
    def __init__(
        self,
        client: UserHandle,
        seed_nodes: list[Node],
        edit_types: list[EditType],
        node_types: list[NodeType],
        update_properties: dict[NodeType, list[Property | Any]],
    ):
        self.client = client
        self.edits: list[EditData] = []
        self.seed_nodes = seed_nodes
        self.edit_types = edit_types
        self.node_types = node_types
        self.update_properties = update_properties

    def _make_edit(self, round: int, rng: random.Random) -> EditData:
        # TODO :Incomplete :Test!: make EditProducer more general & diverse
        node = rng.choice(self.seed_nodes)
        edit_type = rng.choice(self.edit_types)

        if edit_type == EditType.UPDATE:
            prop = rng.choice(self.update_properties[node.metatype])
            if prop.primitive_type == PrimitiveType.STRING:
                new_value = f"{prop.name} {self.client.client.name}:{round}"
            elif prop.reference_struct == StructType.TEXT:
                new_value = Text.plain(f"{prop.name} {self.client.client.name}:{round}")._to_data()
            else:
                raise ValueError(f"unsupported primitive type: {prop.primitive_type}")
            node_data = cast(AnyNodeData, wiring.pack_node(node))
            setattr(node_data, prop.name, new_value)
            edit = EditData(
                id=new_edit_id(),
                type=wiring.pack_enum(EditType, edit_type),
                node_ptr=NodeReference.from_node_data(node_data),
                properties=[prop.id],
                new_node_packed=pack_node_delta(node_data, only=(prop,)),
                old_node_packed=pack_node_delta(node_data, only=(prop,)),
                origin=self.client.origin,
                subject_ptr=self.client.user.to_ref()._to_data(),
                edited_at=utcnow(),
            )
            return edit

        raise ValueError(f"unsupported edit type: {edit_type.name}")

    async def produce(
        self,
        graph: GraphIoStub,
        *,
        scope: GraphScope,
        rng: random.Random,
        round: int,
        num_edits: int,
    ):
        edits = [self._make_edit(round, rng) for _ in range(num_edits)]
        commit_req = CommitTransactionRequest(scope=scope, id=str(uuid4()), edits=edits)
        commit_rep = await graph.commit_transaction(commit_req, metadata=self.client.headers)
        for edit, accepted_revision in zip(edits, commit_rep.revisions):
            edit.revision = accepted_revision
        self.edits.extend(edits)


async def run_and_check_simulation(
    *,
    consumers: list[EditConsumer],
    producers: list[EditProducer],
    graph: GraphIoStub,
    scope: GraphScope,
    config: SimulationConfig,
):
    await run_simulation(
        consumers=consumers, graph=graph, producers=producers, scope=scope, config=config
    )
    check_simulation(consumers=consumers, producers=producers, config=config)


async def run_simulation(
    *,
    consumers: list[EditConsumer],
    graph: GraphIoStub,
    producers: list[EditProducer],
    scope: GraphScope,
    config: SimulationConfig,
):
    try:
        # start consumers
        for consumer in consumers:
            await consumer.start(graph, scope=scope, since_epoch=None)

        # run producers per round
        rng = random.Random(config.seed)
        for round in range(config.num_rounds):
            shuffled_producers = rng.sample(producers, k=len(producers))
            for producer in shuffled_producers:
                await producer.produce(
                    graph, scope=scope, rng=rng, round=round, num_edits=config.num_edits_per_round
                )
    finally:
        # stop consumers
        for consumer in consumers:
            consumer.stop()


def check_simulation(
    *, consumers: list[EditConsumer], producers: list[EditProducer], config: SimulationConfig
):
    # all edits have been produced
    for producer in producers:
        assert len(producer.edits) == config.num_rounds * config.num_edits_per_round

    # all edits have been consumed by all consumers in the same order
    assert len(consumers) > 0, "no consumers"
    edits = consumers[0].edits
    for consumer in consumers[1:]:
        assert consumer.edits == edits


async def make_clients(
    supervisor: SupervisorStub, config: SimulationConfig
) -> tuple[User, list[UserHandle]]:
    user_slug = secrets.token_hex(8)
    user = User(
        name=user_slug,
        slug=user_slug,
        email=user_slug + "@symbolx.com",
        status=UserStatus.REGISTERED,
    )
    password = secrets.token_hex(8)
    _ = await make_new_user_handle(supervisor, user, password=password, client_name="Root")
    clients: list[UserHandle] = [
        await make_existing_user_handle(
            supervisor, user, password=password, client_name=f"client_{i}"
        )
        for i in range(config.num_clients)
    ]
    return user, clients


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
    node_data = wiring.pack_node(user)
    edit = EditData(
        id=new_edit_id(),
        type=wiring.pack_enum(EditType, EditType.UPDATE),
        node_ptr=NodeReference.from_node_data(node_data),
        properties=[cast(Property, User.name).id],
        new_node_packed=pack_node_delta(node_data, only=(User.name,)),
        old_node_packed=pack_node_delta(node_data, only=(User.name,)),
        edited_at=utcnow(),
        origin=some_user.origin,
        subject_ptr=some_user.user.to_ref()._to_data(),
    )
    with raises_grpc_error(grpclib.Status.INVALID_ARGUMENT):
        _ = await supervisor.commit_transaction(
            CommitTransactionRequest(scope=GraphScope(), id=str(uuid4()), edits=[edit]),
            metadata=some_user.headers,
        )
