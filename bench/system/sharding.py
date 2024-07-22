from typing import Literal

from bench.language.const import Region
from bench.utils.utils import get_from_env


class HostMap:
    """
    Maps regions to host URIs.
    Right now this is just a simple region->host map.
    """

    def __init__(self, host_map: dict[Region | Literal["*"], str]):
        self._host_map = host_map

    def get(self, region: Region) -> str | None:
        """Gets the host URI for the given region."""
        host_uri = self._host_map.get(region)
        if host_uri is None:
            host_uri = self._host_map.get("*")
        return host_uri

    def get_or_error(self, region: Region) -> str:
        """Gets the host URI for the given region (error if none)."""
        host_uri = self.get(region)
        if host_uri is None:
            raise LookupError(f"no known host for {region}")
        return host_uri

    __getitem__ = get_or_error

    @staticmethod
    def from_string(host_map_str: str) -> "HostMap":
        """
        Parses a host map string like:
         'eu-zurich=localhost:8080,eu-frankfurt=localhost:8081'
         'eu-frankfurt=aws-eu-frankfurt.host.justbench.com,*=host.justbench.com'
        """
        host_map = {}
        for map_str in host_map_str.split(","):
            key, host_uri_str = map_str.split("=", 1)
            if key != "*":
                key = Region[key.upper()]
            host_uri = host_uri_str.strip()
            host_map[key] = host_uri
        return HostMap(host_map)

    @staticmethod
    def from_env() -> "HostMap":
        """Parses the HOST_MAP from the environment."""
        host_map_str = get_from_env("HOST_MAP", description="Host map for sharding")
        return HostMap.from_string(host_map_str)
