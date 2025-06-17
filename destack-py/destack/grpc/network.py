import abc
from collections.abc import AsyncIterator
from typing import TYPE_CHECKING, Optional, cast, override
from urllib.parse import urlparse

import cachetools
import grpclib
import grpclib.client
from grpclib.client import Channel

from destack import proto
from destack.grpc.wiring import pack_rpc_headers
from destack.proto import RpcMetadata
from destack.utils.env import get_from_env
from destack.utils.telemetry import collect_propagation_context
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import ClientType


IS_IN_DOCKER = get_from_env(
    "IS_IN_DOCKER", typ=bool, default=False, description="Whether we're running in Docker"
)
IS_IN_MINIKUBE = get_from_env(
    "IS_IN_MINIKUBE", typ=bool, default=False, description="Whether we're running in Minikube"
)


def localize_url(domain: str) -> str:
    """Converts a domain name to something we can reach inside the current environment."""
    if IS_IN_DOCKER:
        return dockerify_url(domain)
    elif IS_IN_MINIKUBE:
        return minikubeify_url(domain)
    else:
        return domain


def dockerify_url(domain: str) -> str:
    """Converts a domain name to something we can reach inside Docker."""
    domain = domain.replace("localhost", "host.docker.internal")
    domain = domain.replace("127.0.0.1", "host.docker.internal")
    return domain


def minikubeify_url(domain: str) -> str:
    """Converts a domain name to something we can reach inside Minikube."""
    domain = domain.replace("localhost", "host.minikube.internal")
    domain = domain.replace("127.0.0.1", "host.minikube.internal")
    return domain


def get_rpc_metadata(
    *,
    client_type: "ClientType",
    client_id: str | UUID,
    client_access_token: str | UUID,
    client_nonce: str | UUID | None = None,
):
    """Gets the gRPRpcMetadatafor a client."""
    rpc_metadata = RpcMetadata(
        client_type=cast(proto.ClientTypeProto, client_type),
        client_id=str(client_id),
        client_nonce=str(client_nonce) if client_nonce is not None else None,
        client_access_token=str(client_access_token),
    )
    return rpc_metadata


def get_rpc_headers(
    *,
    client_type: "ClientType",
    client_id: str | UUID,
    client_access_token: str | UUID,
    client_nonce: str | UUID | None = None,
):
    """Gets the gRPC headers for a client."""
    rpc_metadata = get_rpc_metadata(
        client_type=client_type,
        client_id=client_id,
        client_access_token=client_access_token,
        client_nonce=client_nonce,
    )
    rpc_headers = pack_rpc_headers(rpc_metadata)
    return rpc_headers


async def unary_stream_rpc[ReqT, RepT](
    method: grpclib.client.UnaryStreamMethod[ReqT, RepT],
    request: ReqT,
    *,
    timeout: Optional[float] = None,
) -> AsyncIterator[RepT]:
    async with method.open(timeout=timeout, metadata=collect_propagation_context()) as stream:
        await stream.send_message(request, end=True)
        async for response in stream:
            yield response


class Network(abc.ABC):
    """A Network for connecting gRPC Services and Clients"""

    @abc.abstractmethod
    async def get_channel(self, url: str, *, source_id: str) -> Channel:
        """
        Get a gRPC Channel to the given URL.
        The source_id should match the 'calling' Service's id
         and is used for internal tracking and routing (esp. in Simulation).
        """
        ...


class NullNetwork(Network):
    """A Network that does nothing."""

    @override
    async def get_channel(self, url: str, *, source_id: str) -> Channel:
        raise NotImplementedError(f"{self.__class__.__name__} is disabled: {url=}, {source_id=}")


class RealNetwork(Network):
    """A real Network"""

    def __init__(self):
        self.channels = cachetools.TTLCache(maxsize=128, ttl=300)

    @override
    async def get_channel(self, url: str, *, source_id: str) -> Channel:
        channel = self.channels.get(url)
        if channel is not None:
            return channel
        connection_info = urlparse(url)
        assert isinstance(connection_info.netloc, str), f"invalid URL: {url}"
        assert connection_info.port is not None, f"invalid URL: {url}"
        netloc = connection_info.netloc.split(":", 1)[0]
        channel = Channel(
            host=netloc, port=connection_info.port, ssl=connection_info.scheme == "https"
        )
        self.channels[url] = channel
        return channel
