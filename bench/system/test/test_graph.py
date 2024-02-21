import asyncio
import random
import secrets
from typing import NamedTuple, cast
from uuid import uuid4

import structlog

from bench.language import Node, Property, User
from bench.language.const import EditType, NodeType, PrimitiveType, UserStatus
from bench.language.node import new_struct_id
from bench.proto import wire, wiring
from bench.proto.wire import (
    CommitTransactionRequest,
    EditData,
    GraphIoStub,
    GraphScope,
    SupervisorStub,
    WatchEditsRequest,
    WatchEditsResponse,
)
from bench.system.test.conftest import UserHandle, make_existing_user_handle, make_new_user_handle

# TODO :Robustness! :Test: test GraphIO much more thoroughly (see FoundationDB)
#  eventually we'll offer some of our testing tools to users as well

NUM_CLIENTS = 3
NUM_ROUNDS = 3
NUM_EDITS_PER_ROUND = 1

logger = structlog.get_logger(__name__)


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
        update_properties: dict[NodeType, list[Property]],
    ):
        self.client = client
        self.edits: list[EditData] = []
        self.seed_nodes = seed_nodes
        self.edit_types = edit_types
        self.node_types = node_types
        self.update_properties = update_properties

    def _make_edit(self, round: int, rng: random.Random) -> EditData:
        # TODO :Incomplete :Test make EditProducer more general & diverse
        node = rng.choice(self.seed_nodes)
        edit_type = rng.choice(self.edit_types)

        if edit_type == EditType.UPDATE:
            prop = rng.choice(self.update_properties[node.metatype])
            if prop.primitive_type == PrimitiveType.STRING:
                new_value = f"{prop.name} {self.client.client.name}:{round}"
                node_data = wiring.pack_node(node)
                setattr(node_data, prop.name, new_value)
                edit = EditData(
                    id=new_struct_id(),
                    type=wiring.pack_enum(EditType, edit_type),
                    node_type=wiring.pack_enum(NodeType, node.metatype),
                    node=wiring.wrap_some_node(node_data),
                    properties=[prop.id],
                )
                return edit

        raise ValueError(f"unsupported edit type: {edit_type}")

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
    seed: int,
):
    await run_simulation(
        consumers=consumers, graph=graph, producers=producers, scope=scope, seed=seed
    )
    check_simulation(consumers=consumers, producers=producers)


async def run_simulation(
    *,
    consumers: list[EditConsumer],
    graph: GraphIoStub,
    producers: list[EditProducer],
    scope: GraphScope,
    seed: int,
):
    try:
        # start consumers
        for consumer in consumers:
            await consumer.start(graph, scope=scope, since_epoch=None)

        # run producers per round
        rng = random.Random(seed)
        for round in range(NUM_ROUNDS):
            shuffled_producers = rng.sample(producers, k=len(producers))
            for producer in shuffled_producers:
                await producer.produce(
                    graph, scope=scope, rng=rng, round=round, num_edits=NUM_EDITS_PER_ROUND
                )
    finally:
        # stop consumers
        for consumer in consumers:
            consumer.stop()


def check_simulation(*, consumers: list[EditConsumer], producers: list[EditProducer]):
    # all edits have been produced
    for producer in producers:
        assert len(producer.edits) == NUM_ROUNDS * NUM_EDITS_PER_ROUND

    # all edits have been consumed by all consumers in the same order
    assert len(consumers) > 0, "no consumers"
    edits = consumers[0].edits
    for consumer in consumers[1:]:
        assert consumer.edits == edits


async def test_supervisor_single_node_conflict(supervisor: SupervisorStub):
    """Simultaneously update a User's name & text."""

    random_slug = secrets.token_hex(8)
    user = User(
        name="",
        slug=random_slug,
        email=random_slug + "@symbolx.com",
        status=UserStatus.REGISTERED,
    )
    password = secrets.token_hex(8)
    _ = await make_new_user_handle(supervisor, user, password=password, client_name="root")
    clients: list[UserHandle] = [
        await make_existing_user_handle(
            supervisor, user, password=password, client_name=f"client_{i}"
        )
        for i in range(NUM_CLIENTS)
    ]
    consumers = [EditConsumer(client) for client in clients]
    producers = [
        EditProducer(
            client,
            seed_nodes=[user],
            edit_types=[EditType.UPDATE],
            node_types=[NodeType.USER],
            update_properties={NodeType.USER: [User.name]},
        )
        for client in clients
    ]
    await run_and_check_simulation(
        consumers=consumers,
        producers=producers,
        graph=cast(GraphIoStub, supervisor),
        scope=GraphScope(),
        seed=42,
    )
