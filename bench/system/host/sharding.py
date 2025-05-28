import abc
from dataclasses import dataclass
from typing import Literal, Mapping
from uuid import UUID

from bench.language import Region
from bench.utils.utils import get_from_env


@dataclass(slots=True, is_frozen=True)
class HostInfo:
    host_domain: str
    grpc_port: int
    grpc_web_port: int
    ssl: bool

    def render(self) -> str:
        return f"{self.host_domain}:{self.grpc_port}/{self.grpc_web_port}{'s' if self.ssl else ''}"

    @staticmethod
    def parse(host_url: str) -> "HostInfo":
        """Parses a host URL like 'host.justbench.com:8080/443'."""
        if host_url.endswith("s"):
            ssl = True
            host_url = host_url[:-1]
        else:
            ssl = False
        domain, ports_str = host_url.split(":", maxsplit=1)
        grpc_port, grpc_web_port = ports_str.split("/", maxsplit=1)
        return HostInfo(
            host_domain=domain, grpc_port=int(grpc_port), grpc_web_port=int(grpc_web_port), ssl=ssl
        )


class HostMap(abc.ABC):
    """
    Maps Regions and Bench IDs to Host URLs.
    """

    @abc.abstractmethod
    def get(self, bench_id: UUID, region: Region) -> HostInfo:
        """Gets the host info for the given bench ID and region (error if none)."""
        ...


class StaticHostMap(HostMap):
    """A static host map backed by a dictionary."""

    def __init__(
        self,
        hosts: Mapping[UUID | Region | Literal["*"], HostInfo],
    ):
        self._hosts = hosts

    def __str__(self) -> str:
        return host_map_to_string(self)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {host_map_to_string(self) or '<empty>'}>"

    def get(self, bench_id: UUID, region: Region) -> HostInfo:
        """Gets the host info for the given bench ID and region (error if none)."""
        host_info = self._hosts.get(bench_id)
        if host_info is None:
            host_info = self._hosts.get(region)
        if host_info is None:
            host_info = self._hosts.get("*")
        if host_info is None:
            raise LookupError(f"no host info for {region.bench_name} in {self!r}")
        return host_info


def get_host_map_from_string(host_map_str: str) -> StaticHostMap:
    """
    Parses a host map string like:
        'eu-zurich=localhost:60061/8080,eu-frankfurt=localhost:60061/8081'
        'eu-frankfurt=aws-eu-frankfurt.host.justbench.com:8080/443,*=host.justbench.com:8080/443'
        '01234567-8910-0000-0000-000000000000=host1.justbench.com:8080/443,eu-frankfurt=host2.justbench.com:8080/443'
    """
    hosts: dict[UUID | Region | Literal["*"], HostInfo] = {}

    for mapping_str in host_map_str.split(","):
        key_str, host_info_str = mapping_str.split("=", 1)
        host_info = HostInfo.parse(host_info_str.strip())
        try:
            bench_id = UUID(key_str)
            hosts[bench_id] = host_info
        except ValueError:
            region = Region.get_by_slug(key_str) if key_str != "*" else key_str
            hosts[region] = host_info

    return StaticHostMap(hosts=hosts)


def host_map_to_string(host_map: StaticHostMap) -> str:
    """Renders a host map back into a string."""
    return ",".join(
        f"{k.slug if isinstance(k, Region) else k}={v.render()}" for k, v in host_map._hosts.items()
    )


def get_host_map_from_env() -> StaticHostMap:
    """Parses the HOST_MAP from the environment."""
    host_map_str = get_from_env("HOST_MAP", description="Host map for sharding")
    return get_host_map_from_string(host_map_str)


HOST_MAP = get_host_map_from_env()
