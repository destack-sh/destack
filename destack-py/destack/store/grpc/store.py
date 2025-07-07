from collections.abc import AsyncGenerator, Sequence
from typing import ClassVar, override

from destack.grpc import pack_rpc_headers
from destack.grpc.network import unary_stream_rpc
from destack.language import Event, LiveStore, Query, QueryResult, QueryUpdate, StoreImplementation
from destack.proto import AppendRequest, QueryRequest, RpcMetadata, SpaceClient, SubscribeRequest


class GrpcStore(LiveStore):
    """A Store that fetches data from a remote source via gRPC."""

    implementation: ClassVar[StoreImplementation | None] = StoreImplementation.POSTGRES

    def __init__(self, space_client: SpaceClient, metadata: RpcMetadata):
        self.client = space_client
        self.metadata = metadata
        self.metadata_packed = pack_rpc_headers(metadata)

    @override
    async def query(self, query: Query) -> QueryResult:
        request = QueryRequest(query=query.to_proto())
        response = await self.client.query(message=request, metadata=self.metadata_packed)
        return QueryResult.from_proto(response.result)

    @override
    async def append(self, events: Sequence[Event]) -> Sequence[Event]:
        request = AppendRequest(events=[event.to_proto() for event in events])
        response = await self.client.append(message=request, metadata=self.metadata_packed)
        applied_events = [Event.from_proto(event) for event in response.events]
        return applied_events

    @override
    async def subscribe(self, query: Query) -> AsyncGenerator[QueryUpdate]:
        request = SubscribeRequest(query=query.to_proto())
        async for response in unary_stream_rpc(
            self.client.subscribe,
            request,
            metadata=self.metadata_packed,
        ):
            yield QueryUpdate.from_proto(response.update)
