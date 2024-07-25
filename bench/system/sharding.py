from typing import Literal

from attr import dataclass

from bench.language.const import Region
from bench.utils.utils import get_from_env


@dataclass
class HostInfo:
    host_domain: str
    grpc_port: int
    grpc_web_port: int

    def render(self) -> str:
        return f"{self.host_domain}:{self.grpc_port}/{self.grpc_web_port}"

    @staticmethod
    def parse(host_uri: str) -> "HostInfo":
        """Parses a host URI like 'host.justbench.com:8080/443'."""
        domain, ports_str = host_uri.split(":", maxsplit=1)
        grpc_port, grpc_web_port = ports_str.split("/", maxsplit=1)
        return HostInfo(
            host_domain=domain, grpc_port=int(grpc_port), grpc_web_port=int(grpc_web_port)
        )


class HostMap:
    """
    Maps regions to host URIs.
    Right now this is just a simple region->host map (with grpc & grpc-web ports).
    """

    def __init__(self, host_map: dict[Region | Literal["*"], HostInfo]):
        self._host_map = host_map

    def __str__(self) -> str:
        return host_map_to_string(self)

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {host_map_to_string(self)}>"

    def get(self, region: Region) -> HostInfo | None:
        """Gets the host URI for the given region."""
        host_uri = self._host_map.get(region)
        if host_uri is None:
            host_uri = self._host_map.get("*")
        return host_uri

    def get_or_error(self, region: Region) -> HostInfo:
        """Gets the host URI for the given region (error if none)."""
        host_info = self.get(region)
        if host_info is None:
            raise LookupError(f"no known host for {region}")
        return host_info

    __getitem__ = get_or_error


def host_map_from_env() -> "HostMap":
    """Parses the HOST_MAP from the environment."""
    host_map_str = get_from_env("HOST_MAP", description="Host map for sharding")
    return host_map_from_string(host_map_str)


def host_map_from_string(host_map_str: str) -> "HostMap":
    """
    Parses a host map string like:
        'eu-zurich=localhost:60061/8080,eu-frankfurt=localhost:60061/8081'
        'eu-frankfurt=aws-eu-frankfurt.host.justbench.com:8080/443,*=host.justbench.com:8080/443'
    """
    host_map = {}
    for mapping_str in host_map_str.split(","):
        region_str, host_info_str = mapping_str.split("=", 1)
        region = Region.get_by_slug(region_str) if region_str != "*" else region_str
        host_info = HostInfo.parse(host_info_str.strip())
        host_map[region] = host_info
    return HostMap(host_map=host_map)


def host_map_to_string(host_map: "HostMap") -> str:
    """Renders a host map back into a string."""
    return ",".join(
        f"{k.slug if isinstance(k, Region) else k}={v.render()}"
        for k, v in host_map._host_map.items()
    )
